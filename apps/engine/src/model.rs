// apps/engine/src/model.rs

use serde::Serialize;

/// A search hit returned by the public API and rendered by the CSR client.
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub document_id: String,
    pub mapping_id: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub updated_at: String,
    pub images: Vec<String>,
    pub content_hash: String,
    pub generation: u64,
    pub score: f32,
}

/// A stable response envelope for paginated search requests.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub generation: u64,
    pub results: Vec<SearchHit>,
}

/// Current index lifecycle state exposed to the mock console.
#[derive(Debug, Clone, Serialize)]
pub struct IndexStatus {
    pub documents: usize,
    pub mappings: usize,
    pub generation: u64,
    pub last_rebuilt_at: String,
    pub last_ingested_at: String,
}

/// Result of one complete sitemap ingestion.
#[derive(Debug, Clone, Serialize)]
pub struct RebuildReport {
    pub mappings: usize,
    pub rows: usize,
    pub documents: usize,
    pub generation: u64,
    pub added: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub fetched: usize,
    pub not_modified: usize,
    pub retries: usize,
    pub deferred: usize,
}
