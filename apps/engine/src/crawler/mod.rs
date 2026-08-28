// apps/engine/src/crawler/mod.rs

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Url;
use tokio::time::{Duration, sleep};

use crate::fon::{LoadedCatalog, materialize_row_from_source, parse_mappings, parse_sitemap_rows};

mod fetcher;
mod state;

pub use fetcher::{FetchPolicy, FetchedResource, RemoteFetcher};
pub use state::CrawlerState;
use state::SuccessUpdate;

pub type CrawlerResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Limits and retry policy for one explicitly submitted site ingestion.
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub refresh_after: u64,
    pub lease_seconds: u64,
    pub max_backoff_seconds: u64,
    pub max_attempts: u32,
    pub max_mappings: usize,
    pub max_rows: usize,
    pub max_documents: usize,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            refresh_after: 3600,
            lease_seconds: 120,
            max_backoff_seconds: 3600,
            max_attempts: 3,
            max_mappings: 256,
            max_rows: 100_000,
            max_documents: 1_000_000,
        }
    }
}

/// Counters for remote resources consumed while building one finite catalog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CrawlStats {
    pub fetched: usize,
    pub not_modified: usize,
    pub retries: usize,
    pub deferred: usize,
}

/// A crawler that never discovers links beyond the submitted mapping origin.
#[derive(Clone)]
pub struct Crawler {
    fetcher: RemoteFetcher,
    state: Arc<CrawlerState>,
    config: CrawlerConfig,
}

impl Crawler {
    /// Create a crawler with durable state and explicit network/resource limits.
    pub fn open(
        state_path: &Path,
        fetch_policy: FetchPolicy,
        config: CrawlerConfig,
    ) -> CrawlerResult<Self> {
        if config.max_attempts == 0 {
            return Err("crawler max_attempts must be greater than zero".into());
        }
        Ok(Self {
            fetcher: RemoteFetcher::new(fetch_policy)?,
            state: Arc::new(CrawlerState::open(state_path)?),
            config,
        })
    }

    /// Ingest only the mapping locator explicitly submitted by the webmaster.
    pub async fn ingest_mapping(&self, mapping_locator: &str) -> CrawlerResult<RemoteCatalog> {
        let mapping_url = Url::parse(mapping_locator)?;
        let mut stats = CrawlStats::default();
        let mapping_source = self.fetch_text(&mapping_url, &mut stats).await?;
        let mapping_path = PathBuf::from(mapping_url.as_str());
        let mappings = parse_mappings(&mapping_source, &mapping_path)?;
        if mappings.len() > self.config.max_mappings {
            return Err("mapping catalog exceeds mapping limit".into());
        }

        let mut pages = Vec::new();
        let mut total_rows = 0_usize;
        for mapping in &mappings {
            let data_url = same_origin_url(&mapping_url, &mapping.datas)?;
            let rows_source = self.fetch_text(&data_url, &mut stats).await?;
            let rows = parse_sitemap_rows(&rows_source, Path::new(data_url.as_str()))?;
            total_rows = total_rows.saturating_add(rows.len());
            if total_rows > self.config.max_rows {
                return Err("submitted sitemap rows exceed row limit".into());
            }
            let mut content_cache: BTreeMap<String, String> = BTreeMap::new();
            for row in rows {
                let content_reference = row_content_reference(&row)?;
                let content_url = same_origin_url(&data_url, &content_reference)?;
                let content_source = if let Some(source) = content_cache.get(content_url.as_str()) {
                    source.clone()
                } else {
                    let source = self.fetch_text(&content_url, &mut stats).await?;
                    content_cache.insert(content_url.to_string(), source.clone());
                    source
                };
                pages.extend(materialize_row_from_source(
                    mapping,
                    &row,
                    content_url.as_str(),
                    &content_source,
                    Path::new(content_url.as_str()),
                )?);
                if pages.len() > self.config.max_documents {
                    return Err("expanded documents exceed document limit".into());
                }
            }
        }
        Ok(RemoteCatalog {
            catalog: LoadedCatalog { mappings, pages },
            rows: total_rows,
            stats,
        })
    }

    async fn fetch_text(&self, url: &Url, stats: &mut CrawlStats) -> CrawlerResult<String> {
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        for attempt in 0..self.config.max_attempts {
            let previous = self.state.get(url.as_str())?;
            let Some(_) = self.state.try_acquire(
                url.as_str(),
                unix_timestamp(),
                self.config.lease_seconds,
            )?
            else {
                stats.deferred = stats.deferred.saturating_add(1);
                return Err("remote resource is currently leased".into());
            };
            match self.fetcher.fetch(url.as_str(), previous.as_ref()).await {
                Ok(FetchedResource {
                    url: fetched_url,
                    status: 304,
                    etag,
                    last_modified,
                    body,
                }) => {
                    let _ = self.state.apply_not_modified(
                        fetched_url.as_str(),
                        unix_timestamp(),
                        self.config.refresh_after,
                    )?;
                    let cached = self
                        .state
                        .body(fetched_url.as_str())?
                        .ok_or_else(|| "304 response has no cached body".to_string())?;
                    stats.not_modified = stats.not_modified.saturating_add(1);
                    let _ = (etag, last_modified, body);
                    return String::from_utf8(cached).map_err(Into::into);
                }
                Ok(FetchedResource {
                    url: fetched_url,
                    status,
                    etag,
                    last_modified,
                    body,
                }) => {
                    let text = String::from_utf8(body.clone())?;
                    self.state.apply_success(
                        fetched_url.as_str(),
                        SuccessUpdate {
                            status,
                            etag,
                            last_modified,
                            body: &body,
                            fetched_at: unix_timestamp(),
                            refresh_after: self.config.refresh_after,
                        },
                    )?;
                    stats.fetched = stats.fetched.saturating_add(1);
                    stats.retries = stats.retries.saturating_add(attempt as usize);
                    return Ok(text);
                }
                Err(error) => {
                    self.state.apply_failure(
                        url.as_str(),
                        unix_timestamp(),
                        self.config.max_backoff_seconds,
                    )?;
                    last_error = Some(error);
                    if attempt + 1 < self.config.max_attempts {
                        sleep(Duration::from_millis(
                            50_u64.saturating_mul(attempt as u64 + 1),
                        ))
                        .await;
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| "remote fetch failed".into()))
    }
}

/// A finite remote catalog ready for the existing authoritative index sync.
#[derive(Debug, Clone)]
pub struct RemoteCatalog {
    pub catalog: LoadedCatalog,
    pub rows: usize,
    pub stats: CrawlStats,
}

fn row_content_reference(row: &crate::core::ast::RuntimeValue) -> CrawlerResult<PathBuf> {
    let crate::core::ast::RuntimeValue::Object(object) = row else {
        return Err("each sitemap row must be an object".into());
    };
    object
        .get("content")
        .map(crate::core::ast::RuntimeValue::as_string)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "sitemap row.content is required".into())
}

fn same_origin_url(base: &Url, locator: &Path) -> CrawlerResult<Url> {
    let raw = locator
        .to_str()
        .ok_or_else(|| "remote locator must be valid UTF-8".to_string())?;
    if locator
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("submitted locator must not contain parent traversal".into());
    }
    let target = if let Ok(target) = Url::parse(raw) {
        target
    } else if let Some(root_relative) = raw.strip_prefix("./") {
        let mut target = base.clone();
        target.set_path(&format!("/{root_relative}"));
        target.set_query(None);
        target.set_fragment(None);
        target
    } else if raw.starts_with('/') {
        let mut target = base.clone();
        target.set_path(raw);
        target.set_query(None);
        target.set_fragment(None);
        target
    } else {
        return Err("submitted locator must be absolute or root-relative".into());
    };
    if !same_origin(base, &target) {
        return Err("submitted locator crosses site origin".into());
    }
    Ok(target)
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host() == right.host()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::Router;
    use axum::routing::get;
    use tempfile::tempdir;

    use super::{Crawler, CrawlerConfig, FetchPolicy};

    #[tokio::test]
    async fn ingests_only_same_origin_submitted_fon_resources() {
        let app = Router::new().route(
            "/search-patterns.fon",
            get(|| async {
                "[{ pattern = `/profile/{username}` params = struct { username: string } datas = `/sitemap.fon` }]"
            }),
        ).route(
            "/sitemap.fon",
            get(|| async {
                "[{ params = { username = `Alice` } content = `/content.fon` }]"
            }),
        ).route(
            "/content.fon",
            get(|| async { "{ title = `Alice` content = `# Hello` }" }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let root = tempdir().unwrap();
        let crawler = Crawler::open(
            &root.path().join("state"),
            FetchPolicy::for_tests(),
            CrawlerConfig {
                max_attempts: 1,
                ..CrawlerConfig::default()
            },
        )
        .unwrap();
        let result = crawler
            .ingest_mapping(&format!("http://{address}/search-patterns.fon"))
            .await
            .unwrap();
        assert_eq!(result.catalog.mappings.len(), 1);
        assert_eq!(result.catalog.pages.len(), 1);
        assert_eq!(result.stats.fetched, 3);
    }

    #[tokio::test]
    async fn rejects_cross_origin_sitemap_locator() {
        let root = tempdir().unwrap();
        let crawler = Crawler::open(
            &root.path().join("state"),
            FetchPolicy::for_tests(),
            CrawlerConfig {
                max_attempts: 1,
                ..CrawlerConfig::default()
            },
        )
        .unwrap();
        let result = crawler
            .ingest_mapping("http://127.0.0.1:9/search-patterns.fon")
            .await;
        assert!(result.is_err());
    }
}
