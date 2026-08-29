// apps/engine/src/crawler/state.rs

use std::path::Path;
use std::sync::Arc;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::CrawlerResult;

const RECORDS: TableDefinition<&str, &[u8]> = TableDefinition::new("resource");
const BODIES: TableDefinition<&str, &[u8]> = TableDefinition::new("body");

/// Persisted metadata for one submitted remote resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRecord {
    pub url: String,
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub last_modified: Option<String>,
    pub status: u16,
    #[serde(default)]
    pub content_hash: String,
    pub fetched_at: u64,
    pub next_fetch_at: u64,
    pub retry_count: u32,
    pub lease_until: u64,
}

/// Input payload for an atomic successful resource update.
pub(crate) struct SuccessUpdate<'a> {
    pub status: u16,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub body: &'a [u8],
    pub fetched_at: u64,
    pub refresh_after: u64,
}

/// Durable crawler state backed by one redb database file.
#[derive(Clone)]
pub struct CrawlerState {
    db: Arc<Database>,
}

impl CrawlerState {
    /// Open or create the crawler state database with explicit persistence semantics.
    pub fn open(path: &Path) -> CrawlerResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db_path = if path.extension().is_some() {
            path.to_path_buf()
        } else {
            path.join("crawler.redb")
        };
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Database::create(db_path)?;
        Ok(Self { db: Arc::new(db) })
    }

    /// Read the last response metadata for a submitted URL.
    pub fn get(&self, url: &str) -> CrawlerResult<Option<ResourceRecord>> {
        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(RECORDS) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        table
            .get(record_key(url).as_str())?
            .map(|value| serde_json::from_slice(value.value()))
            .transpose()
            .map_err(Into::into)
    }

    /// Read the cached response body used when a conditional request returns 304.
    pub fn body(&self, url: &str) -> CrawlerResult<Option<Vec<u8>>> {
        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(BODIES) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        Ok(table
            .get(body_key(url).as_str())?
            .map(|value| value.value().to_vec()))
    }

    /// Acquire a process-scoped lease so one resource is fetched by one worker at a time.
    pub fn try_acquire(
        &self,
        url: &str,
        now: u64,
        lease_seconds: u64,
    ) -> CrawlerResult<Option<ResourceRecord>> {
        let key = record_key(url);
        let write_txn = self.db.begin_write()?;
        let mut table = write_txn.open_table(RECORDS)?;
        let mut record = table
            .get(key.as_str())?
            .map(|value| serde_json::from_slice(value.value()))
            .transpose()?
            .unwrap_or_else(|| ResourceRecord {
                url: url.to_string(),
                etag: None,
                last_modified: None,
                status: 0,
                content_hash: String::new(),
                fetched_at: 0,
                next_fetch_at: 0,
                retry_count: 0,
                lease_until: 0,
            });
        if record.lease_until > now {
            return Ok(None);
        }
        record.lease_until = now.saturating_add(lease_seconds);
        let serialized = serde_json::to_vec(&record)?;
        table.insert(key.as_str(), serialized.as_slice())?;
        drop(table);
        write_txn.commit()?;
        Ok(Some(record))
    }

    /// Atomically persist a successful body and its cache validators.
    pub fn apply_success(
        &self,
        url: &str,
        update: SuccessUpdate<'_>,
    ) -> CrawlerResult<ResourceRecord> {
        let record = ResourceRecord {
            url: url.to_string(),
            etag: update.etag,
            last_modified: update.last_modified,
            status: update.status,
            content_hash: sha256(update.body),
            fetched_at: update.fetched_at,
            next_fetch_at: update.fetched_at.saturating_add(update.refresh_after),
            retry_count: 0,
            lease_until: 0,
        };
        let serialized = serde_json::to_vec(&record)?;
        let record_key = record_key(url);
        let body_key = body_key(url);
        let write_txn = self.db.begin_write()?;
        {
            let mut records = write_txn.open_table(RECORDS)?;
            records.insert(record_key.as_str(), serialized.as_slice())?;
            let mut bodies = write_txn.open_table(BODIES)?;
            bodies.insert(body_key.as_str(), update.body)?;
        }
        write_txn.commit()?;
        Ok(record)
    }

    /// Persist a 304 result while retaining the previously cached response body.
    pub fn apply_not_modified(
        &self,
        url: &str,
        fetched_at: u64,
        refresh_after: u64,
    ) -> CrawlerResult<Option<ResourceRecord>> {
        let Some(mut record) = self.get(url)? else {
            return Ok(None);
        };
        record.status = 304;
        record.fetched_at = fetched_at;
        record.next_fetch_at = fetched_at.saturating_add(refresh_after);
        record.retry_count = 0;
        record.lease_until = 0;
        let serialized = serde_json::to_vec(&record)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(RECORDS)?;
            table.insert(record_key(url).as_str(), serialized.as_slice())?;
        }
        write_txn.commit()?;
        Ok(Some(record))
    }

    /// Record a failed attempt with bounded exponential backoff and release its lease.
    pub fn apply_failure(&self, url: &str, now: u64, max_backoff: u64) -> CrawlerResult<()> {
        let current = self.get(url)?;
        let mut record = current.unwrap_or_else(|| ResourceRecord {
            url: url.to_string(),
            etag: None,
            last_modified: None,
            status: 0,
            content_hash: String::new(),
            fetched_at: 0,
            next_fetch_at: now,
            retry_count: 0,
            lease_until: 0,
        });
        record.retry_count = record.retry_count.saturating_add(1);
        let exponent = record.retry_count.min(10);
        let delay = 5_u64.saturating_mul(1_u64 << exponent).min(max_backoff);
        record.next_fetch_at = now.saturating_add(delay);
        record.lease_until = 0;
        let serialized = serde_json::to_vec(&record)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(RECORDS)?;
            table.insert(record_key(url).as_str(), serialized.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

fn record_key(url: &str) -> String {
    format!("resource/{}", hex_digest(url_hash(url)))
}

fn body_key(url: &str) -> String {
    format!("body/{}", hex_digest(url_hash(url)))
}

fn url_hash(url: &str) -> [u8; 32] {
    Sha256::digest(url.as_bytes()).into()
}

fn sha256(body: &[u8]) -> String {
    format!("sha256:{}", hex_digest(Sha256::digest(body)))
}

fn hex_digest<D: AsRef<[u8]>>(digest: D) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::CrawlerState;

    #[test]
    fn lease_and_conditional_body_state_are_persistent() {
        let directory = tempdir().unwrap();
        let state = CrawlerState::open(&directory.path().join("state")).unwrap();
        let url = "https://example.com/catalog.fon";
        assert!(state.try_acquire(url, 100, 60).unwrap().is_some());
        assert!(state.try_acquire(url, 101, 60).unwrap().is_none());
        state
            .apply_success(
                url,
                super::SuccessUpdate {
                    status: 200,
                    etag: Some("etag-1".to_string()),
                    last_modified: None,
                    body: b"catalog",
                    fetched_at: 100,
                    refresh_after: 3600,
                },
            )
            .unwrap();
        assert_eq!(state.body(url).unwrap().as_deref(), Some(&b"catalog"[..]));
        assert_eq!(
            state.get(url).unwrap().unwrap().etag.as_deref(),
            Some("etag-1")
        );
        let unchanged = state.apply_not_modified(url, 200, 3600).unwrap().unwrap();
        assert_eq!(unchanged.status, 304);
        assert_eq!(state.body(url).unwrap().as_deref(), Some(&b"catalog"[..]));
    }

    #[test]
    fn failure_backoff_is_bounded_and_releases_lease() {
        let directory = tempdir().unwrap();
        let state = CrawlerState::open(&directory.path().join("state")).unwrap();
        let url = "https://example.com/fail.fon";
        state.try_acquire(url, 100, 60).unwrap();
        state.apply_failure(url, 100, 120).unwrap();
        let record = state.get(url).unwrap().unwrap();
        assert_eq!(record.retry_count, 1);
        assert_eq!(record.lease_until, 0);
        assert_eq!(record.next_fetch_at, 110);
    }
}
