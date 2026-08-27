// apps/fetch/back-end/src/main.rs

mod fon;
mod model;
mod search;
mod web;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use search::SearchEngine;
use web::{AppState, router};

/// Start the local Fetch search API with configurable data and index directories.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_root = std::env::var_os("FETCH_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let index_root = std::env::var_os("FETCH_INDEX_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".fetch-index"));
    let bind_address = std::env::var("FETCH_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let engine = Arc::new(SearchEngine::open(&index_root)?);
    let state = AppState::new(engine, data_root);
    let address: SocketAddr = bind_address.parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Fetch search API listening on http://{address}");
    axum::serve(listener, router(state)).await?;
    Ok(())
}
