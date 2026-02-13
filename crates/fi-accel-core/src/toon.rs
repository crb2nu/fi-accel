//! TOON (Token Oriented Object Notation) parser and serializer
//!
//! TOON is a human-readable, token-efficient serialization format designed for
//! LLM context windows. It features:
//! - Indentation-based nesting (2-space)
//! - Compact array notation: `name[count]: val1,val2,val3`
//! - Tabular data: `name[count]{col1,col2}:` with rows below
//! - Minimal quoting (only when necessary)

use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToonError {
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    #[error("Invalid indentation at line {line}")]
    IndentError { line: usize },
    #[error("Unexpected token at line {line}: {token}")]
    UnexpectedToken { line: usize, token: String },
}

static NUMERIC_RE: OnceLock<Regex> = OnceLock::new();

fn numeric_regex() -> &'static Regex {
    NUMERIC_RE.get_or_init(|| Regex::new(r"^[+-]?(?:\d+|\d*\.\d+)(?:[eE][+-]?\d+)?$").unwrap())
}

// ============================================================================
// Serializer (Encode)
// ============================================================================

/// Serialize a JSON value to TOON format
pub fn to_toon(value: &Value) -> String {
    let mut lines = Vec::new();
    match value {
        Value::Object(obj) => encode_object(&mut lines, 0, obj),
        Value::Array(arr) => encode_array(&mut lines, 0, "items", arr, true),
        _ => {
            if let Some(tok) = primitive_token(value) {
                return tok;
            }
            return String::new();
        }
    }
    lines.join("\n")
}

/// Check if a string needs quoting
fn needs_quotes(text: &str, delimiter: char) -> bool {
    if text.is_empty() {
        return true;
    }

    let lower = text.to_lowercase();
    if lower == "true" || lower == "false" || lower == "null" {
        return true;
    }

    if numeric_regex().is_match(text) {
        return true;
    }

    for ch in text.chars() {
        let code = ch as u32;
        if code < 0x20 || code == 0x7F {
            return true;
        }
        if ch == ' ' || ch == '"' || ch == '\\' || ch == ':' || ch == '{' || ch == '}' || ch == '['
            || ch == ']' || ch == delimiter
        {
            return true;
        }
    }

    false
}

/// Quote a string with escape sequences
fn quote_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Convert a string to a TOON token (quoted if necessary)
fn string_token(text: &str, delimiter: char) -> String {
    if text.is_empty() {
        return "\"\"".to_string();
    }
    if needs_quotes(text, delimiter) {
        quote_string(text)
    } else {
        text.to_string()
    }
}

/// Convert a primitive value to a TOON token
fn primitive_token(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("null".to_string()),
        Value::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
        Value::Number(n) => {
            let s = n.to_string();
            // Canonicalize -0
            if s == "-0" || s == "-0.0" {
                Some("0".to_string())
            } else {
                Some(s)
            }
        }
        Value::String(s) => Some(string_token(s, ',')),
        _ => None,
    }
}

/// Try to convert an array of objects to tabular format
fn tabularize(items: &[Value]) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    if items.is_empty() {
        return None;
    }

    // Get header keys from first object
    let first = items.first()?.as_object()?;
    if first.is_empty() {
        return None;
    }

    let header_keys: Vec<String> = first.keys().cloned().collect();
    let mut rows: Vec<Vec<String>> = Vec::new();

    for item in items {
        let obj = item.as_object()?;

        // Check all items have same keys (for strict mode)
        let mut row: Vec<String> = Vec::new();
        for key in &header_keys {
            let val = obj.get(key)?;
            let token = primitive_token(val)?;
            row.push(token);
        }
        rows.push(row);
    }

    Some((header_keys, rows))
}

/// Encode an array to TOON lines
fn encode_array(lines: &mut Vec<String>, indent: usize, key: &str, items: &[Value], anonymous: bool) {
    let indent_str = "  ".repeat(indent);
    let name = if anonymous {
        "items".to_string()
    } else {
        string_token(key, ',')
    };

    if items.is_empty() {
        lines.push(format!("{}{}[0]:", indent_str, name));
        return;
    }

    // Try tabular format
    if let Some((header_keys, rows)) = tabularize(items) {
        lines.push(format!(
            "{}{}[{}]{{{}}}:",
            indent_str,
            name,
            items.len(),
            header_keys.join(",")
        ));
        let row_indent = "  ".repeat(indent + 1);
        for row in rows {
            lines.push(format!("{}{}", row_indent, row.join(",")));
        }
        return;
    }

    // Try inline primitive array
    let encoded_prims: Option<Vec<String>> = items.iter().map(primitive_token).collect();
    if let Some(prims) = encoded_prims {
        lines.push(format!(
            "{}{}[{}]: {}",
            indent_str,
            name,
            items.len(),
            prims.join(",")
        ));
        return;
    }

    // Complex array with dash markers
    lines.push(format!("{}{}[{}]:", indent_str, name, items.len()));
    let item_indent = "  ".repeat(indent + 1);
    for item in items {
        match item {
            Value::Object(obj) => {
                lines.push(format!("{}-", item_indent));
                encode_object(lines, indent + 2, obj);
            }
            Value::Array(arr) => {
                lines.push(format!("{}-", item_indent));
                encode_array(lines, indent + 2, "items", arr, true);
            }
            _ => {
                let tok = primitive_token(item).unwrap_or_else(|| {
                    string_token(&item.to_string(), ',')
                });
                lines.push(format!("{}- {}", item_indent, tok));
            }
        }
    }
}

/// Encode an object to TOON lines
fn encode_object(lines: &mut Vec<String>, indent: usize, obj: &Map<String, Value>) {
    let mut keys: Vec<_> = obj.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = obj.get(key) {
            encode_value(lines, indent, key, value);
        }
    }
}

/// Encode a single key-value pair
fn encode_value(lines: &mut Vec<String>, indent: usize, key: &str, value: &Value) {
    let indent_str = "  ".repeat(indent);
    let key_tok = string_token(key, ',');

    match value {
        Value::Object(obj) => {
            lines.push(format!("{}{}:", indent_str, key_tok));
            if !obj.is_empty() {
                encode_object(lines, indent + 1, obj);
            }
        }
        Value::Array(arr) => {
            encode_array(lines, indent, key, arr, false);
        }
        _ => {
            let tok = primitive_token(value).unwrap_or_else(|| {
                string_token(&value.to_string(), ',')
            });
            lines.push(format!("{}{}: {}", indent_str, key_tok, tok));
        }
    }
}

// ============================================================================
// Parser (Decode)
// ============================================================================

/// Parse TOON format to JSON value
pub fn parse_toon(input: &str) -> Result<Value, ToonError> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Ok(Value::Null);
    }

    let mut parser = ToonParser::new(&lines);
    parser.parse_root()
}

struct ToonParser<'a> {
    lines: &'a [&'a str],
    pos: usize,
}

impl<'a> ToonParser<'a> {
    fn new(lines: &'a [&'a str]) -> Self {
        Self { lines, pos: 0 }
    }

    fn parse_root(&mut self) -> Result<Value, ToonError> {
        // Skip empty lines at start
        self.skip_empty_lines();

        if self.pos >= self.lines.len() {
            return Ok(Value::Null);
        }

        let first_line = self.lines[self.pos];

        // Check if it starts with "items[" (anonymous array)
        if first_line.trim_start().starts_with("items[") {
            return self.parse_value_at_indent(0);
        }

        // Otherwise parse as object
        self.parse_object_at_indent(0)
    }

    fn skip_empty_lines(&mut self) {
        while self.pos < self.lines.len() && self.lines[self.pos].trim().is_empty() {
            self.pos += 1;
        }
    }

    fn get_indent(line: &str) -> usize {
        let trimmed = line.trim_start_matches(' ');
        (line.len() - trimmed.len()) / 2
    }

    fn parse_object_at_indent(&mut self, min_indent: usize) -> Result<Value, ToonError> {
        let mut obj = Map::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            let indent = Self::get_indent(line);
            if indent < min_indent {
                break;
            }

            let trimmed = line.trim();

            // Parse key
            let (key, rest) = self.parse_key(trimmed)?;

            // Check for array notation: key[count]
            if let Some(arr_match) = self.parse_array_header(&key, rest) {
                let (actual_key, count, headers) = arr_match;
                let value = self.parse_array_content(indent, count, headers)?;
                obj.insert(actual_key, value);
            } else if rest.is_empty() || rest == ":" {
                // Nested object
                self.pos += 1;
                let value = self.parse_object_at_indent(indent + 1)?;
                obj.insert(key, value);
            } else if let Some(val_str) = rest.strip_prefix(": ") {
                // Simple value
                let value = self.parse_primitive(val_str)?;
                obj.insert(key, value);
                self.pos += 1;
            } else if rest == ":" {
                // Empty nested object
                obj.insert(key, Value::Object(Map::new()));
                self.pos += 1;
            } else {
                return Err(ToonError::ParseError {
                    line: self.pos + 1,
                    message: format!("Unexpected content after key: {}", rest),
                });
            }
        }

        Ok(Value::Object(obj))
    }

    fn parse_key<'b>(&self, line: &'b str) -> Result<(String, &'b str), ToonError> {
        if line.starts_with('"') {
            // Quoted key
            let end = self.find_quote_end(line, 1)?;
            let key = self.unescape_string(&line[1..end])?;
            let rest = &line[end + 1..];
            Ok((key, rest.trim_start()))
        } else {
            // Unquoted key - find the delimiter
            let delim_pos = line.find(|c| c == ':' || c == '[');
            match delim_pos {
                Some(pos) => {
                    let key = line[..pos].trim().to_string();
                    let rest = &line[pos..];
                    Ok((key, rest))
                }
                None => Ok((line.to_string(), "")),
            }
        }
    }

    fn parse_array_header(&self, key: &str, rest: &str) -> Option<(String, usize, Option<Vec<String>>)> {
        // Pattern: [count] or [count]{headers}:
        let full = format!("{}{}", key, rest);

        // Find [count]
        let bracket_start = full.find('[')?;
        let bracket_end = full.find(']')?;
        let count_str = &full[bracket_start + 1..bracket_end];
        let count: usize = count_str.parse().ok()?;
        let actual_key = full[..bracket_start].to_string();

        // Check for {headers}
        let after_bracket = &full[bracket_end + 1..];
        let headers = if after_bracket.starts_with('{') {
            let brace_end = after_bracket.find('}')?;
            let headers_str = &after_bracket[1..brace_end];
            Some(headers_str.split(',').map(|s| s.trim().to_string()).collect())
        } else {
            None
        };

        Some((actual_key, count, headers))
    }

    fn parse_array_content(
        &mut self,
        base_indent: usize,
        count: usize,
        headers: Option<Vec<String>>,
    ) -> Result<Value, ToonError> {
        let line = self.lines[self.pos];
        let trimmed = line.trim();

        // Check for inline array: key[count]: val1,val2,val3
        if let Some(colon_pos) = trimmed.rfind("]: ") {
            let values_str = &trimmed[colon_pos + 3..];
            let values = self.parse_inline_array(values_str)?;
            self.pos += 1;
            return Ok(Value::Array(values));
        }

        // Tabular or complex array
        self.pos += 1;

        if let Some(hdrs) = headers {
            // Tabular format
            self.parse_tabular_rows(base_indent, count, &hdrs)
        } else {
            // Complex array with dash markers
            self.parse_dash_array(base_indent, count)
        }
    }

    fn parse_inline_array(&self, values_str: &str) -> Result<Vec<Value>, ToonError> {
        if values_str.is_empty() {
            return Ok(Vec::new());
        }

        let mut values = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in values_str.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_quotes {
                current.push(ch);
                escape_next = true;
                continue;
            }

            if ch == '"' {
                in_quotes = !in_quotes;
                current.push(ch);
                continue;
            }

            if ch == ',' && !in_quotes {
                values.push(self.parse_primitive(current.trim())?);
                current.clear();
                continue;
            }

            current.push(ch);
        }

        if !current.is_empty() {
            values.push(self.parse_primitive(current.trim())?);
        }

        Ok(values)
    }

    fn parse_tabular_rows(
        &mut self,
        base_indent: usize,
        count: usize,
        headers: &[String],
    ) -> Result<Value, ToonError> {
        let mut rows = Vec::new();

        for _ in 0..count {
            self.skip_empty_lines();
            if self.pos >= self.lines.len() {
                break;
            }

            let line = self.lines[self.pos];
            let indent = Self::get_indent(line);
            if indent <= base_indent {
                break;
            }

            let values = self.parse_inline_array(line.trim())?;
            let mut obj = Map::new();
            for (i, header) in headers.iter().enumerate() {
                if let Some(val) = values.get(i) {
                    obj.insert(header.clone(), val.clone());
                }
            }
            rows.push(Value::Object(obj));
            self.pos += 1;
        }

        Ok(Value::Array(rows))
    }

    fn parse_dash_array(&mut self, base_indent: usize, _count: usize) -> Result<Value, ToonError> {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            self.skip_empty_lines();
            if self.pos >= self.lines.len() {
                break;
            }

            let line = self.lines[self.pos];
            let indent = Self::get_indent(line);
            if indent <= base_indent {
                break;
            }

            let trimmed = line.trim();
            if !trimmed.starts_with('-') {
                break;
            }

            let after_dash = trimmed[1..].trim();
            if after_dash.is_empty() {
                // Complex item (object or array on next lines)
                self.pos += 1;
                let value = self.parse_value_at_indent(indent + 1)?;
                items.push(value);
            } else {
                // Inline value after dash
                let value = self.parse_primitive(after_dash)?;
                items.push(value);
                self.pos += 1;
            }
        }

        Ok(Value::Array(items))
    }

    fn parse_value_at_indent(&mut self, min_indent: usize) -> Result<Value, ToonError> {
        self.skip_empty_lines();
        if self.pos >= self.lines.len() {
            return Ok(Value::Null);
        }

        let line = self.lines[self.pos];
        let trimmed = line.trim();

        // Check if it's an array
        if trimmed.contains('[') && trimmed.contains(']') {
            let (key, rest) = self.parse_key(trimmed)?;
            if let Some((_, count, headers)) = self.parse_array_header(&key, rest) {
                return self.parse_array_content(Self::get_indent(line), count, headers);
            }
        }

        // Otherwise parse as object
        self.parse_object_at_indent(min_indent)
    }

    fn parse_primitive(&self, s: &str) -> Result<Value, ToonError> {
        let trimmed = s.trim();

        if trimmed == "null" {
            return Ok(Value::Null);
        }
        if trimmed == "true" {
            return Ok(Value::Bool(true));
        }
        if trimmed == "false" {
            return Ok(Value::Bool(false));
        }

        // Quoted string
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let inner = &trimmed[1..trimmed.len() - 1];
            return Ok(Value::String(self.unescape_string(inner)?));
        }

        // Number
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(Value::Number(n.into()));
        }
        if let Ok(n) = trimmed.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(n) {
                return Ok(Value::Number(num));
            }
        }

        // Unquoted string
        Ok(Value::String(trimmed.to_string()))
    }

    fn find_quote_end(&self, s: &str, start: usize) -> Result<usize, ToonError> {
        let mut i = start;
        let chars: Vec<char> = s.chars().collect();
        while i < chars.len() {
            if chars[i] == '\\' {
                i += 2;
                continue;
            }
            if chars[i] == '"' {
                return Ok(i);
            }
            i += 1;
        }
        Err(ToonError::ParseError {
            line: self.pos + 1,
            message: "Unterminated string".to_string(),
        })
    }

    fn unescape_string(&self, s: &str) -> Result<String, ToonError> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('b') => result.push('\x08'),
                    Some('f') => result.push('\x0c'),
                    Some('u') => {
                        let hex: String = chars.by_ref().take(4).collect();
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(code) {
                                result.push(c);
                            }
                        }
                    }
                    Some(c) => result.push(c),
                    None => break,
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }
}

/// Validate TOON syntax
pub fn validate_toon(input: &str) -> Result<(), ToonError> {
    parse_toon(input)?;
    Ok(())
}

/// Convert JSON to TOON
pub fn json_to_toon(json: &Value) -> String {
    to_toon(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple_object() {
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "enabled": true
        });

        let toon = to_toon(&json);
        assert!(toon.contains("name: test"));
        assert!(toon.contains("count: 42"));
        assert!(toon.contains("enabled: true"));
    }

    #[test]
    fn test_encode_nested_object() {
        let json = serde_json::json!({
            "package": {
                "name": "my-app",
                "version": "1.0.0"
            }
        });

        let toon = to_toon(&json);
        assert!(toon.contains("package:"));
        assert!(toon.contains("  name: my-app"));
        assert!(toon.contains("  version: 1.0.0"));
    }

    #[test]
    fn test_encode_primitive_array() {
        let json = serde_json::json!({
            "tags": ["a", "b", "c"]
        });

        let toon = to_toon(&json);
        assert!(toon.contains("tags[3]: a,b,c"));
    }

    #[test]
    fn test_encode_tabular_array() {
        let json = serde_json::json!({
            "items": [
                {"name": "foo", "value": 1},
                {"name": "bar", "value": 2}
            ]
        });

        let toon = to_toon(&json);
        assert!(toon.contains("items[2]{"));
        assert!(toon.contains("name,value") || toon.contains("value,name"));
    }

    #[test]
    fn test_encode_quoted_string() {
        let json = serde_json::json!({
            "message": "hello world",
            "path": "/usr/bin"
        });

        let toon = to_toon(&json);
        // "hello world" needs quotes due to space
        assert!(toon.contains("message: \"hello world\""));
    }

    #[test]
    fn test_parse_simple_object() {
        let toon = "name: test\ncount: 42\nenabled: true";
        let value = parse_toon(toon).unwrap();

        assert_eq!(value["name"], "test");
        assert_eq!(value["count"], 42);
        assert_eq!(value["enabled"], true);
    }

    #[test]
    fn test_parse_nested_object() {
        let toon = "package:\n  name: my-app\n  version: 1.0.0";
        let value = parse_toon(toon).unwrap();

        assert_eq!(value["package"]["name"], "my-app");
        assert_eq!(value["package"]["version"], "1.0.0");
    }

    #[test]
    fn test_parse_inline_array() {
        let toon = "tags[3]: a,b,c";
        let value = parse_toon(toon).unwrap();

        let tags = value["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0], "a");
        assert_eq!(tags[1], "b");
        assert_eq!(tags[2], "c");
    }

    #[test]
    fn test_roundtrip() {
        let original = serde_json::json!({
            "name": "roundtrip",
            "count": 123,
            "nested": {
                "key": "value"
            },
            "tags": ["x", "y", "z"]
        });

        let toon = to_toon(&original);
        let parsed = parse_toon(&toon).unwrap();

        assert_eq!(parsed["name"], original["name"]);
        assert_eq!(parsed["count"], original["count"]);
        assert_eq!(parsed["nested"]["key"], original["nested"]["key"]);
    }

    #[test]
    fn test_special_chars_quoted() {
        let json = serde_json::json!({
            "path": "C:\\Users\\test",
            "query": "SELECT * FROM users"
        });

        let toon = to_toon(&json);
        // Should be properly escaped
        assert!(toon.contains("path:"));
        assert!(toon.contains("query:"));
    }

    #[test]
    fn test_null_values() {
        let json = serde_json::json!({
            "value": null
        });

        let toon = to_toon(&json);
        assert!(toon.contains("value: null"));

        let parsed = parse_toon(&toon).unwrap();
        assert!(parsed["value"].is_null());
    }
}
