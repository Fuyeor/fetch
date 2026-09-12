// src/main.rs
mod config;
mod entities;
mod modules;
// mod utils;

use axum::{
    Router,
    extract::FromRef,
    routing::{delete, get, post},
};
use fplatform_oauth::{OAuthClient, OAuthConfig};
use hickory_resolver::TokioAsyncResolver;
use std::net::SocketAddr;

use crate::modules::{
    auth::controller as auth,
    /*
     domains::controller as domains,
    ingestions::controller as ingestions,
    sitemaps::controller as sitemaps,
    */
};

/// Shared application state across all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
    pub config: config::AppConfig,
    pub oauth: OAuthClient,
    pub http_client: reqwest::Client,
    pub dns_resolver: TokioAsyncResolver,
}

impl FromRef<AppState> for sea_orm::DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for config::AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for OAuthClient {
    fn from_ref(state: &AppState) -> Self {
        state.oauth.clone()
    }
}

impl FromRef<AppState> for reqwest::Client {
    fn from_ref(state: &AppState) -> Self {
        state.http_client.clone()
    }
}

impl FromRef<AppState> for TokioAsyncResolver {
    fn from_ref(state: &AppState) -> Self {
        state.dns_resolver.clone()
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = config::AppConfig::load();

    let db = sea_orm::Database::connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    // initialize fsocial OAuth
    let oauth_config = OAuthConfig::from_env().expect("Failed to load OAuth configuration");
    let oauth_client = OAuthClient::new(oauth_config).expect("Failed to create OAuth client");

    // initialize HTTP client and asynchronous DNS resolver
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let dns_resolver = TokioAsyncResolver::tokio_from_system_conf()
        .expect("Failed to initialize system DNS resolver");

    let state = AppState {
        db,
        config: config.clone(),
        oauth: oauth_client,
        http_client,
        dns_resolver,
    };

    let app = Router::new()
        // Auth Routes
        .route("/auth/callback", post(auth::oauth_callback))
        .route("/auth/refresh-token", post(auth::refresh))
        .route("/auth/me", get(auth::get_me))
        /*
          // Domain Routes
          .route("/domains", get(domains::list_domains).post(domains::add_domain))
          .route("/domains/:domain", delete(domains::delete_domain))
          .route("/domains/:domain/verify", post(domains::verify_domain))
          // Sitemap Routes
          .route("/domains/:domain/sitemaps", get(sitemaps::list_sitemaps).post(sitemaps::submit_sitemap))
          // Ingestion Routes
          .route("/domains/:domain/ingest", post(ingestions::trigger_ingest))
          .route("/domains/:domain/ingestions", get(ingestions::list_ingestions))
        */
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("🚀 Fetch Console API is running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
