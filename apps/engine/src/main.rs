// apps/engine/src/main.rs

mod config;
mod core;
mod fon;
mod index;
mod model;
mod search;
mod web;

use std::sync::Arc;

use config::Config;
use search::SearchEngine;
use web::{AppState, router};

/// Start the local Fetch search API with configurable data and index directories.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::from_env()?;
    let engine = Arc::new(SearchEngine::open(&config.index_root)?);
    let state = AppState::new(engine, config.data_root);
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    println!(
        "Fetch search API listening on http://{}",
        config.bind_address
    );
    axum::serve(listener, router(state)).await?;
    Ok(())
}
