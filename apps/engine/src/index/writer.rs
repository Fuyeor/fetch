// apps/engine/src/index/writer.rs

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::schema::IndexResult;

pub(crate) const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexManifest {
    pub(crate) version: u32,
    pub(crate) generation: u64,
    pub(crate) documents: BTreeMap<String, ManifestDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManifestDocument {
    pub(crate) content_hash: String,
    pub(crate) generation: u64,
}

impl Default for IndexManifest {
    fn default() -> Self {
        Self {
            version: MANIFEST_VERSION,
            generation: 0,
            documents: BTreeMap::new(),
        }
    }
}

pub(crate) fn load_manifest(path: &Path) -> IndexResult<IndexManifest> {
    if !path.exists() {
        return Ok(IndexManifest::default());
    }
    let bytes = std::fs::read(path)?;
    let manifest: IndexManifest = serde_json::from_slice(&bytes)?;
    if manifest.version != MANIFEST_VERSION {
        return Err(format!("unsupported index manifest version: {}", manifest.version).into());
    }
    Ok(manifest)
}

/// Persist the logical catalog after Tantivy commit using an atomic same-directory rename.
pub(crate) fn persist_manifest(path: &Path, manifest: &IndexManifest) -> IndexResult<()> {
    let temporary_path = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(manifest)?;
    let mut file = File::create(&temporary_path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    std::fs::rename(temporary_path, path)?;
    Ok(())
}
