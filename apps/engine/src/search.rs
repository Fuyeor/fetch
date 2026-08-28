// apps/engine/src/search.rs

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use jieba_rs::Jieba;
use tantivy::collector::{Count, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{TantivyDocument, Value as TantivyValue};
use tantivy::tokenizer::{TextAnalyzer, Token, TokenStream, Tokenizer};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Term};

use crate::core::document::SearchDocument;
use crate::index::{
    IndexManifest, ManifestDocument, SearchFields, build_schema, fields_from_schema, first_text,
    load_manifest, persist_manifest,
};
use crate::model::{SearchHit, SearchResponse};

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const MAX_QUERY_CHARS: usize = 256;
const MAX_PAGE_SIZE: usize = 50;
const INDEX_MEMORY_BYTES: usize = 32 * 1024 * 1024;

/// A shared Jieba tokenizer that preserves byte offsets required by Tantivy.
#[derive(Clone)]
pub struct JiebaTokenizer {
    jieba: Arc<Jieba>,
}

impl Default for JiebaTokenizer {
    fn default() -> Self {
        Self {
            jieba: Arc::new(Jieba::new()),
        }
    }
}

impl Tokenizer for JiebaTokenizer {
    type TokenStream<'a> = JiebaTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let tokens = self
            .jieba
            .cut_for_search(text, true)
            .into_iter()
            .map(|token| Token {
                offset_from: token.byte_start,
                offset_to: token.byte_end,
                position: token.byte_start,
                text: token.word.to_string(),
                position_length: 1,
            })
            .collect();
        JiebaTokenStream {
            tokens,
            index: 0,
            current: Token::default(),
        }
    }
}

/// A reusable Tantivy token stream backed by owned Jieba token text.
pub struct JiebaTokenStream {
    tokens: Vec<Token>,
    index: usize,
    current: Token,
}

impl TokenStream for JiebaTokenStream {
    fn advance(&mut self) -> bool {
        let Some(token) = self.tokens.get(self.index).cloned() else {
            return false;
        };
        self.current = token;
        self.index += 1;
        true
    }

    fn token(&self) -> &Token {
        &self.current
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.current
    }
}

/// Changes applied to the Tantivy index in one committed batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexSyncReport {
    pub generation: u64,
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
}

/// A single-writer Tantivy index with manual reader reloads after commits.
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: SearchFields,
    manifest_path: PathBuf,
    generation: AtomicU64,
}

impl SearchEngine {
    /// Open or create the on-disk index and register the Chinese analyzer.
    pub fn open(index_path: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(index_path)?;
        let manifest_path = index_path.join("manifest.json");
        let (new_schema, new_fields) = build_schema();
        let (initial_generation, index, fields) = if index_path.join("meta.json").exists() {
            let index = Index::open(MmapDirectory::open(index_path)?)?;
            match fields_from_schema(&index.schema()) {
                Ok(fields) => (load_manifest(&manifest_path)?.generation, index, fields),
                Err(error) => {
                    drop(index);
                    quarantine_incompatible_index(index_path, error.as_ref())?;
                    let index = Index::create_in_dir(index_path, new_schema.clone())?;
                    (0, index, new_fields)
                }
            }
        } else {
            let initial_generation = load_manifest(&manifest_path)?.generation;
            (
                initial_generation,
                Index::create_in_dir(index_path, new_schema)?,
                new_fields,
            )
        };
        index.tokenizers().register(
            "jieba_search",
            TextAnalyzer::builder(JiebaTokenizer::default()).build(),
        );
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let writer = index.writer(INDEX_MEMORY_BYTES)?;
        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
            manifest_path,
            generation: AtomicU64::new(initial_generation),
        })
    }

    /// Apply an authoritative finite catalog using stable IDs and content hashes.
    pub fn sync_documents(&self, pages: &[SearchDocument]) -> AppResult<IndexSyncReport> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| "index writer lock poisoned")?;
        let mut manifest = load_manifest(&self.manifest_path)?;
        let live_document_count = self.document_count()?;
        let legacy_index = !self.manifest_path.exists() && live_document_count > 0;
        let mut incoming = BTreeMap::new();
        for page in pages {
            if incoming.insert(page.document_id.clone(), page).is_some() {
                return Err(format!("duplicate document_id: {}", page.document_id).into());
            }
        }

        let mut report = IndexSyncReport {
            generation: manifest.generation,
            ..IndexSyncReport::default()
        };
        if legacy_index {
            writer.delete_all_documents()?;
            report.deleted = live_document_count;
            manifest = IndexManifest::default();
        }

        for (document_id, page) in &incoming {
            match manifest.documents.get(document_id) {
                None => report.added += 1,
                Some(previous) if previous.content_hash == page.content_hash => {
                    report.unchanged += 1
                }
                Some(_) => report.updated += 1,
            }
        }
        report.deleted += manifest
            .documents
            .keys()
            .filter(|document_id| !incoming.contains_key(*document_id))
            .count();

        let changed = report.added + report.updated + report.deleted;
        if changed == 0 {
            return Ok(report);
        }
        let generation = manifest.generation.saturating_add(1);
        report.generation = generation;

        for document_id in manifest.documents.keys().filter(|document_id| {
            !incoming.contains_key(*document_id)
                || incoming.get(*document_id).is_some_and(|page| {
                    manifest
                        .documents
                        .get(*document_id)
                        .is_some_and(|previous| previous.content_hash != page.content_hash)
                })
        }) {
            writer.delete_term(Term::from_field_text(self.fields.document_id, document_id));
        }
        if legacy_index {
            // The legacy delete_all above already removed unknown IDs; this loop is intentionally empty.
        }
        for (document_id, page) in &incoming {
            let needs_write = manifest
                .documents
                .get(document_id)
                .is_none_or(|previous| previous.content_hash != page.content_hash);
            if needs_write {
                let document = (*page).clone().with_generation(generation);
                writer.add_document(self.to_document(&document))?;
            }
        }
        writer.commit()?;
        self.reader.reload()?;

        manifest.generation = generation;
        manifest.documents = incoming
            .into_iter()
            .map(|(document_id, page)| {
                (
                    document_id,
                    ManifestDocument {
                        content_hash: page.content_hash.clone(),
                        generation,
                    },
                )
            })
            .collect();
        persist_manifest(&self.manifest_path, &manifest)?;
        self.generation.store(generation, Ordering::Release);
        Ok(report)
    }

    /// Search indexed title/content with bounded pagination and stored metadata.
    pub fn search(&self, query: &str, offset: usize, limit: usize) -> AppResult<SearchResponse> {
        let query = query.trim();
        if query.chars().count() > MAX_QUERY_CHARS {
            return Err("query is too long".into());
        }
        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        let searcher = self.reader.searcher();
        let parser =
            QueryParser::for_index(&self.index, vec![self.fields.title, self.fields.content]);
        let parsed_query = parser.parse_query(query)?;
        let total = searcher.search(&parsed_query, &Count)?;
        let top_docs = searcher.search(
            &parsed_query,
            &TopDocs::with_limit(limit)
                .and_offset(offset)
                .order_by_score(),
        )?;
        let mut results = Vec::with_capacity(top_docs.len());
        for (score, address) in top_docs {
            let document = searcher.doc::<TantivyDocument>(address)?;
            results.push(SearchHit {
                document_id: first_text(&document, self.fields.document_id),
                mapping_id: first_text(&document, self.fields.mapping_id),
                url: first_text(&document, self.fields.url),
                title: first_text(&document, self.fields.title),
                snippet: first_text(&document, self.fields.content)
                    .chars()
                    .take(180)
                    .collect(),
                updated_at: first_text(&document, self.fields.updated_at),
                images: document
                    .get_all(self.fields.image_url)
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect(),
                content_hash: first_text(&document, self.fields.content_hash),
                generation: first_text(&document, self.fields.generation)
                    .parse()
                    .unwrap_or_default(),
                score,
            });
        }
        Ok(SearchResponse {
            query: query.to_string(),
            offset,
            limit,
            total,
            generation: self.generation()?,
            results,
        })
    }

    /// Return the current number of live documents after a reader refresh.
    pub fn document_count(&self) -> AppResult<usize> {
        Ok(self
            .reader
            .searcher()
            .search(&tantivy::query::AllQuery, &Count)?)
    }

    /// Return the last committed logical catalog generation.
    pub fn generation(&self) -> AppResult<u64> {
        Ok(self.generation.load(Ordering::Acquire))
    }

    fn to_document(&self, page: &SearchDocument) -> TantivyDocument {
        let mut document = TantivyDocument::default();
        document.add_text(self.fields.document_id, &page.document_id);
        document.add_text(self.fields.mapping_id, &page.mapping_id);
        document.add_text(self.fields.source, &page.source);
        document.add_text(self.fields.url, &page.url);
        document.add_text(self.fields.title, &page.title);
        document.add_text(self.fields.content, &page.body);
        document.add_text(self.fields.updated_at, &page.updated_at);
        document.add_text(self.fields.content_hash, &page.content_hash);
        document.add_text(self.fields.generation, page.generation.to_string());
        if let Some(graph) = &page.graph {
            document.add_text(self.fields.graph, graph);
        }
        for image in &page.images {
            document.add_text(self.fields.image_url, image);
        }
        document
    }
}

/// Move an incompatible Tantivy directory aside before creating the current schema.
fn quarantine_incompatible_index(
    index_path: &Path,
    error: &dyn std::error::Error,
) -> AppResult<()> {
    let parent = index_path
        .parent()
        .ok_or_else(|| "index path has no parent directory".to_string())?;
    let name = index_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "index path has no valid directory name".to_string())?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let mut backup = parent.join(format!("{name}.incompatible-{timestamp}"));
    let mut suffix = 0_u32;
    while backup.exists() {
        suffix = suffix.saturating_add(1);
        backup = parent.join(format!("{name}.incompatible-{timestamp}-{suffix}"));
    }
    std::fs::rename(index_path, &backup).map_err(|rename_error| {
        format!(
            "incompatible Tantivy schema ({error}); failed to quarantine old index at {}: {rename_error}",
            backup.display()
        )
    })?;
    std::fs::create_dir_all(index_path)?;
    eprintln!(
        "Quarantined incompatible Tantivy index ({error}) at {}",
        backup.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{JiebaTokenizer, SearchEngine};
    use crate::core::document::SearchDocument;
    use tantivy::Index;
    use tantivy::schema::{STORED, Schema};
    use tantivy::tokenizer::{TokenStream, Tokenizer};

    #[test]
    fn jieba_emits_chinese_terms_with_byte_offsets() {
        let mut tokenizer = JiebaTokenizer::default();
        let mut stream = tokenizer.token_stream("关于我的动态");
        let mut terms = Vec::new();
        while stream.advance() {
            terms.push(stream.token().text.clone());
        }
        assert!(terms.iter().any(|term| term == "动态"));
    }

    #[test]
    fn unchanged_sync_keeps_generation_and_changed_sync_increments_it() {
        let directory = tempdir().unwrap();
        let engine = SearchEngine::open(directory.path()).unwrap();
        let page = SearchDocument::new(
            "mapping",
            "content.fon",
            "/stable",
            "Stable",
            "Initial body",
            "2026-08-30",
            Vec::new(),
            None,
        );
        let first = engine.sync_documents(std::slice::from_ref(&page)).unwrap();
        assert_eq!(first.generation, 1);
        assert_eq!(first.added, 1);
        let unchanged = engine.sync_documents(std::slice::from_ref(&page)).unwrap();
        assert_eq!(unchanged.generation, 1);
        assert_eq!(unchanged.unchanged, 1);
        let changed = SearchDocument::new(
            "mapping",
            "content.fon",
            "/stable",
            "Stable",
            "Changed body",
            "2026-08-30",
            Vec::new(),
            None,
        );
        let second = engine
            .sync_documents(std::slice::from_ref(&changed))
            .unwrap();
        assert_eq!(second.generation, 2);
        assert_eq!(second.updated, 1);
        assert_eq!(engine.document_count().unwrap(), 1);
    }

    #[test]
    fn incompatible_schema_is_quarantined_and_recreated() {
        let parent = tempdir().unwrap();
        let index_path = parent.path().join("index");
        std::fs::create_dir_all(&index_path).unwrap();
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", STORED);
        let old_index = Index::create_in_dir(&index_path, schema_builder.build()).unwrap();
        drop(old_index);

        let engine = SearchEngine::open(&index_path).unwrap();
        assert_eq!(engine.document_count().unwrap(), 0);
        assert!(
            std::fs::read_dir(parent.path())
                .unwrap()
                .flatten()
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("index.incompatible-"))
        );
    }

    #[test]
    fn missing_authoritative_document_is_deleted() {
        let directory = tempdir().unwrap();
        let engine = SearchEngine::open(directory.path()).unwrap();
        let page = SearchDocument::new(
            "mapping",
            "content.fon",
            "/to-delete",
            "Delete me",
            "Body",
            "2026-08-30",
            Vec::new(),
            None,
        );
        engine.sync_documents(std::slice::from_ref(&page)).unwrap();
        let report = engine.sync_documents(&[]).unwrap();
        assert_eq!(report.deleted, 1);
        assert_eq!(engine.document_count().unwrap(), 0);
    }
}
