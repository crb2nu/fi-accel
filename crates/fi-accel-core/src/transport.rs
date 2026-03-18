use crate::json::{self, JsonRpcMessage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    #[error("invalid UTF-8: {0}")]
    InvalidUtf8(String),
    #[error("invalid JSON-RPC line {line}: {message}")]
    InvalidJsonRpcLine { line: usize, message: String },
    #[error("invalid header line {line}: {message}")]
    InvalidHeaderLine { line: usize, message: String },
    #[error("invalid SSE line {line}: {message}")]
    InvalidSseLine { line: usize, message: String },
    #[error("serialization failed: {0}")]
    SerializationFailed(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpHttpMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<String>,
    pub accepts_sse: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SseEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub data: String,
}

#[derive(Default)]
struct SseEventBuilder {
    id: Option<String>,
    event: Option<String>,
    data_lines: Vec<String>,
}

impl SseEventBuilder {
    fn is_empty(&self) -> bool {
        self.id.is_none() && self.event.is_none() && self.data_lines.is_empty()
    }

    fn finish(&mut self) -> Option<SseEvent> {
        if self.is_empty() {
            return None;
        }

        let event = SseEvent {
            id: self.id.take(),
            event: self.event.take(),
            data: self.data_lines.join("\n"),
        };
        self.data_lines.clear();
        Some(event)
    }
}

/// Parse line-delimited JSON-RPC messages used by stdio transports.
pub fn parse_jsonrpc_lines(data: &[u8]) -> Result<Vec<JsonRpcMessage>, TransportError> {
    let text = std::str::from_utf8(data).map_err(|e| TransportError::InvalidUtf8(e.to_string()))?;
    let mut messages = Vec::new();

    for (idx, raw_line) in text.split('\n').enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let message = json::parse_jsonrpc(line.as_bytes()).map_err(|err| {
            TransportError::InvalidJsonRpcLine {
                line: idx + 1,
                message: err.to_string(),
            }
        })?;
        messages.push(message);
    }

    Ok(messages)
}

/// Serialize JSON-RPC messages into newline-delimited framing for stdio transports.
pub fn serialize_jsonrpc_lines(messages: &[JsonRpcMessage]) -> Result<Vec<u8>, TransportError> {
    json::to_ndjson(messages)
        .map(|value| value.into_bytes())
        .map_err(|err| TransportError::SerializationFailed(err.to_string()))
}

/// Extract MCP Streamable HTTP metadata from a raw header block.
pub fn extract_mcp_http_meta(headers: &[u8]) -> Result<McpHttpMeta, TransportError> {
    let text =
        std::str::from_utf8(headers).map_err(|e| TransportError::InvalidUtf8(e.to_string()))?;
    let mut meta = McpHttpMeta::default();

    for (idx, raw_line) in text.split('\n').enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let (name, value) =
            line.split_once(':')
                .ok_or_else(|| TransportError::InvalidHeaderLine {
                    line: idx + 1,
                    message: "missing ':' separator".to_string(),
                })?;
        let name = name.trim();
        if name.is_empty() {
            return Err(TransportError::InvalidHeaderLine {
                line: idx + 1,
                message: "header name is empty".to_string(),
            });
        }
        let value = value.trim();

        if name.eq_ignore_ascii_case("mcp-session-id") {
            meta.session_id = Some(value.to_string());
        } else if name.eq_ignore_ascii_case("mcp-protocol-version") {
            meta.protocol_version = Some(value.to_string());
        } else if name.eq_ignore_ascii_case("last-event-id") {
            meta.last_event_id = Some(value.to_string());
        } else if name.eq_ignore_ascii_case("accept") {
            meta.accepts_sse |= value
                .split(',')
                .filter_map(|part| part.trim().split(';').next())
                .any(|mime| mime.eq_ignore_ascii_case("text/event-stream"));
        }
    }

    Ok(meta)
}

/// Parse Server-Sent Events from an MCP-style event stream.
pub fn parse_sse_events(data: &[u8]) -> Result<Vec<SseEvent>, TransportError> {
    let text = std::str::from_utf8(data).map_err(|e| TransportError::InvalidUtf8(e.to_string()))?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut events = Vec::new();
    let mut current = SseEventBuilder::default();

    for (idx, line) in normalized.split('\n').enumerate() {
        if line.is_empty() {
            if let Some(event) = current.finish() {
                events.push(event);
            }
            continue;
        }

        if line.starts_with(':') {
            continue;
        }

        let (field, raw_value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line, ""),
        };

        match field {
            "event" => current.event = Some(raw_value.to_string()),
            "data" => current.data_lines.push(raw_value.to_string()),
            "id" => {
                if raw_value.contains('\0') {
                    return Err(TransportError::InvalidSseLine {
                        line: idx + 1,
                        message: "event id contains NUL".to_string(),
                    });
                }
                current.id = Some(raw_value.to_string());
            }
            "retry" => {}
            _ if field.contains(char::is_whitespace) => {
                return Err(TransportError::InvalidSseLine {
                    line: idx + 1,
                    message: "field name contains whitespace".to_string(),
                });
            }
            _ => {}
        }
    }

    if let Some(event) = current.finish() {
        events.push(event);
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_jsonrpc_lines_single() {
        let input = br#"{"jsonrpc":"2.0","method":"ping","id":1}"#;
        let messages = parse_jsonrpc_lines(input).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].method.as_deref(), Some("ping"));
    }

    #[test]
    fn test_parse_jsonrpc_lines_multiple_with_blank_lines() {
        let input = b"{\"jsonrpc\":\"2.0\",\"method\":\"a\",\"id\":1}\n\n{\"jsonrpc\":\"2.0\",\"method\":\"b\",\"id\":2}\n";
        let messages = parse_jsonrpc_lines(input).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].method.as_deref(), Some("b"));
    }

    #[test]
    fn test_parse_jsonrpc_lines_invalid() {
        let input = b"{\"jsonrpc\":\"2.0\",\"method\":\"a\",\"id\":1}\nnot-json";
        let err = parse_jsonrpc_lines(input).unwrap_err();
        assert!(matches!(
            err,
            TransportError::InvalidJsonRpcLine { line: 2, .. }
        ));
    }

    #[test]
    fn test_parse_jsonrpc_lines_embedded_newline_is_invalid() {
        let input = b"{\"jsonrpc\":\"2.0\",\"method\":\"a\nb\",\"id\":1}";
        assert!(matches!(
            parse_jsonrpc_lines(input),
            Err(TransportError::InvalidJsonRpcLine { .. })
        ));
    }

    #[test]
    fn test_serialize_jsonrpc_lines() {
        let messages = vec![
            JsonRpcMessage {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: Some("ping".to_string()),
                params: None,
                result: None,
                error: None,
            },
            JsonRpcMessage {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(2)),
                method: None,
                params: None,
                result: Some(json!({"ok": true})),
                error: None,
            },
        ];

        let out = serialize_jsonrpc_lines(&messages).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 2);
    }

    #[test]
    fn test_extract_mcp_http_meta() {
        let headers = b"Mcp-Session-Id: abc123\r\nMCP-Protocol-Version: 2025-11-05\r\nLast-Event-ID: 42\r\nAccept: application/json, text/event-stream\r\n";
        let meta = extract_mcp_http_meta(headers).unwrap();
        assert_eq!(meta.session_id.as_deref(), Some("abc123"));
        assert_eq!(meta.protocol_version.as_deref(), Some("2025-11-05"));
        assert_eq!(meta.last_event_id.as_deref(), Some("42"));
        assert!(meta.accepts_sse);
    }

    #[test]
    fn test_extract_mcp_http_meta_without_sse_accept() {
        let headers = b"Accept: application/json\r\n";
        let meta = extract_mcp_http_meta(headers).unwrap();
        assert!(!meta.accepts_sse);
    }

    #[test]
    fn test_parse_sse_single_event() {
        let input = b"event: message\ndata: hello\n\n";
        let events = parse_sse_events(input).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("message"));
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn test_parse_sse_multi_event() {
        let input = b"id: 1\ndata: one\n\nid: 2\ndata: two\n\n";
        let events = parse_sse_events(input).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id.as_deref(), Some("1"));
        assert_eq!(events[1].data, "two");
    }

    #[test]
    fn test_parse_sse_event_without_name() {
        let input = b"data: hello\n\n";
        let events = parse_sse_events(input).unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].event.is_none());
    }

    #[test]
    fn test_parse_sse_multiline_data() {
        let input = b"data: one\ndata: two\n\n";
        let events = parse_sse_events(input).unwrap();
        assert_eq!(events[0].data, "one\ntwo");
    }

    #[test]
    fn test_parse_sse_malformed_payload() {
        let input = b"id: bad\0id\n\n";
        assert!(matches!(
            parse_sse_events(input),
            Err(TransportError::InvalidSseLine { .. })
        ));
    }
}
