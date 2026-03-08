//! FFI bindings for transport helpers.

use crate::types::*;
use fi_accel_core::json::JsonRpcMessage;

unsafe fn read_input<'a>(data: *const u8, data_len: usize, err: *mut FiError) -> Option<&'a [u8]> {
    if data.is_null() {
        if let Some(err) = err.as_mut() {
            err.set(FiErrorCode::InvalidArg, "data pointer is null");
        }
        return None;
    }

    Some(std::slice::from_raw_parts(data, data_len))
}

fn serialize_to_buffer<T: serde::Serialize>(value: &T, err: *mut FiError) -> FiBuffer {
    match serde_json::to_string(value) {
        Ok(json) => FiBuffer::from_string(json),
        Err(e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                err.set(FiErrorCode::Failed, &format!("serialization failed: {}", e));
            }
            FiBuffer::null()
        }
    }
}

/// Parse newline-delimited JSON-RPC messages.
#[no_mangle]
pub unsafe extern "C" fn fi_parse_jsonrpc_lines(
    data: *const u8,
    data_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let Some(slice) = read_input(data, data_len, err) else {
        return FiBuffer::null();
    };

    match fi_accel_core::transport::parse_jsonrpc_lines(slice) {
        Ok(messages) => serialize_to_buffer(&messages, err),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Serialize JSON-RPC messages from a JSON array into line-delimited framing.
#[no_mangle]
pub unsafe extern "C" fn fi_serialize_jsonrpc_lines(
    json_array: *const u8,
    json_array_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let Some(slice) = read_input(json_array, json_array_len, err) else {
        return FiBuffer::null();
    };

    let str_data = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::InvalidArg, "invalid UTF-8");
            }
            return FiBuffer::null();
        }
    };

    let messages: Vec<JsonRpcMessage> = match serde_json::from_str(str_data) {
        Ok(v) => v,
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &format!("parse failed: {}", e));
            }
            return FiBuffer::null();
        }
    };

    match fi_accel_core::transport::serialize_jsonrpc_lines(&messages) {
        Ok(output) => match String::from_utf8(output) {
            Ok(output) => FiBuffer::from_string(output),
            Err(e) => {
                if let Some(err) = err.as_mut() {
                    err.set(FiErrorCode::Failed, &format!("serialization failed: {}", e));
                }
                FiBuffer::null()
            }
        },
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Parse Server-Sent Events into a JSON array of transport events.
#[no_mangle]
pub unsafe extern "C" fn fi_parse_sse_events(
    data: *const u8,
    data_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let Some(slice) = read_input(data, data_len, err) else {
        return FiBuffer::null();
    };

    match fi_accel_core::transport::parse_sse_events(slice) {
        Ok(events) => serialize_to_buffer(&events, err),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Extract MCP Streamable HTTP metadata from a raw header block.
#[no_mangle]
pub unsafe extern "C" fn fi_extract_mcp_http_meta(
    headers: *const u8,
    headers_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let Some(slice) = read_input(headers, headers_len, err) else {
        return FiBuffer::null();
    };

    match fi_accel_core::transport::extract_mcp_http_meta(slice) {
        Ok(meta) => serialize_to_buffer(&meta, err),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer_to_string(buf: FiBuffer) -> String {
        let bytes = unsafe { std::slice::from_raw_parts(buf.data, buf.len) }.to_vec();
        unsafe { fi_buffer_free(buf) };
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_parse_jsonrpc_lines_round_trip() {
        let data = b"{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n";
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe { fi_parse_jsonrpc_lines(data.as_ptr(), data.len(), &mut err) };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"method\":\"ping\""));
    }

    #[test]
    fn test_serialize_jsonrpc_lines_round_trip() {
        let data = br#"[{"jsonrpc":"2.0","method":"ping","id":1}]"#;
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe { fi_serialize_jsonrpc_lines(data.as_ptr(), data.len(), &mut err) };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert_eq!(output.trim(), r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
    }

    #[test]
    fn test_parse_sse_events_round_trip() {
        let data = b"id: 1\ndata: hello\n\n";
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe { fi_parse_sse_events(data.as_ptr(), data.len(), &mut err) };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"data\":\"hello\""));
    }

    #[test]
    fn test_extract_mcp_http_meta_round_trip() {
        let data = b"Mcp-Session-Id: abc\r\nAccept: text/event-stream\r\n";
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe { fi_extract_mcp_http_meta(data.as_ptr(), data.len(), &mut err) };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"session_id\":\"abc\""));
        assert!(output.contains("\"accepts_sse\":true"));
    }

    #[test]
    fn test_parse_sse_events_error_propagation() {
        let data = b"id: bad\0id\n\n";
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe { fi_parse_sse_events(data.as_ptr(), data.len(), &mut err) };
        assert!(buf.data.is_null());
        assert!(!matches!(err.code, FiErrorCode::Ok));
    }
}
