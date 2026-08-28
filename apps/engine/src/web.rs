// apps/engine/src/web.rs

use std::sync::{Arc, RwLock};

use axum::Router;
use axum::extract::{Json as JsonRequest, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::crawler::Crawler;
use crate::fon::load_catalog;
use crate::model::{IndexStatus, RebuildReport, SearchResponse};
use crate::search::SearchEngine;

/// Shared application state kept behind an immutable Arc for concurrent requests.
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<SearchEngine>,
    pub mock_root: std::path::PathBuf,
    pub crawler: Option<Arc<Crawler>>,
    metadata: Arc<RwLock<IndexMetadata>>,
}

#[derive(Debug, Default)]
struct IndexMetadata {
    mappings: usize,
    last_rebuilt_at: String,
    last_ingested_at: String,
}

impl AppState {
    /// Build a local-only state used by fixture tests and development rebuilds.
    #[cfg(test)]
    pub fn new(engine: Arc<SearchEngine>, mock_root: std::path::PathBuf) -> Self {
        Self {
            engine,
            mock_root,
            crawler: None,
            metadata: Arc::new(RwLock::new(IndexMetadata::default())),
        }
    }

    /// Build the production-shaped state with a durable submitted-locator crawler.
    pub fn new_with_crawler(
        engine: Arc<SearchEngine>,
        mock_root: std::path::PathBuf,
        crawler: Arc<Crawler>,
    ) -> Self {
        Self {
            engine,
            mock_root,
            crawler: Some(crawler),
            metadata: Arc::new(RwLock::new(IndexMetadata::default())),
        }
    }
}

/// Build the data-plane API; gateway version prefixes are intentionally outside this router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/search", get(search))
        .route("/index/status", get(index_status))
        .route("/index/rebuild", post(rebuild_index))
        .route("/index/ingest", post(ingest_index))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub mapping_url: String,
}

/// Execute a bounded full-text query and return a stable JSON envelope.
async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    let response = state.engine.search(
        &query.q,
        query.offset.unwrap_or(0),
        query.limit.unwrap_or(20),
    )?;
    Ok(Json(response))
}

/// Report the number of currently searchable documents and the published generation.
async fn index_status(State(state): State<AppState>) -> Result<Json<IndexStatus>, ApiError> {
    let metadata = state
        .metadata
        .read()
        .map_err(|_| "index metadata lock poisoned")?;
    Ok(Json(IndexStatus {
        documents: state.engine.document_count()?,
        mappings: metadata.mappings,
        generation: state.engine.generation()?,
        last_rebuilt_at: metadata.last_rebuilt_at.clone(),
        last_ingested_at: metadata.last_ingested_at.clone(),
    }))
}

/// Apply the local finite FON catalog without crossing site boundaries or crawling remote pages.
async fn rebuild_index(State(state): State<AppState>) -> Result<Json<RebuildReport>, ApiError> {
    let catalog = load_catalog(&state.mock_root)?;
    let sync = state.engine.sync_documents(&catalog.pages)?;
    let mut metadata = state
        .metadata
        .write()
        .map_err(|_| "index metadata lock poisoned")?;
    metadata.mappings = catalog.mappings.len();
    metadata.last_rebuilt_at = unix_timestamp().to_string();
    Ok(Json(RebuildReport {
        mappings: catalog.mappings.len(),
        rows: catalog.pages.len(),
        documents: catalog.pages.len(),
        generation: sync.generation,
        added: sync.added,
        updated: sync.updated,
        deleted: sync.deleted,
        unchanged: sync.unchanged,
        fetched: 0,
        not_modified: 0,
        retries: 0,
        deferred: 0,
    }))
}

/// Ingest a webmaster-submitted remote mapping and atomically publish its finite catalog.
async fn ingest_index(
    State(state): State<AppState>,
    JsonRequest(request): JsonRequest<IngestRequest>,
) -> Result<Json<RebuildReport>, ApiError> {
    let crawler = state
        .crawler
        .as_ref()
        .ok_or_else(|| "remote ingestion is not enabled".to_string())?;
    let remote = crawler.ingest_mapping(&request.mapping_url).await?;
    let sync = state.engine.sync_documents(&remote.catalog.pages)?;
    let mut metadata = state
        .metadata
        .write()
        .map_err(|_| "index metadata lock poisoned")?;
    metadata.mappings = remote.catalog.mappings.len();
    metadata.last_ingested_at = unix_timestamp().to_string();
    Ok(Json(RebuildReport {
        mappings: remote.catalog.mappings.len(),
        rows: remote.rows,
        documents: remote.catalog.pages.len(),
        generation: sync.generation,
        added: sync.added,
        updated: sync.updated,
        deleted: sync.deleted,
        unchanged: sync.unchanged,
        fetched: remote.stats.fetched,
        not_modified: remote.stats.not_modified,
        retries: remote.stats.retries,
        deferred: remote.stats.deferred,
    }))
}

/// Return a stable wall-clock marker without adding a date-time dependency.
fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

/// Convert internal failures into one JSON error shape without leaking implementation details.
#[derive(Debug)]
struct ApiError(Box<dyn std::error::Error + Send + Sync>);

impl From<Box<dyn std::error::Error + Send + Sync>> for ApiError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(error)
    }
}

impl From<String> for ApiError {
    fn from(error: String) -> Self {
        Self(std::io::Error::other(error).into())
    }
}

impl From<&'static str> for ApiError {
    fn from(error: &'static str) -> Self {
        Self(std::io::Error::other(error).into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.0.to_string() });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::{AppState, router};
    use crate::search::SearchEngine;

    #[tokio::test]
    async fn rebuild_then_search_returns_indexed_mock_content() {
        let index_dir = tempfile::tempdir().expect("temporary index directory must be created");
        let engine =
            Arc::new(SearchEngine::open(index_dir.path()).expect("temporary index must open"));
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let app = router(AppState::new(engine, root));

        let rebuild = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/index/rebuild")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("rebuild response must be returned");
        assert_eq!(rebuild.status(), StatusCode::OK);
        let rebuild_body = to_bytes(rebuild.into_body(), usize::MAX)
            .await
            .expect("rebuild body must be readable");
        let rebuild_json: serde_json::Value =
            serde_json::from_slice(&rebuild_body).expect("rebuild body must be JSON");
        assert_eq!(rebuild_json["documents"], 5);
        assert_eq!(rebuild_json["added"], 5);
        assert_eq!(rebuild_json["generation"], 1);

        let unchanged = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/index/rebuild")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("second rebuild response must be returned");
        let unchanged_body = to_bytes(unchanged.into_body(), usize::MAX)
            .await
            .expect("second rebuild body must be readable");
        let unchanged_json: serde_json::Value =
            serde_json::from_slice(&unchanged_body).expect("second rebuild body must be JSON");
        assert_eq!(unchanged_json["unchanged"], 5);
        assert_eq!(unchanged_json["generation"], 1);

        let search = app
            .oneshot(
                Request::builder()
                    .uri("/search?q=搜索引擎&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("search response must be returned");
        assert_eq!(search.status(), StatusCode::OK);
        let search_body = to_bytes(search.into_body(), usize::MAX)
            .await
            .expect("search body must be readable");
        let search_json: serde_json::Value =
            serde_json::from_slice(&search_body).expect("search body must be JSON");
        assert_eq!(search_json["total"], 2);
        assert_eq!(search_json["generation"], 1);
        assert!(
            search_json["results"][0]["url"]
                .as_str()
                .is_some_and(|url| url.contains("thought"))
        );
    }

    #[tokio::test]
    async fn ingest_endpoint_publishes_remote_submitted_catalog() {
        let remote_app = axum::Router::new()
            .route(
                "/search-patterns.fon",
                axum::routing::get(|| async {
                    "[{ pattern = `/profile/{username}` params = struct { username: string } datas = `/sitemap.fon` }]"
                }),
            )
            .route(
                "/sitemap.fon",
                axum::routing::get(|| async {
                    "[{ params = { username = `Alice` } content = `/content.fon` }]"
                }),
            )
            .route(
                "/content.fon",
                axum::routing::get(|| async { "{ title = `Alice` content = `# Hello` }" }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, remote_app).await.unwrap() });

        let index_dir = tempfile::tempdir().expect("temporary index directory must be created");
        let state_dir = tempfile::tempdir().expect("temporary crawler state must be created");
        let engine =
            Arc::new(SearchEngine::open(index_dir.path()).expect("temporary index must open"));
        let crawler = Arc::new(
            crate::crawler::Crawler::open(
                &state_dir.path().join("state"),
                crate::crawler::FetchPolicy::for_tests(),
                crate::crawler::CrawlerConfig {
                    max_attempts: 1,
                    ..crate::crawler::CrawlerConfig::default()
                },
            )
            .expect("crawler must open"),
        );
        let app = router(AppState::new_with_crawler(
            engine,
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            crawler,
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/index/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        "{{\"mapping_url\":\"http://{address}/search-patterns.fon\"}}"
                    )))
                    .unwrap(),
            )
            .await
            .expect("ingest response must be returned");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["documents"], 1);
        assert_eq!(json["fetched"], 3);
        assert_eq!(json["generation"], 1);
    }
}
