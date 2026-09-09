// apps/engine/src/index/reader.rs

use tantivy::schema::{Field, TantivyDocument, Value as TantivyValue};

/// Read the first stored string value for a field without exposing Tantivy values upstream.
pub(crate) fn first_text(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}
