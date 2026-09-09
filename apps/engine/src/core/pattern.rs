// apps/engine/src/core/pattern.rs

use super::ast::{CoreResult, RuntimeValue};

/// Normalize and validate one FRL pattern before it becomes a mapping identity.
pub fn canonical_pattern(pattern: &str) -> CoreResult<String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("pattern must not be empty".into());
    }
    if !pattern.starts_with('/') {
        return Err("pattern must start with '/'".into());
    }
    if pattern.contains("//") {
        return Err("pattern must not contain repeated slashes".into());
    }
    if pattern.contains('?') || pattern.contains('#') {
        return Err("pattern must not contain a query or fragment".into());
    }
    if pattern.chars().any(char::is_control) {
        return Err("pattern must not contain control characters".into());
    }
    placeholder_names(pattern)?;
    Ok(pattern.to_string())
}

/// Extract and validate unique placeholder names from one FRL pattern.
pub fn placeholder_names(pattern: &str) -> CoreResult<Vec<String>> {
    if pattern.is_empty() {
        return Err("pattern must not be empty".into());
    }
    let mut names = Vec::new();
    let mut cursor = 0;
    while cursor < pattern.len() {
        let Some((offset, delimiter)) = pattern[cursor..]
            .char_indices()
            .find(|(_, character)| *character == '{' || *character == '}')
        else {
            break;
        };
        let position = cursor + offset;
        if delimiter == '}' {
            return Err("pattern contains an unmatched closing brace".into());
        }
        let close = pattern[position + 1..]
            .find('}')
            .map(|offset| position + 1 + offset)
            .ok_or_else(|| "pattern contains an unterminated parameter".to_string())?;
        let name = &pattern[position + 1..close];
        if name.is_empty()
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == '-'
            })
        {
            return Err("pattern contains an invalid parameter name".into());
        }
        if names.iter().any(|existing| existing == name) {
            return Err(format!("pattern contains duplicate parameter: {name}").into());
        }
        names.push(name.to_string());
        cursor = close + 1;
    }
    Ok(names)
}

/// Expand one finite set of bindings into canonical FRL paths.
pub fn expand_pattern(
    pattern: &str,
    params: &std::collections::BTreeMap<String, RuntimeValue>,
) -> CoreResult<Vec<String>> {
    let _ = placeholder_names(pattern)?;
    let mut variants = vec![String::new()];
    let mut cursor = 0;
    while let Some(open_offset) = pattern[cursor..].find('{') {
        let open = cursor + open_offset;
        let close = pattern[open..]
            .find('}')
            .map(|offset| open + offset)
            .ok_or_else(|| "pattern contains an unterminated parameter".to_string())?;
        let name = &pattern[open + 1..close];
        if name.is_empty() || name.contains('{') || name.contains('}') {
            return Err("pattern contains an invalid parameter name".into());
        }
        let prefix = &pattern[cursor..open];
        let value = params.get(name);
        let optional_terminal = value.is_none() && is_terminal_segment(pattern, open, close);
        let replacements = match value {
            Some(RuntimeValue::Array(values)) => values
                .iter()
                .map(path_value)
                .collect::<CoreResult<Vec<_>>>()?,
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
    if pattern[cursor..].contains('{') || pattern[cursor..].contains('}') {
        return Err("pattern contains an unmatched brace".into());
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

fn path_value(value: &RuntimeValue) -> CoreResult<String> {
    match value {
        RuntimeValue::String(value) | RuntimeValue::Unknown(value) | RuntimeValue::Enum(value) => {
            Ok(percent_encode_segment(value))
        }
        RuntimeValue::Number(value) => Ok(percent_encode_segment(value)),
        RuntimeValue::Boolean(value) => Ok(value.to_string()),
        RuntimeValue::Array(_) | RuntimeValue::Object(_) => {
            Err("path parameters must be scalar values or finite arrays".into())
        }
    }
}

/// Normalize a generated FRL to one absolute path.
pub fn canonical_path(value: &str) -> String {
    if value.starts_with('/') {
        value.to_string()
    } else {
        format!("/{value}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{canonical_pattern, expand_pattern};
    use crate::core::ast::RuntimeValue;

    #[test]
    fn canonical_pattern_rejects_ambiguous_paths() {
        assert!(canonical_pattern("/@{username}//posts").is_err());
        assert!(canonical_pattern("/@{username}?tab=posts").is_err());
        assert!(canonical_pattern("@{username}").is_err());
    }

    #[test]
    fn canonical_pattern_preserves_meaningful_trailing_slash() {
        assert_eq!(canonical_pattern("/posts/").unwrap(), "/posts/");
    }

    #[test]
    fn expands_array_parameters_to_multiple_paths() {
        let mut params = BTreeMap::new();
        params.insert(
            "tab".to_string(),
            RuntimeValue::Array(vec![
                RuntimeValue::Enum("thoughts".to_string()),
                RuntimeValue::Enum("comments".to_string()),
            ]),
        );
        params.insert(
            "username".to_string(),
            RuntimeValue::String("Fuyeor".to_string()),
        );
        let paths = expand_pattern("/@{username}/{tab}", &params).unwrap();
        assert_eq!(paths, ["/@Fuyeor/thoughts", "/@Fuyeor/comments"]);
    }

    #[test]
    fn missing_terminal_parameter_is_optional() {
        let mut params = BTreeMap::new();
        params.insert(
            "username".to_string(),
            RuntimeValue::String("Alice".to_string()),
        );
        assert_eq!(
            expand_pattern("/@{username}/{tab}", &params).unwrap(),
            ["/@Alice"]
        );
    }

    #[test]
    fn missing_non_terminal_parameter_is_rejected() {
        let params = BTreeMap::new();
        assert!(expand_pattern("/@{username}/profile", &params).is_err());
    }

    #[test]
    fn percent_encodes_path_segments() {
        let mut params = BTreeMap::new();
        params.insert(
            "username".to_string(),
            RuntimeValue::String("A B".to_string()),
        );
        assert_eq!(
            expand_pattern("/@{username}", &params).unwrap(),
            ["/@A%20B"]
        );
    }
}
