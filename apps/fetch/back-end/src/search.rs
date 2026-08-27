// apps/fetch/back-end/src/search.rs

use std::path::Path;
use std::sync::{Arc, Mutex};

use jieba_rs::Jieba;
use tantivy::collector::{Count, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{
    Field, IndexRecordOption, STORED, STRING, Schema, TantivyDocument, TextFieldIndexing,
    TextOptions, Value as TantivyValue,
};
use tantivy::tokenizer::{TextAnalyzer, Token, TokenStream, Tokenizer};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, doc};

use crate::model::{IndexPage, SearchHit, SearchResponse};

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

/// Field handles kept together so query and document code cannot drift apart.
#[derive(Clone, Copy)]
struct SearchFields {
    url: Field,
    title: Field,
    content: Field,
    updated_at: Field,
    image_url: Field,
    graph: Field,
}

/// A single-writer Tantivy index with manual reader reloads after commits.
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: SearchFields,
}

impl SearchEngine {
    /// Open or create the on-disk index and register the Chinese analyzer.
    pub fn open(index_path: &Path) -> AppResult<Self> {
        std::fs::create_dir_all(index_path)?;
        let (schema, fields) = build_schema();
        let index = if index_path.join("meta.json").exists() {
            Index::open(MmapDirectory::open(index_path)?)?
        } else {
            Index::create_in_dir(index_path, schema.clone())?
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
        })
    }

    /// Replace the current index atomically at the Tantivy commit boundary.
    pub fn replace_all(&self, pages: &[IndexPage]) -> AppResult<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| "index writer lock poisoned")?;
        writer.delete_all_documents()?;
        for page in pages {
            writer.add_document(self.to_document(page))?;
        }
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
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
                score,
            });
        }
        Ok(SearchResponse {
            query: query.to_string(),
            offset,
            limit,
            total,
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

    fn to_document(&self, page: &IndexPage) -> TantivyDocument {
        let mut document = doc!(
            self.fields.url => page.url.as_str(),
            self.fields.title => page.title.as_str(),
            self.fields.content => page.body.as_str(),
            self.fields.updated_at => page.updated_at.as_str(),
        );
        if let Some(graph) = &page.graph {
            document.add_text(self.fields.graph, graph);
        }
        for image in &page.images {
            document.add_text(self.fields.image_url, image);
        }
        document
    }
}

/// Build stored fields and jieba-backed text fields for the MVP schema.
fn build_schema() -> (Schema, SearchFields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("jieba_search")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    let url = builder.add_text_field("url", STRING | STORED);
    let title = builder.add_text_field("title", text_options.clone());
    let content = builder.add_text_field("content", text_options);
    let updated_at = builder.add_text_field("updated_at", STRING | STORED);
    let image_url = builder.add_text_field("image_url", STRING | STORED);
    let graph = builder.add_text_field("graph", STRING | STORED);
    let schema = builder.build();
    (
        schema,
        SearchFields {
            url,
            title,
            content,
            updated_at,
            image_url,
            graph,
        },
    )
}

fn first_text(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::JiebaTokenizer;
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
}
