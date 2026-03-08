//! FFI bindings for chunking helpers.

use std::ffi::{c_char, CStr};

use crate::types::*;
use fi_accel_core::chunker::{self, ChunkConfig, ContentType, Language};

fn optional_string_arg(
    ptr: *const c_char,
    field_name: &str,
    err: *mut FiError,
) -> Option<Option<String>> {
    if ptr.is_null() {
        return Some(None);
    }

    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(value) => Some(Some(value.to_string())),
        Err(_) => {
            if let Some(err) = unsafe { err.as_mut() } {
                err.set(
                    FiErrorCode::InvalidArg,
                    &format!("{field_name} is not valid UTF-8"),
                );
            }
            None
        }
    }
}

fn required_str_arg<'a>(
    ptr: *const c_char,
    field_name: &str,
    err: *mut FiError,
) -> Option<&'a str> {
    match unsafe { cstr_to_str(ptr) } {
        Some(value) => Some(value),
        None => {
            if let Some(err) = unsafe { err.as_mut() } {
                err.set(
                    FiErrorCode::InvalidArg,
                    &format!("{field_name} is null or invalid UTF-8"),
                );
            }
            None
        }
    }
}

fn chunk_config(
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: Option<String>,
    model: Option<String>,
) -> ChunkConfig {
    ChunkConfig {
        max_tokens: if max_tokens == 0 { 512 } else { max_tokens },
        overlap_tokens,
        file_path,
        model,
    }
}

fn serialize_chunks(chunks: &[chunker::Chunk], err: *mut FiError) -> FiBuffer {
    match serde_json::to_string(chunks) {
        Ok(json) => FiBuffer::from_string(json),
        Err(e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                err.set(FiErrorCode::Failed, &format!("serialization failed: {}", e));
            }
            FiBuffer::null()
        }
    }
}

fn content_type_name(content_type: ContentType) -> String {
    match content_type {
        ContentType::Code(lang) => format!("code:{}", lang.as_str()),
        ContentType::Markdown => "markdown".to_string(),
        ContentType::PlainText => "text".to_string(),
    }
}

/// Chunk source code using the Rust tree-sitter chunker.
#[no_mangle]
pub unsafe extern "C" fn fi_chunk_code(
    content: *const c_char,
    language: *const c_char,
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: *const c_char,
    model: *const c_char,
    err: *mut FiError,
) -> FiBuffer {
    let Some(content) = required_str_arg(content, "content", err) else {
        return FiBuffer::null();
    };
    let Some(language) = required_str_arg(language, "language", err) else {
        return FiBuffer::null();
    };
    let Some(file_path) = optional_string_arg(file_path, "file_path", err) else {
        return FiBuffer::null();
    };
    let Some(model) = optional_string_arg(model, "model", err) else {
        return FiBuffer::null();
    };

    let Some(language) = Language::from_str(language) else {
        if let Some(err) = err.as_mut() {
            err.set(
                FiErrorCode::InvalidArg,
                &format!("unsupported language: {}", language),
            );
        }
        return FiBuffer::null();
    };

    match chunker::chunk_code(
        content,
        &language,
        &chunk_config(max_tokens, overlap_tokens, file_path, model),
    ) {
        Ok(chunks) => serialize_chunks(&chunks, err),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Chunk markdown content.
#[no_mangle]
pub unsafe extern "C" fn fi_chunk_markdown(
    content: *const c_char,
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: *const c_char,
    model: *const c_char,
    err: *mut FiError,
) -> FiBuffer {
    let Some(content) = required_str_arg(content, "content", err) else {
        return FiBuffer::null();
    };
    let Some(file_path) = optional_string_arg(file_path, "file_path", err) else {
        return FiBuffer::null();
    };
    let Some(model) = optional_string_arg(model, "model", err) else {
        return FiBuffer::null();
    };

    match chunker::chunk_markdown(
        content,
        &chunk_config(max_tokens, overlap_tokens, file_path, model),
    ) {
        Ok(chunks) => serialize_chunks(&chunks, err),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Chunk plain text content.
#[no_mangle]
pub unsafe extern "C" fn fi_chunk_text(
    content: *const c_char,
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: *const c_char,
    model: *const c_char,
    err: *mut FiError,
) -> FiBuffer {
    let Some(content) = required_str_arg(content, "content", err) else {
        return FiBuffer::null();
    };
    let Some(file_path) = optional_string_arg(file_path, "file_path", err) else {
        return FiBuffer::null();
    };
    let Some(model) = optional_string_arg(model, "model", err) else {
        return FiBuffer::null();
    };

    let chunks = chunker::chunk_text(
        content,
        &chunk_config(max_tokens, overlap_tokens, file_path, model),
    );
    serialize_chunks(&chunks, err)
}

/// Detect content type from file path and chunk accordingly.
#[no_mangle]
pub unsafe extern "C" fn fi_chunk_auto(
    content: *const c_char,
    file_path: *const c_char,
    max_tokens: usize,
    overlap_tokens: usize,
    model: *const c_char,
    err: *mut FiError,
) -> FiBuffer {
    let Some(content) = required_str_arg(content, "content", err) else {
        return FiBuffer::null();
    };
    let Some(file_path_str) = required_str_arg(file_path, "file_path", err) else {
        return FiBuffer::null();
    };
    let Some(model) = optional_string_arg(model, "model", err) else {
        return FiBuffer::null();
    };
    let file_path_owned = Some(file_path_str.to_string());

    match chunker::chunk(
        content,
        chunker::detect_content_type(file_path_str),
        &chunk_config(max_tokens, overlap_tokens, file_path_owned, model),
    ) {
        Ok(chunks) => serialize_chunks(&chunks, err),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Detect content type from file path.
#[no_mangle]
pub unsafe extern "C" fn fi_detect_content_type(
    file_path: *const c_char,
    err: *mut FiError,
) -> FiBuffer {
    let Some(file_path) = required_str_arg(file_path, "file_path", err) else {
        return FiBuffer::null();
    };

    FiBuffer::from_string(content_type_name(chunker::detect_content_type(file_path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer_to_string(buf: FiBuffer) -> String {
        let bytes = unsafe { std::slice::from_raw_parts(buf.data, buf.len) }.to_vec();
        unsafe { fi_buffer_free(buf) };
        String::from_utf8(bytes).unwrap()
    }

    fn empty_error() -> FiError {
        FiError {
            code: FiErrorCode::Ok,
            message: [0; 256],
            message_len: 0,
        }
    }

    #[test]
    fn test_chunk_code_round_trip() {
        let content = std::ffi::CString::new("def hello(name):\n    return name\n").unwrap();
        let language = std::ffi::CString::new("python").unwrap();
        let mut err = empty_error();

        let buf = unsafe {
            fi_chunk_code(
                content.as_ptr(),
                language.as_ptr(),
                512,
                50,
                std::ptr::null(),
                std::ptr::null(),
                &mut err,
            )
        };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"name\":\"hello\""));
    }

    #[test]
    fn test_chunk_markdown_round_trip() {
        let content = std::ffi::CString::new("# Title\n\nBody\n").unwrap();
        let mut err = empty_error();
        let buf =
            unsafe { fi_chunk_markdown(content.as_ptr(), 512, 50, std::ptr::null(), std::ptr::null(), &mut err) };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"symbol_kind\":\"section\""));
    }

    #[test]
    fn test_chunk_auto_round_trip() {
        let content = std::ffi::CString::new("line one\n\nline two\n").unwrap();
        let file_path = std::ffi::CString::new("notes.txt").unwrap();
        let mut err = empty_error();
        let buf = unsafe {
            fi_chunk_auto(
                content.as_ptr(),
                file_path.as_ptr(),
                512,
                50,
                std::ptr::null(),
                &mut err,
            )
        };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert!(output.contains("\"symbol_kind\":\"paragraph\""));
    }

    #[test]
    fn test_detect_content_type_round_trip() {
        let file_path = std::ffi::CString::new("README.md").unwrap();
        let mut err = empty_error();
        let buf = unsafe { fi_detect_content_type(file_path.as_ptr(), &mut err) };
        assert!(!buf.data.is_null());
        let output = buffer_to_string(buf);
        assert_eq!(output, "markdown");
    }
}
