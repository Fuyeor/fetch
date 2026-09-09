// apps/engine/src/index/mod.rs

mod reader;
mod schema;
mod writer;

pub(crate) use reader::first_text;
pub(crate) use schema::{SearchFields, build_schema, fields_from_schema};
pub(crate) use writer::{IndexManifest, ManifestDocument, load_manifest, persist_manifest};
