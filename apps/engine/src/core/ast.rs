// apps/engine/src/core/ast.rs

use std::collections::BTreeMap;
use std::path::Path;

use fon_parser::{Ast, Member, StringPartKind, Value, parse};

pub type CoreResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Convert one FON source document into the supported object or array runtime value.
pub fn parse_root_value(source: &str, path: &Path) -> CoreResult<RuntimeValue> {
    let result = parse(source);
    if result.has_errors() {
        return Err(format!("invalid FON {}: {:?}", path.display(), result.diagnostics).into());
    }
    if let Some(items) = result.document.ast.root_array_items() {
        let values = items
            .iter()
            .map(|id| value_to_runtime(&result.document.ast, *id))
            .collect::<CoreResult<Vec<_>>>()?;
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

/// Extract a typed string field from an object member list.
pub fn object_string_field(
    ast: &Ast,
    members: &[fon_parser::MemberId],
    name: &str,
) -> CoreResult<Option<String>> {
    let Some(value) = object_field_value(ast, members, name) else {
        return Ok(None);
    };
    match value {
        Value::String(value) => Ok(Some(unquote_string(&value.raw))),
        _ => Err(format!("{name} must be a FON string").into()),
    }
}

/// Extract a path-like or string field from an object member list.
pub fn object_path_field(
    ast: &Ast,
    members: &[fon_parser::MemberId],
    name: &str,
) -> CoreResult<Option<String>> {
    let Some(value) = object_field_value(ast, members, name) else {
        return Ok(None);
    };
    match value {
        Value::Unknown(value) => Ok(Some(value.raw.clone())),
        Value::String(value) => Ok(Some(unquote_string(&value.raw))),
        _ => Err(format!("{name} must be a FON path or string").into()),
    }
}

/// Find a value by its raw FON member key.
pub fn object_field_value<'a>(
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

/// Convert parser values into the small, explicit runtime representation used by the engine.
pub fn value_to_runtime(ast: &Ast, value_id: fon_parser::ValueId) -> CoreResult<RuntimeValue> {
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
                .collect::<CoreResult<Vec<_>>>()?,
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

/// Decode the FON string envelope and only the escape sequences supported by the data layer.
pub fn unquote_string(raw: &str) -> String {
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

/// Runtime values accepted by sitemap bindings and content metadata.
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Boolean(bool),
    Number(String),
    String(String),
    Enum(String),
    Array(Vec<RuntimeValue>),
    Object(BTreeMap<String, RuntimeValue>),
    Unknown(String),
}

impl RuntimeValue {
    /// Return a scalar value as text while rejecting compound values by returning an empty string.
    pub fn as_string(&self) -> String {
        match self {
            Self::Boolean(value) => value.to_string(),
            Self::Number(value)
            | Self::String(value)
            | Self::Enum(value)
            | Self::Unknown(value) => value.clone(),
            Self::Array(_) | Self::Object(_) => String::new(),
        }
    }

    /// Borrow an array when the value is a collection.
    pub fn as_array(&self) -> Option<&[RuntimeValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }
}
