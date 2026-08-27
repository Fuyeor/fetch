// apps/engine/src/web.rs

use std::sync::{Arc, RwLock};

use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::fon::load_catalog;
use crate::model::{IndexStatus, RebuildReport, SearchResponse};
use crate::search::SearchEngine;

/// Shared application state kept behind an immutable Arc for concurrent requests.
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<SearchEngine>,
    pub mock_root: std::path::PathBuf,
    metadata: Arc<RwLock<IndexMetadata>>,
}

#[derive(Debug, Default)]
struct IndexMetadata {
    mappings: usize,
    last_rebuilt_at: String,
}

impl AppState {
    pub fn new(engine: Arc<SearchEngine>, mock_root: std::path::PathBuf) -> Self {
        Self {
            engine,
            mock_root,
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
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
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
}
