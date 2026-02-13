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
            simd_json::StaticNode::F64(n) => {
                serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
        },
        simd_json::BorrowedValue::String(s) => serde_json::Value::String(s.to_string()),
        simd_json::BorrowedValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(borrowed_to_owned).collect())
        }
        simd_json::BorrowedValue::Object(obj) => {
            serde_json::Value::Object(
                obj.iter()
                    .map(|(k, v)| (k.to_string(), borrowed_to_owned(v)))
                    .collect(),
            )
        }
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
        let values = vec![
            serde_json::json!({"a": 1}),
            serde_json::json!({"b": 2}),
        ];
        let ndjson = to_ndjson(&values).unwrap();
        assert!(ndjson.contains("{\"a\":1}"));
        assert!(ndjson.contains("{\"b\":2}"));
    }
}
