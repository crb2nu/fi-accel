//! FFI bindings for generic event-log helpers.

use crate::types::*;
use fi_accel_core::eventlog::EventProjectionConfig;

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

unsafe fn parse_config(
    config_json: *const u8,
    config_json_len: usize,
    err: *mut FiError,
) -> Option<EventProjectionConfig> {
    if config_json.is_null() || config_json_len == 0 {
        return Some(EventProjectionConfig::default());
    }

    let slice = std::slice::from_raw_parts(config_json, config_json_len);
    let text = match std::str::from_utf8(slice) {
        Ok(value) => value,
        Err(_) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::InvalidArg, "config JSON is not valid UTF-8");
            }
            return None;
        }
    };

    match serde_json::from_str(text) {
        Ok(config) => Some(config),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(
                    FiErrorCode::InvalidArg,
                    &format!("config parse failed: {}", e),
                );
            }
            None
        }
    }
}

/// Project NDJSON/JSONL event records into a normalized timeline shape.
#[no_mangle]
pub unsafe extern "C" fn fi_project_eventlog(
    data: *const u8,
    data_len: usize,
    config_json: *const u8,
    config_json_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let Some(slice) = read_input(data, data_len, err) else {
        return FiBuffer::null();
    };
    let Some(config) = parse_config(config_json, config_json_len, err) else {
        return FiBuffer::null();
    };

    let events = fi_accel_core::eventlog::project_eventlog_ndjson(slice, &config);
    serialize_to_buffer(&events, err)
}

/// Summarize NDJSON/JSONL event records by event and actor dimensions.
#[no_mangle]
pub unsafe extern "C" fn fi_summarize_eventlog(
    data: *const u8,
    data_len: usize,
    config_json: *const u8,
    config_json_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let Some(slice) = read_input(data, data_len, err) else {
        return FiBuffer::null();
    };
    let Some(config) = parse_config(config_json, config_json_len, err) else {
        return FiBuffer::null();
    };

    let summary = fi_accel_core::eventlog::summarize_eventlog_ndjson(slice, &config);
    serialize_to_buffer(&summary, err)
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
    fn test_project_eventlog_round_trip() {
        let data =
            br#"{"timestamp":"2026-03-18T10:00:00Z","event_type":"started","agent_id":"codex"}"#;
        let config = br#"{"timestamp_field":"timestamp","event_type_field":"event_type","actor_id_field":"agent_id","actor_type_field":null,"data_field":null,"limit":null}"#;
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe {
            fi_project_eventlog(
                data.as_ptr(),
                data.len(),
                config.as_ptr(),
                config.len(),
                &mut err,
            )
        };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"event_type\":\"started\""));
        assert!(output.contains("\"actor_id\":\"codex\""));
    }

    #[test]
    fn test_summarize_eventlog_round_trip() {
        let data = br#"{"timestamp":"2026-03-18T10:00:00Z","status":"success","agent_id":"codex"}
{"timestamp":"2026-03-18T10:01:00Z","status":"success","agent_id":"codex"}"#;
        let config = br#"{"timestamp_field":"timestamp","event_type_field":"status","actor_id_field":"agent_id","actor_type_field":null,"data_field":null,"limit":null}"#;
        let mut err = FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        };

        let buf = unsafe {
            fi_summarize_eventlog(
                data.as_ptr(),
                data.len(),
                config.as_ptr(),
                config.len(),
                &mut err,
            )
        };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"total_events\":2"));
        assert!(output.contains("\"success\":2"));
    }
}
