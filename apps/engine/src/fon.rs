// apps/engine/src/fon.rs

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use fon_parser::{SchemaKind, Value, parse};

use crate::core::ast::{
    CoreResult, RuntimeValue, object_field_value, object_path_field, object_string_field,
    parse_root_value,
};
use crate::core::document::{SearchDocument, stable_mapping_id};
use crate::core::pattern::{canonical_pattern, expand_pattern, placeholder_names};

/// Describes one pattern-to-sitemap mapping declared by the site.
#[derive(Debug, Clone)]
pub struct MappingSpec {
    /// Internal key derived from the canonical pattern; never read from FON.
    pub mapping_id: String,
    pub pattern: String,
    pub datas: PathBuf,
}

/// Contains the finite pages materialized from all submitted mappings.
#[derive(Debug, Clone)]
pub struct LoadedCatalog {
    pub mappings: Vec<MappingSpec>,
    pub pages: Vec<SearchDocument>,
}

/// Load the three-file mock protocol from a local site root.
pub fn load_catalog(root: &Path) -> CoreResult<LoadedCatalog> {
    let mapping_path = resolve_site_path(root, Path::new("./well-known/search-patterns.fon"))?;
    let mapping_source = std::fs::read_to_string(&mapping_path)?;
    let mappings = parse_mappings(&mapping_source, &mapping_path)?;
    let mut pages = Vec::new();

    for mapping in &mappings {
        let data_path = resolve_site_path(root, &mapping.datas)?;
        let rows_source = std::fs::read_to_string(&data_path)?;
        let rows = parse_sitemap_rows(&rows_source, &data_path)?;
        for row in rows {
            pages.extend(materialize_row(root, mapping, &row)?);
        }
    }

    Ok(LoadedCatalog { mappings, pages })
}

/// Parse and semantically inspect the mapping index while preserving schema syntax.
pub(crate) fn parse_mappings(source: &str, path: &Path) -> CoreResult<Vec<MappingSpec>> {
    let result = parse(source);
    if result.has_errors() {
        return Err(format!(
            "invalid mapping FON {}: {:?}",
            path.display(),
            result.diagnostics
        )
        .into());
    }
    let item_ids = result
        .document
        .ast
        .root_array_items()
        .ok_or_else(|| format!("{} must contain a root array", path.display()))?;
    let mut mappings = Vec::with_capacity(item_ids.len());
    let mut mapping_patterns = BTreeSet::new();

    for item_id in item_ids {
        let value = result
            .document
            .ast
            .value(*item_id)
            .ok_or_else(|| "mapping references an invalid value".to_string())?;
        let Value::Object(object) = value else {
            return Err("each mapping must be an object".into());
        };
        for member in &object.members {
            let key = result
                .document
                .ast
                .member_key_text(*member)
                .ok_or_else(|| "mapping contains an invalid member".to_string())?;
            if !matches!(key, "pattern" | "params" | "datas") {
                return Err(format!("unsupported mapping field: {key}").into());
            }
        }
        let pattern = object_string_field(&result.document.ast, &object.members, "pattern")?
            .ok_or_else(|| "mapping.pattern is required".to_string())?;
        let pattern = canonical_pattern(&pattern)?;
        let datas = object_path_field(&result.document.ast, &object.members, "datas")?
            .ok_or_else(|| "mapping.datas is required".to_string())?;
        let params = object_field_value(&result.document.ast, &object.members, "params")
            .ok_or_else(|| "mapping.params is required".to_string())?;
        let placeholders = placeholder_names(&pattern)?;
        let Value::Schema(schema_value) = params else {
            return Err("mapping.params must be a struct schema".into());
        };
        if schema_value.kind != SchemaKind::Struct {
            return Err("mapping.params must be a struct schema".into());
        }
        let schema = result
            .document
            .ast
            .schema(schema_value.schema)
            .ok_or_else(|| "mapping.params references an invalid schema".to_string())?;
        let declared = schema
            .fields
            .iter()
            .map(|field| field.key.raw.as_str())
            .collect::<BTreeSet<_>>();
        if declared.len() != schema.fields.len() {
            return Err("mapping.params contains duplicate fields".into());
        }
        if declared
            .iter()
            .any(|name| !placeholders.iter().any(|placeholder| placeholder == *name))
            || placeholders
                .iter()
                .any(|name| !declared.contains(name.as_str()))
        {
            return Err("mapping.params fields must exactly match pattern placeholders".into());
        }
        if !mapping_patterns.insert(pattern.clone()) {
            return Err("mapping patterns must be unique after canonicalization".into());
        }
        mappings.push(MappingSpec {
            mapping_id: stable_mapping_id(&pattern),
            pattern,
            datas: PathBuf::from(datas),
        });
    }

    Ok(mappings)
}

/// Parse a finite sitemap binding file without fetching or executing its content.
pub(crate) fn parse_sitemap_rows(source: &str, path: &Path) -> CoreResult<Vec<RuntimeValue>> {
    let value = parse_root_value(source, path)?;
    let RuntimeValue::Array(rows) = value else {
        return Err(format!("{} must contain a root array", path.display()).into());
    };
    Ok(rows)
}

/// Materialize one sitemap row into one or more concrete pages.
fn materialize_row(
    root: &Path,
    mapping: &MappingSpec,
    row: &RuntimeValue,
) -> CoreResult<Vec<SearchDocument>> {
    let RuntimeValue::Object(object) = row else {
        return Err("each sitemap row must be an object".into());
    };
    let content_reference = object
        .get("content")
        .map(RuntimeValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "sitemap row.content is required".to_string())?;
    let content_path = resolve_site_path(root, Path::new(&content_reference))?;
    let content_source = std::fs::read_to_string(&content_path)?;
    materialize_row_from_source(
        mapping,
        row,
        &content_reference,
        &content_source,
        Path::new(&content_reference),
    )
}

/// Materialize one already-fetched content source for a sitemap row.
pub(crate) fn materialize_row_from_source(
    mapping: &MappingSpec,
    row: &RuntimeValue,
    source: &str,
    content_source: &str,
    display_path: &Path,
) -> CoreResult<Vec<SearchDocument>> {
    let RuntimeValue::Object(object) = row else {
        return Err("each sitemap row must be an object".into());
    };
    let RuntimeValue::Object(params) = object
        .get("params")
        .ok_or_else(|| "sitemap row.params must be an object".to_string())?
    else {
        return Err("sitemap row.params must be an object".into());
    };
    let updated_at = object
        .get("updated-at")
        .map(RuntimeValue::as_string)
        .unwrap_or_default();
    let content = parse_content_source(content_source, display_path)?;
    let urls = expand_pattern(&mapping.pattern, params)?;

    Ok(urls
        .into_iter()
        .map(|url| {
            SearchDocument::new(
                mapping.mapping_id.clone(),
                source.to_string(),
                url,
                choose_title(content.title.as_deref(), &content.body),
                content.body.clone(),
                updated_at.clone(),
                content.images.clone(),
                content.graph.clone(),
            )
        })
        .collect())
}

/// Parsed content envelope; image values are retained only as remote pointers.
#[derive(Debug, Clone)]
struct ContentPage {
    title: Option<String>,
    body: String,
    images: Vec<String>,
    graph: Option<String>,
}

fn parse_content_source(source: &str, path: &Path) -> CoreResult<ContentPage> {
    let value = parse_root_value(source, path)?;
    let RuntimeValue::Object(object) = value else {
        return Err(format!("{} must contain an object", path.display()).into());
    };
    let title = object
        .get("title")
        .map(RuntimeValue::as_string)
        .filter(|value| !value.trim().is_empty());
    let body = object
        .get("content")
        .map(RuntimeValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} must contain content", path.display()))?;
    let images = object
        .get("images")
        .and_then(RuntimeValue::as_array)
        .map(|values| {
            values
                .iter()
                .map(RuntimeValue::as_string)
                .filter(|value| !value.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let graph = object.get("graph").map(|value| format!("{value:?}"));
    Ok(ContentPage {
        title,
        body,
        images,
        graph,
    })
}

/// Select an explicit title, Markdown heading, or a bounded content prefix.
pub fn choose_title(explicit: Option<&str>, content: &str) -> String {
    if let Some(title) = explicit.map(str::trim).filter(|title| !title.is_empty()) {
        return title.to_string();
    }
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            if !heading.trim().is_empty() {
                return heading.trim().to_string();
            }
        }
    }
    content
        .split_whitespace()
        .take(10)
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_site_path(root: &Path, path: &Path) -> CoreResult<PathBuf> {
    let relative = path.strip_prefix("./").unwrap_or(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!("unsafe site path: {}", path.display()).into());
    }
    Ok(root.join(relative))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{choose_title, load_catalog, parse_mappings, stable_mapping_id};

    #[test]
    fn loads_repository_mock_catalog() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalog = load_catalog(&root).expect("repository mock catalog must load");
        assert_eq!(catalog.mappings.len(), 3);
        assert_eq!(catalog.pages.len(), 5);
        assert_eq!(
            catalog.mappings[0].mapping_id,
            stable_mapping_id("/@{username}")
        );
        assert_eq!(
            catalog.mappings[1].mapping_id,
            stable_mapping_id("/@{username}/{tab}")
        );
        assert_eq!(
            catalog.mappings[2].mapping_id,
            stable_mapping_id("/{locale}/thought/{id}")
        );
        assert!(
            catalog
                .pages
                .iter()
                .any(|page| page.url == "/zh-hans/thought/1001")
        );
        assert!(catalog.pages.iter().all(|page| page.images.len() == 1));
        assert!(
            catalog
                .pages
                .iter()
                .all(|page| page.document_id.starts_with("doc_"))
        );
        assert!(
            catalog
                .pages
                .iter()
                .all(|page| page.content_hash.starts_with("sha256:"))
        );
    }

    #[test]
    fn duplicate_mapping_patterns_are_rejected() {
        let source = r#"[
          { pattern = `/{username}` params = struct { username: string } datas = ./a.fon }
          { pattern = `/{username}` params = struct { username: string } datas = ./b.fon }
        ]"#;
        assert!(parse_mappings(source, std::path::Path::new("mappings.fon")).is_err());
    }

    #[test]
    fn explicit_mapping_id_is_rejected() {
        let source = r#"[
          { id = `profile` pattern = `/{username}` params = struct { username: string } datas = ./a.fon }
        ]"#;
        assert!(parse_mappings(source, std::path::Path::new("mappings.fon")).is_err());
    }

    #[test]
    fn mapping_params_must_match_pattern_placeholders() {
        let source = r#"[
          { pattern = `/{username}` params = struct { account: string } datas = ./a.fon }
        ]"#;
        assert!(parse_mappings(source, std::path::Path::new("mappings.fon")).is_err());
    }

    #[test]
    fn title_falls_back_to_markdown_heading() {
        assert_eq!(
            choose_title(None, "# A useful title\n\nBody"),
            "A useful title"
        );
    }

    #[test]
    fn title_falls_back_to_first_ten_whitespace_words() {
        assert_eq!(choose_title(None, "one two three"), "one two three");
    }
}
