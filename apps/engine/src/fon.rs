// apps/fetch/back-end/src/fon.rs

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use fon_parser::{Ast, Member, StringPartKind, Value, parse};

use crate::model::IndexPage;

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Describes one pattern-to-sitemap mapping declared by the site.
#[derive(Debug, Clone)]
pub struct MappingSpec {
    pub pattern: String,
    pub datas: PathBuf,
}

/// Contains the finite pages materialized from all submitted mappings.
#[derive(Debug, Clone)]
pub struct LoadedCatalog {
    pub mappings: Vec<MappingSpec>,
    pub pages: Vec<IndexPage>,
}

/// Load the three-file mock protocol from a local site root.
pub fn load_catalog(root: &Path) -> AppResult<LoadedCatalog> {
    let mapping_path = resolve_site_path(root, Path::new("./well-known/search-patterns.fon"))?;
    let mapping_source = std::fs::read_to_string(&mapping_path)?;
    let mappings = parse_mappings(&mapping_source, &mapping_path)?;
    let mut pages = Vec::new();

    for mapping in &mappings {
        let data_path = resolve_site_path(root, &mapping.datas)?;
        let rows_source = std::fs::read_to_string(&data_path)?;
        let rows = parse_root_value(&rows_source, &data_path)?;
        let RuntimeValue::Array(rows) = rows else {
            return Err(format!("{} must contain a root array", data_path.display()).into());
        };
        for row in rows {
            pages.extend(materialize_row(root, mapping, &row)?);
        }
    }

    Ok(LoadedCatalog { mappings, pages })
}

/// Parse and semantically inspect the mapping index while preserving schema syntax.
fn parse_mappings(source: &str, path: &Path) -> AppResult<Vec<MappingSpec>> {
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

    for item_id in item_ids {
        let value = result
            .document
            .ast
            .value(*item_id)
            .ok_or_else(|| "mapping references an invalid value".to_string())?;
        let Value::Object(object) = value else {
            return Err("each mapping must be an object".into());
        };
        let pattern = object_string_field(&result.document.ast, &object.members, "pattern")?
            .ok_or_else(|| "mapping.pattern is required".to_string())?;
        let datas = object_path_field(&result.document.ast, &object.members, "datas")?
            .ok_or_else(|| "mapping.datas is required".to_string())?;
        let params = object_field_value(&result.document.ast, &object.members, "params")
            .ok_or_else(|| "mapping.params is required".to_string())?;
        if !matches!(params, Value::Schema(_)) {
            return Err("mapping.params must be a struct or enum schema".into());
        }
        mappings.push(MappingSpec {
            pattern,
            datas: PathBuf::from(datas),
        });
    }

    Ok(mappings)
}

/// Materialize one sitemap row into one or more concrete pages.
fn materialize_row(
    root: &Path,
    mapping: &MappingSpec,
    row: &RuntimeValue,
) -> AppResult<Vec<IndexPage>> {
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
    let content_reference = object
        .get("content")
        .map(RuntimeValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "sitemap row.content is required".to_string())?;
    let content_path = resolve_site_path(root, Path::new(&content_reference))?;
    let content = parse_content(&content_path)?;
    let urls = expand_pattern(&mapping.pattern, params)?;

    Ok(urls
        .into_iter()
        .map(|url| IndexPage {
            url,
            title: choose_title(content.title.as_deref(), &content.body),
            body: content.body.clone(),
            updated_at: updated_at.clone(),
            images: content.images.clone(),
            graph: content.graph.clone(),
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

/// Parse content FON without interpreting HTML; content remains plain text/Markdown.
fn parse_content(path: &Path) -> AppResult<ContentPage> {
    let source = std::fs::read_to_string(path)?;
    let value = parse_root_value(&source, path)?;
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

/// Expand scalar, optional and array bindings without enumerating an unbounded space.
fn expand_pattern(
    pattern: &str,
    params: &BTreeMap<String, RuntimeValue>,
) -> AppResult<Vec<String>> {
    let mut variants = vec![String::new()];
    let mut cursor = 0;
    while let Some(open_offset) = pattern[cursor..].find('{') {
        let open = cursor + open_offset;
        let close = pattern[open..]
            .find('}')
            .map(|offset| open + offset)
            .ok_or_else(|| "pattern contains an unterminated parameter".to_string())?;
        let name = &pattern[open + 1..close];
        let prefix = &pattern[cursor..open];
        let value = params.get(name);
        let optional_terminal = value.is_none() && is_terminal_segment(pattern, open, close);
        let replacements = match value {
            Some(RuntimeValue::Array(values)) => values
                .iter()
                .map(path_value)
                .collect::<AppResult<Vec<_>>>()?,
            Some(value) => vec![path_value(value)?],
            None if optional_terminal => vec![String::new()],
            None => return Err(format!("missing non-terminal parameter: {name}").into()),
        };
        let prefix = if optional_terminal {
            prefix.trim_end_matches('/')
        } else {
            prefix
        };
        variants = variants
            .into_iter()
            .flat_map(|base| {
                replacements
                    .iter()
                    .map(move |replacement| format!("{base}{prefix}{replacement}"))
            })
            .collect();
        cursor = close + 1;
    }
    let suffix = &pattern[cursor..];
    Ok(variants
        .into_iter()
        .map(|value| canonical_path(&format!("{value}{suffix}")))
        .collect())
}

/// Detect a missing placeholder at the end of a path segment.
fn is_terminal_segment(pattern: &str, open: usize, close: usize) -> bool {
    pattern[..open].ends_with('/') && close + 1 == pattern.len()
}

/// Percent-encode one path value while keeping unreserved URI characters intact.
fn percent_encode_segment(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn path_value(value: &RuntimeValue) -> AppResult<String> {
    match value {
        RuntimeValue::String(value) | RuntimeValue::Unknown(value) | RuntimeValue::Enum(value) => {
            Ok(percent_encode_segment(value))
        }
        RuntimeValue::Number(value) => Ok(percent_encode_segment(value)),
        RuntimeValue::Boolean(value) => Ok(value.to_string()),
        _ => Err("path parameters must be scalar strings, numbers, booleans, or arrays".into()),
    }
}

fn canonical_path(value: &str) -> String {
    if value.starts_with('/') {
        value.to_string()
    } else {
        format!("/{value}")
    }
}

fn resolve_site_path(root: &Path, path: &Path) -> AppResult<PathBuf> {
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

fn object_string_field(
    ast: &Ast,
    members: &[fon_parser::MemberId],
    name: &str,
) -> AppResult<Option<String>> {
    let Some(value) = object_field_value(ast, members, name) else {
        return Ok(None);
    };
    match value {
        Value::String(value) => Ok(Some(unquote_string(&value.raw))),
        _ => Err(format!("{name} must be a FON string").into()),
    }
}

fn object_path_field(
    ast: &Ast,
    members: &[fon_parser::MemberId],
    name: &str,
) -> AppResult<Option<String>> {
    let Some(value) = object_field_value(ast, members, name) else {
        return Ok(None);
    };
    match value {
        Value::Unknown(value) => Ok(Some(value.raw.clone())),
        Value::String(value) => Ok(Some(unquote_string(&value.raw))),
        _ => Err(format!("{name} must be a FON path or string").into()),
    }
}

fn object_field_value<'a>(
    ast: &'a Ast,
    members: &[fon_parser::MemberId],
    name: &str,
) -> Option<&'a Value> {
    members.iter().find_map(|id| {
        let binding = ast.member(*id).and_then(Member::binding)?;
        (binding.key.raw == name)
            .then(|| ast.value(binding.value))
            .flatten()
    })
}

fn unquote_string(raw: &str) -> String {
    let value = raw
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(raw);
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('`') => output.push('`'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

/// Convert parsed AST values into a small, explicit runtime representation.
#[derive(Debug, Clone)]
enum RuntimeValue {
    Boolean(bool),
    Number(String),
    String(String),
    Enum(String),
    Array(Vec<RuntimeValue>),
    Object(BTreeMap<String, RuntimeValue>),
    Unknown(String),
}

impl RuntimeValue {
    fn as_string(&self) -> String {
        match self {
            Self::Boolean(value) => value.to_string(),
            Self::Number(value)
            | Self::String(value)
            | Self::Enum(value)
            | Self::Unknown(value) => value.clone(),
            Self::Array(_) | Self::Object(_) => String::new(),
        }
    }

    fn as_array(&self) -> Option<&[RuntimeValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }
}

fn parse_root_value(source: &str, path: &Path) -> AppResult<RuntimeValue> {
    let result = parse(source);
    if result.has_errors() {
        return Err(format!("invalid FON {}: {:?}", path.display(), result.diagnostics).into());
    }
    if let Some(items) = result.document.ast.root_array_items() {
        let values = items
            .iter()
            .map(|id| value_to_runtime(&result.document.ast, *id))
            .collect::<AppResult<Vec<_>>>()?;
        return Ok(RuntimeValue::Array(values));
    }
    let members = result
        .document
        .ast
        .object_members()
        .ok_or_else(|| format!("{} has no supported root", path.display()))?;
    let mut object = BTreeMap::new();
    for member_id in members {
        let member = result
            .document
            .ast
            .member(*member_id)
            .ok_or_else(|| "invalid member reference".to_string())?;
        let binding = member
            .binding()
            .ok_or_else(|| "data objects cannot contain type declarations".to_string())?;
        let value = value_to_runtime(&result.document.ast, binding.value)?;
        if object.insert(binding.key.raw.clone(), value).is_some() {
            return Err(format!("duplicate FON field: {}", binding.key.raw).into());
        }
    }
    Ok(RuntimeValue::Object(object))
}

fn value_to_runtime(ast: &Ast, value_id: fon_parser::ValueId) -> AppResult<RuntimeValue> {
    let value = ast
        .value(value_id)
        .ok_or_else(|| "invalid value reference".to_string())?;
    match value {
        Value::Boolean { value, .. } => Ok(RuntimeValue::Boolean(*value)),
        Value::Number { raw, .. } => Ok(RuntimeValue::Number(raw.clone())),
        Value::String(value) => {
            if value
                .parts
                .iter()
                .any(|part| part.kind == StringPartKind::Interpolation)
            {
                return Err("FON interpolation is not allowed in data files".into());
            }
            Ok(RuntimeValue::String(unquote_string(&value.raw)))
        }
        Value::EnumPath(value) => Ok(RuntimeValue::Enum(
            value.path.trim_start_matches('.').to_string(),
        )),
        Value::Array(value) => Ok(RuntimeValue::Array(
            value
                .items
                .iter()
                .map(|id| value_to_runtime(ast, *id))
                .collect::<AppResult<Vec<_>>>()?,
        )),
        Value::Object(value) => {
            let mut object = BTreeMap::new();
            for member_id in &value.members {
                let member = ast
                    .member(*member_id)
                    .ok_or_else(|| "invalid nested member reference".to_string())?;
                let binding = member.binding().ok_or_else(|| {
                    "nested data objects cannot contain type declarations".to_string()
                })?;
                let nested = value_to_runtime(ast, binding.value)?;
                if object.insert(binding.key.raw.clone(), nested).is_some() {
                    return Err(format!("duplicate FON field: {}", binding.key.raw).into());
                }
            }
            Ok(RuntimeValue::Object(object))
        }
        Value::Unknown(value) => Ok(RuntimeValue::Unknown(value.raw.clone())),
        Value::Regex(_) | Value::Schema(_) | Value::Expression(_) | Value::Error(_) => {
            Err("unsupported executable or schema value in data file".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{choose_title, load_catalog};

    #[test]
    fn loads_repository_mock_catalog() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let catalog = load_catalog(&root).expect("repository mock catalog must load");
        assert_eq!(catalog.mappings.len(), 3);
        assert_eq!(catalog.pages.len(), 5);
        assert!(
            catalog
                .pages
                .iter()
                .any(|page| page.url == "/zh-hans/thought/1001")
        );
        assert!(catalog.pages.iter().all(|page| page.images.len() == 1));
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
