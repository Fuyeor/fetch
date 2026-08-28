// apps/engine/src/core/document.rs

use sha2::{Digest, Sha256};

/// A canonical searchable document independent of the index implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    pub document_id: String,
    /// Internal mapping key derived exclusively from the canonical FRL pattern.
    pub mapping_id: String,
    pub source: String,
    pub url: String,
    pub title: String,
    pub body: String,
    pub updated_at: String,
    pub images: Vec<String>,
    pub graph: Option<String>,
    pub content_hash: String,
    pub generation: u64,
}

impl SearchDocument {
    /// Build a document and derive identity from its pattern-derived mapping key and URL.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mapping_id: impl Into<String>,
        source: impl Into<String>,
        url: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        updated_at: impl Into<String>,
        images: Vec<String>,
        graph: Option<String>,
    ) -> Self {
        let mapping_id = mapping_id.into();
        let source = source.into();
        let url = url.into();
        let title = title.into();
        let body = body.into();
        let updated_at = updated_at.into();
        let document_id = stable_document_id(&mapping_id, &url);
        let content_hash = content_hash(&title, &body, &updated_at, &images, graph.as_deref());
        Self {
            document_id,
            mapping_id,
            source,
            url,
            title,
            body,
            updated_at,
            images,
            graph,
            content_hash,
            generation: 0,
        }
    }

    /// Attach the immutable index generation that published this document.
    pub fn with_generation(mut self, generation: u64) -> Self {
        self.generation = generation;
        self
    }
}

/// Derive a stable internal mapping key from the canonical FRL pattern only.
pub fn stable_mapping_id(canonical_pattern: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"fetch/mapping/v1\0");
    update_length_prefixed(&mut hasher, canonical_pattern.as_bytes());
    format!("map_{}", hex_digest(hasher.finalize()))
}

/// Derive a stable ID from protocol identity, not from mutable page content.
pub fn stable_document_id(mapping_id: &str, canonical_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"fetch/document/v1\0");
    update_length_prefixed(&mut hasher, mapping_id.as_bytes());
    update_length_prefixed(&mut hasher, canonical_url.as_bytes());
    format!("doc_{}", hex_digest(hasher.finalize()))
}

/// Hash all indexed content and metadata using an unambiguous length-prefixed encoding.
pub fn content_hash(
    title: &str,
    body: &str,
    updated_at: &str,
    images: &[String],
    graph: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"fetch/content/v1\0");
    for value in [title, body, updated_at] {
        update_length_prefixed(&mut hasher, value.as_bytes());
    }
    hasher.update((images.len() as u64).to_be_bytes());
    for image in images {
        update_length_prefixed(&mut hasher, image.as_bytes());
    }
    match graph {
        Some(graph) => {
            hasher.update([1]);
            update_length_prefixed(&mut hasher, graph.as_bytes());
        }
        None => hasher.update([0]),
    }
    format!("sha256:{}", hex_digest(hasher.finalize()))
}

fn update_length_prefixed(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn hex_digest<D: AsRef<[u8]>>(digest: D) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SearchDocument, content_hash, stable_document_id, stable_mapping_id};

    #[test]
    fn mapping_id_depends_on_canonical_pattern_only() {
        let first = stable_mapping_id("/@{username}/thoughts");
        let second = stable_mapping_id("/@{username}/thoughts");
        assert_eq!(first, second);
        assert_ne!(first, stable_mapping_id("/@{username}/articles"));
    }

    #[test]
    fn document_id_depends_on_mapping_and_url_only() {
        let mapping_id = stable_mapping_id("/@{username}/thoughts");
        let first = stable_document_id(&mapping_id, "/@Fuyeor/thoughts");
        let second = stable_document_id(&mapping_id, "/@Fuyeor/thoughts");
        assert_eq!(first, second);
        assert_ne!(first, stable_document_id(&mapping_id, "/@Fuyeor/comments"));
    }

    #[test]
    fn content_hash_changes_when_indexed_content_changes() {
        let base = content_hash("Title", "Body", "2026-08-30", &[], None);
        let changed = content_hash("Title", "Changed", "2026-08-30", &[], None);
        assert_ne!(base, changed);
    }

    #[test]
    fn new_document_starts_before_publication_generation() {
        let document = SearchDocument::new(
            stable_mapping_id("/@{username}"),
            "profile.fon",
            "/@Fuyeor",
            "Fuyeor",
            "Body",
            "2026-08-30",
            Vec::new(),
            None,
        );
        assert_eq!(document.generation, 0);
        assert!(document.content_hash.starts_with("sha256:"));
    }
}
