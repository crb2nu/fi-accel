//! Fast TOML parsing and conversion
//!
//! Provides TOML parsing with conversion to JSON-compatible values
//! for easy interop with existing JSON-based workflows.

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TomlError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Serialization error: {0}")]
    SerializeError(String),
    #[error("Invalid TOML at line {line}, column {column}: {message}")]
    InvalidToml {
        line: usize,
        column: usize,
        message: String,
    },
}

/// Parse TOML string to a serde_json::Value
///
/// This allows TOML to be used interchangeably with JSON in workflows
/// that expect JSON-like data structures.
pub fn parse_toml(toml_str: &str) -> Result<serde_json::Value, TomlError> {
    let toml_value: toml::Value =
        toml::from_str(toml_str).map_err(|e| TomlError::ParseError(e.to_string()))?;

    toml_to_json(&toml_value)
}

/// Parse TOML string to a specific type
pub fn parse_toml_as<T: DeserializeOwned>(toml_str: &str) -> Result<T, TomlError> {
    toml::from_str(toml_str).map_err(|e| TomlError::ParseError(e.to_string()))
}

/// Convert a JSON value to TOML string
pub fn json_to_toml(json: &serde_json::Value) -> Result<String, TomlError> {
    let toml_value = json_to_toml_value(json)?;
    toml::to_string_pretty(&toml_value).map_err(|e| TomlError::SerializeError(e.to_string()))
}

/// Convert a serializable value to TOML string
pub fn to_toml<T: Serialize>(value: &T) -> Result<String, TomlError> {
    toml::to_string_pretty(value).map_err(|e| TomlError::SerializeError(e.to_string()))
}

/// Validate TOML syntax without fully parsing
pub fn validate_toml(toml_str: &str) -> Result<(), TomlError> {
    toml_str
        .parse::<toml::Table>()
        .map_err(|e| {
            // Extract line/column info if available
            let message = e.message();
            if let Some(span) = e.span() {
                // Calculate line and column from span
                let (line, column) = line_col_from_offset(toml_str, span.start);
                TomlError::InvalidToml {
                    line,
                    column,
                    message: message.to_string(),
                }
            } else {
                TomlError::ParseError(message.to_string())
            }
        })?;
    Ok(())
}

/// Get line and column from byte offset
fn line_col_from_offset(s: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, c) in s.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Convert toml::Value to serde_json::Value
fn toml_to_json(toml: &toml::Value) -> Result<serde_json::Value, TomlError> {
    match toml {
        toml::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        toml::Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| TomlError::SerializeError("Invalid float value".to_string())),
        toml::Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        toml::Value::Datetime(dt) => Ok(serde_json::Value::String(dt.to_string())),
        toml::Value::Array(arr) => {
            let json_arr: Result<Vec<_>, _> = arr.iter().map(toml_to_json).collect();
            Ok(serde_json::Value::Array(json_arr?))
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k.clone(), toml_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
    }
}

/// Convert serde_json::Value to toml::Value
fn json_to_toml_value(json: &serde_json::Value) -> Result<toml::Value, TomlError> {
    match json {
        serde_json::Value::Null => {
            // TOML doesn't have null, use empty string as placeholder
            Ok(toml::Value::String(String::new()))
        }
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                Err(TomlError::SerializeError("Invalid number".to_string()))
            }
        }
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let toml_arr: Result<Vec<_>, _> = arr.iter().map(json_to_toml_value).collect();
            Ok(toml::Value::Array(toml_arr?))
        }
        serde_json::Value::Object(obj) => {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                table.insert(k.clone(), json_to_toml_value(v)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

/// Parse multiple TOML documents separated by `---`
pub fn parse_toml_multi(content: &str) -> Vec<Result<serde_json::Value, TomlError>> {
    content
        .split("\n---\n")
        .filter(|s| !s.trim().is_empty())
        .map(parse_toml)
        .collect()
}

/// Merge multiple TOML tables, with later values overriding earlier ones
pub fn merge_toml(base: &str, overlay: &str) -> Result<serde_json::Value, TomlError> {
    let base_value = parse_toml(base)?;
    let overlay_value = parse_toml(overlay)?;

    Ok(merge_json_objects(base_value, overlay_value))
}

/// Deep merge two JSON objects
fn merge_json_objects(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    match (base, overlay) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(overlay_map)) => {
            for (k, v) in overlay_map {
                let merged = if let Some(base_v) = base_map.remove(&k) {
                    merge_json_objects(base_v, v)
                } else {
                    v
                };
                base_map.insert(k, merged);
            }
            serde_json::Value::Object(base_map)
        }
        (_, overlay) => overlay, // Non-objects: overlay wins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_toml() {
        let toml_str = r#"
            name = "test"
            version = "1.0.0"

            [package]
            authors = ["Alice", "Bob"]
        "#;

        let result = parse_toml(toml_str).unwrap();
        assert_eq!(result["name"], "test");
        assert_eq!(result["version"], "1.0.0");
        assert_eq!(result["package"]["authors"][0], "Alice");
    }

    #[test]
    fn test_parse_toml_with_types() {
        let toml_str = r#"
            string = "hello"
            integer = 42
            float = 3.14
            boolean = true
            array = [1, 2, 3]
        "#;

        let result = parse_toml(toml_str).unwrap();
        assert_eq!(result["string"], "hello");
        assert_eq!(result["integer"], 42);
        assert!(result["float"].as_f64().unwrap() - 3.14 < 0.001);
        assert_eq!(result["boolean"], true);
        assert_eq!(result["array"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_validate_valid_toml() {
        let toml_str = r#"
            [section]
            key = "value"
        "#;
        assert!(validate_toml(toml_str).is_ok());
    }

    #[test]
    fn test_validate_invalid_toml() {
        let toml_str = r#"
            [section
            key = "value"
        "#;
        let result = validate_toml(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_to_toml() {
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "enabled": true,
            "tags": ["a", "b"]
        });

        let toml_str = json_to_toml(&json).unwrap();
        assert!(toml_str.contains("name = \"test\""));
        assert!(toml_str.contains("count = 42"));
        assert!(toml_str.contains("enabled = true"));
    }

    #[test]
    fn test_roundtrip() {
        let original = r#"
            name = "roundtrip"
            value = 123

            [nested]
            key = "value"
        "#;

        let json = parse_toml(original).unwrap();
        let back_to_toml = json_to_toml(&json).unwrap();
        let json_again = parse_toml(&back_to_toml).unwrap();

        assert_eq!(json, json_again);
    }

    #[test]
    fn test_merge_toml() {
        let base = r#"
            name = "base"
            version = "1.0"

            [settings]
            debug = false
            timeout = 30
        "#;

        let overlay = r#"
            version = "2.0"

            [settings]
            debug = true
        "#;

        let merged = merge_toml(base, overlay).unwrap();
        assert_eq!(merged["name"], "base"); // From base
        assert_eq!(merged["version"], "2.0"); // Overridden
        assert_eq!(merged["settings"]["debug"], true); // Overridden
        assert_eq!(merged["settings"]["timeout"], 30); // From base
    }

    #[test]
    fn test_parse_toml_multi() {
        let content = r#"
name = "first"
---
name = "second"
---
name = "third"
"#;

        let results = parse_toml_multi(content);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_parse_cargo_toml_style() {
        let toml_str = r#"
            [package]
            name = "my-crate"
            version = "0.1.0"
            edition = "2021"

            [dependencies]
            serde = { version = "1.0", features = ["derive"] }
            tokio = "1.0"

            [dev-dependencies]
            criterion = "0.5"
        "#;

        let result = parse_toml(toml_str).unwrap();
        assert_eq!(result["package"]["name"], "my-crate");
        assert_eq!(result["dependencies"]["serde"]["version"], "1.0");
        assert_eq!(result["dependencies"]["tokio"], "1.0");
    }
}
