// apps/fetch/back-end/src/model.rs

use serde::Serialize;

/// A page materialized from one finite sitemap binding.
#[derive(Debug, Clone)]
pub struct IndexPage {
    pub url: String,
    pub title: String,
    pub body: String,
    pub updated_at: String,
    pub images: Vec<String>,
    pub graph: Option<String>,
}

/// A search hit returned by the public API and rendered by the CSR client.
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub updated_at: String,
    pub images: Vec<String>,
    pub score: f32,
}

/// A stable response envelope for paginated search requests.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub results: Vec<SearchHit>,
}

/// Current index lifecycle state exposed to the mock console.
#[derive(Debug, Clone, Serialize)]
pub struct IndexStatus {
    pub documents: usize,
    pub mappings: usize,
    pub last_rebuilt_at: String,
}

/// Result of one complete mock sitemap ingestion.
#[derive(Debug, Clone, Serialize)]
pub struct RebuildReport {
    pub mappings: usize,
    pub rows: usize,
    pub documents: usize,
}
