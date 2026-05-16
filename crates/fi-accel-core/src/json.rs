//! SIMD-accelerated JSON parsing
//!
//! Uses simd-json for 5-20x faster JSON parsing compared to serde_json
//! for large payloads.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsonError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid JSON at position {position}: {message}")]
    InvalidJson { position: usize, message: String },
    #[error("Invalid JSON on line {line}: {message}")]
    InvalidNdjsonLine { line: usize, message: String },
}

/// A parsed JSON-RPC message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Parse newline-delimited JSON (NDJSON)
///
/// Returns a vector of parsed JSON values. Invalid lines are skipped.
#[cfg(feature = "simd")]
pub fn parse_ndjson(data: &[u8]) -> Vec<serde_json::Value> {
    data.split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let mut owned = line.to_vec();
            simd_json::to_borrowed_value(&mut owned)
                .ok()
                .map(|v| borrowed_to_owned(&v))
        })
        .collect()
}

/// Parse newline-delimited JSON (fallback without SIMD)
#[cfg(not(feature = "simd"))]
pub fn parse_ndjson(data: &[u8]) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(data)
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Convert simd_json borrowed value to serde_json owned value
#[cfg(feature = "simd")]
fn borrowed_to_owned(v: &simd_json::BorrowedValue) -> serde_json::Value {
    match v {
        simd_json::BorrowedValue::Static(s) => match s {
            simd_json::StaticNode::Null => serde_json::Value::Null,
            simd_json::StaticNode::Bool(b) => serde_json::Value::Bool(*b),
            simd_json::StaticNode::I64(n) => serde_json::Value::Number((*n).into()),
            simd_json::StaticNode::U64(n) => serde_json::Value::Number((*n).into()),
            simd_json::StaticNode::F64(n) => serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        },
        simd_json::BorrowedValue::String(s) => serde_json::Value::String(s.to_string()),
        simd_json::BorrowedValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(borrowed_to_owned).collect())
        }
        simd_json::BorrowedValue::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.to_string(), borrowed_to_owned(v)))
                .collect(),
        ),
    }
}

/// Parse a single JSON-RPC message
pub fn parse_jsonrpc(data: &[u8]) -> Result<JsonRpcMessage, JsonError> {
    serde_json::from_slice(data).map_err(|e| JsonError::ParseError(e.to_string()))
}

/// Parse multiple JSON-RPC messages from NDJSON
pub fn parse_jsonrpc_batch(data: &[u8]) -> Vec<Result<JsonRpcMessage, JsonError>> {
    String::from_utf8_lossy(data)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| JsonError::ParseError(e.to_string())))
        .collect()
}

/// Serialize to JSON string
pub fn to_json<T: Serialize>(value: &T) -> Result<String, JsonError> {
    serde_json::to_string(value).map_err(|e| JsonError::ParseError(e.to_string()))
}

/// Serialize to JSON bytes
pub fn to_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, JsonError> {
    serde_json::to_vec(value).map_err(|e| JsonError::ParseError(e.to_string()))
}

/// Serialize multiple values to NDJSON
pub fn to_ndjson<T: Serialize>(values: &[T]) -> Result<String, JsonError> {
    let lines: Result<Vec<String>, _> = values.iter().map(|v| to_json(v)).collect();
    Ok(lines?.join("\n"))
}

/// Strict incremental NDJSON reader.
///
/// Complete malformed lines return an error with a 1-based line number. A
/// trailing partial line stays buffered until a newline arrives.
#[derive(Debug, Clone)]
pub struct NDJSONReader {
    values: Vec<serde_json::Value>,
    buffer: Vec<u8>,
    next_line_number: usize,
}

impl NDJSONReader {
    /// Create a reader and process any complete lines in the initial content.
    pub fn new(initial_content: &[u8]) -> Result<Self, JsonError> {
        let mut reader = Self::empty();
        reader.append(initial_content)?;
        Ok(reader)
    }

    /// Create an empty reader.
    pub fn empty() -> Self {
        Self {
            values: Vec::new(),
            buffer: Vec::new(),
            next_line_number: 1,
        }
    }

    /// Append bytes and return newly parsed JSON values.
    pub fn append(&mut self, content: &[u8]) -> Result<Vec<serde_json::Value>, JsonError> {
        self.buffer.extend_from_slice(content);
        let mut parsed = Vec::new();

        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let mut line = self.buffer.drain(..=pos).collect::<Vec<_>>();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }

            let line_number = self.next_line_number;
            self.next_line_number += 1;

            let trimmed = trim_ascii_whitespace(&line);
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_slice::<serde_json::Value>(trimmed) {
                Ok(value) => {
                    self.values.push(value.clone());
                    parsed.push(value);
                }
                Err(err) => {
                    return Err(JsonError::InvalidNdjsonLine {
                        line: line_number,
                        message: err.to_string(),
                    });
                }
            }
        }

        Ok(parsed)
    }

    /// Return the last n parsed values.
    pub fn tail(&self, n: usize) -> Vec<serde_json::Value> {
        let start = self.values.len().saturating_sub(n);
        self.values[start..].to_vec()
    }

    /// Return all parsed values.
    pub fn all_values(&self) -> &[serde_json::Value] {
        &self.values
    }

    /// Return the number of parsed values.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check whether no values have been parsed.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return the number of bytes currently buffered as an incomplete line.
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }
}

fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ndjson() {
        let data = b"{\"a\": 1}\n{\"b\": 2}\n{\"c\": 3}";
        let values = parse_ndjson(data);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_parse_ndjson_with_empty_lines() {
        let data = b"{\"a\": 1}\n\n{\"b\": 2}\n";
        let values = parse_ndjson(data);
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_parse_ndjson_with_invalid() {
        let data = b"{\"a\": 1}\nnot json\n{\"b\": 2}";
        let values = parse_ndjson(data);
        assert_eq!(values.len(), 2); // Invalid line skipped
    }

    #[test]
    fn test_parse_jsonrpc_request() {
        let data = br#"{"jsonrpc": "2.0", "method": "test", "params": [1, 2], "id": 1}"#;
        let msg = parse_jsonrpc(data).unwrap();
        assert_eq!(msg.jsonrpc, "2.0");
        assert_eq!(msg.method, Some("test".to_string()));
        assert!(msg.id.is_some());
    }

    #[test]
    fn test_parse_jsonrpc_response() {
        let data = br#"{"jsonrpc": "2.0", "result": {"status": "ok"}, "id": 1}"#;
        let msg = parse_jsonrpc(data).unwrap();
        assert_eq!(msg.jsonrpc, "2.0");
        assert!(msg.result.is_some());
    }

    #[test]
    fn test_parse_jsonrpc_error() {
        let data = br#"{"jsonrpc": "2.0", "error": {"code": -32600, "message": "Invalid Request"}, "id": null}"#;
        let msg = parse_jsonrpc(data).unwrap();
        assert!(msg.error.is_some());
        assert_eq!(msg.error.unwrap().code, -32600);
    }

    #[test]
    fn test_parse_jsonrpc_batch() {
        let data = b"{\"jsonrpc\": \"2.0\", \"method\": \"a\", \"id\": 1}\n{\"jsonrpc\": \"2.0\", \"method\": \"b\", \"id\": 2}";
        let msgs = parse_jsonrpc_batch(data);
        assert_eq!(msgs.len(), 2);
        assert!(msgs.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_to_ndjson() {
        let values = vec![serde_json::json!({"a": 1}), serde_json::json!({"b": 2})];
        let ndjson = to_ndjson(&values).unwrap();
        assert!(ndjson.contains("{\"a\":1}"));
        assert!(ndjson.contains("{\"b\":2}"));
    }

    #[test]
    fn test_ndjson_reader_buffers_partial_line() {
        let mut reader = NDJSONReader::empty();
        assert!(reader.append(br#"{"a": 1}"#).unwrap().is_empty());
        assert_eq!(reader.buffered_len(), 8);

        let values = reader.append(b"\n").unwrap();
        assert_eq!(values, vec![serde_json::json!({"a": 1})]);
        assert_eq!(reader.len(), 1);
        assert_eq!(reader.buffered_len(), 0);
    }

    #[test]
    fn test_ndjson_reader_reports_line_number_and_recovers() {
        let mut reader = NDJSONReader::empty();
        let err = reader
            .append(
                br#"{"ok": 1}
not json
{"ok": 2}
"#,
            )
            .unwrap_err();

        assert!(err.to_string().contains("line 2"));
        assert_eq!(reader.len(), 1);

        let values = reader.append(b"").unwrap();
        assert_eq!(values, vec![serde_json::json!({"ok": 2})]);
        assert_eq!(reader.len(), 2);

        let values = reader.append(br#"{"ok": 3}"#.as_ref()).unwrap();
        assert!(values.is_empty());
        let values = reader.append(b"\n").unwrap();
        assert_eq!(values, vec![serde_json::json!({"ok": 3})]);
    }

    #[test]
    fn test_ndjson_reader_skips_blank_lines() {
        let reader = NDJSONReader::new(b"\n{\"a\":1}\n\n").unwrap();
        assert_eq!(reader.len(), 1);
        assert_eq!(reader.tail(1), vec![serde_json::json!({"a": 1})]);
    }
}
