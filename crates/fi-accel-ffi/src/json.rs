//! FFI bindings for the JSON module

use crate::types::*;

/// Parse newline-delimited JSON (NDJSON) data
///
/// - `data`: raw bytes containing NDJSON
/// - `data_len`: length of data in bytes
///
/// Returns a FiBuffer containing the parsed JSON as a JSON array string.
/// The caller should parse the returned string as a JSON array.
/// Must be freed with `fi_buffer_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_parse_ndjson(
    data: *const u8,
    data_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    if data.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "data pointer is null");
        }
        return FiBuffer::null();
    }

    let slice = std::slice::from_raw_parts(data, data_len);
    let values = fi_accel_core::json::parse_ndjson(slice);

    match serde_json::to_string(&values) {
        Ok(json_str) => FiBuffer::from_string(json_str),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &format!("serialization failed: {}", e));
            }
            FiBuffer::null()
        }
    }
}

/// Serialize values to NDJSON format
///
/// - `json_array`: a JSON array string to convert to NDJSON
/// - `json_array_len`: length in bytes
///
/// Returns a FiBuffer containing newline-delimited JSON.
/// Must be freed with `fi_buffer_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_to_ndjson(
    json_array: *const u8,
    json_array_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    if json_array.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "data pointer is null");
        }
        return FiBuffer::null();
    }

    let slice = std::slice::from_raw_parts(json_array, json_array_len);
    let str_data = match std::str::from_utf8(slice) {
        Ok(s) => s,
        Err(_) => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "invalid UTF-8");
            }
            return FiBuffer::null();
        }
    };

    let values: Vec<serde_json::Value> = match serde_json::from_str(str_data) {
        Ok(v) => v,
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &format!("parse failed: {}", e));
            }
            return FiBuffer::null();
        }
    };

    match fi_accel_core::json::to_ndjson(&values) {
        Ok(ndjson) => FiBuffer::from_string(ndjson),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &format!("serialization failed: {}", e));
            }
            FiBuffer::null()
        }
    }
}
