// apps/engine/src/config.rs

use std::net::SocketAddr;
use std::path::PathBuf;

pub type ConfigResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Runtime configuration loaded from environment variables with deterministic defaults.
#[derive(Debug, Clone)]
pub struct Config {
    pub data_root: PathBuf,
    pub index_root: PathBuf,
    pub bind_address: SocketAddr,
}

impl Config {
    /// Load engine configuration without hiding malformed environment values.
    pub fn from_env() -> ConfigResult<Self> {
        let data_root = env_path("FETCH_DATA_ROOT", ".")?;
        let index_root = env_path("FETCH_INDEX_ROOT", ".fetch-index")?;
        let bind_address = std::env::var("FETCH_BIND")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
            .parse()?;
        Ok(Self {
            data_root,
            index_root,
            bind_address,
        })
    }
}

fn env_path(name: &str, default: &str) -> ConfigResult<PathBuf> {
    Ok(std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default)))
}
