// src/config.rs

/// Application configuration loaded strictly from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_key: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub port: u16,

    // search engine microservice (e.g. http://127.0.0.1:3000)
    pub engine_url: String,
    // domain DNS verification token salt
    pub verification_salt: String,
}

impl AppConfig {
    /// Loads configuration from environment variables with fail-fast validation.
    pub fn load() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be specified in `.env` file"),
            jwt_key: std::env::var("JWT_KEY").expect("JWT_KEY must be specified in `.env` file"),
            jwt_issuer: std::env::var("JWT_ISSUER")
                .expect("JWT_ISSUER must be specified in `.env` file"),
            jwt_audience: std::env::var("JWT_AUDIENCE")
                .expect("JWT_AUDIENCE must be specified in `.env` file"),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .expect("PORT must be specified in `.env` file"),
            engine_url: std::env::var("ENGINE_URL")
                .expect("ENGINE_URL must be specified in `.env` file"),
            verification_salt: std::env::var("VERIFICATION_SALT")
                .expect("VERIFICATION_SALT must be specified in `.env` file"),
        }
    }
}
