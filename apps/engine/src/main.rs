// apps/engine/src/main.rs

mod config;
mod core;
mod crawler;
mod fon;
mod index;
mod model;
mod search;
mod web;

use std::sync::Arc;

use config::Config;
use crawler::{Crawler, CrawlerConfig, FetchPolicy};
use search::SearchEngine;
use web::{AppState, router};

/// Start the local Fetch search API with configurable data and index directories.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::from_env()?;
    let engine = Arc::new(SearchEngine::open(&config.index_root)?);
    let crawler = Arc::new(Crawler::open(
        &config.crawler_state_root,
        FetchPolicy::default(),
        CrawlerConfig::default(),
    )?);
    let state = AppState::new_with_crawler(engine, config.data_root, crawler);
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    println!(
        "Fetch search API listening on http://{}",
        config.bind_address
    );
    axum::serve(listener, router(state)).await?;
    Ok(())
}
