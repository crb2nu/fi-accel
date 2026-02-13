//! FFI bindings for the tokenizer module

use std::ffi::c_char;

use crate::types::*;

/// Opaque tokenizer handle
pub struct FiTokenizer {
    inner: fi_accel_core::tokenizer::Tokenizer,
}

/// Create a new tokenizer for the given model name
///
/// Returns null on failure and writes error details to `err`.
/// The returned handle must be freed with `fi_tokenizer_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_tokenizer_new(
    model: *const c_char,
    err: *mut FiError,
) -> *mut FiTokenizer {
    let model_str = match cstr_to_str(model) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "model is null or invalid UTF-8");
            }
            return std::ptr::null_mut();
        }
    };

    match fi_accel_core::tokenizer::Tokenizer::new(model_str) {
        Ok(tok) => Box::into_raw(Box::new(FiTokenizer { inner: tok })),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            std::ptr::null_mut()
        }
    }
}

/// Free a tokenizer handle
#[no_mangle]
pub unsafe extern "C" fn fi_tokenizer_free(tok: *mut FiTokenizer) {
    if !tok.is_null() {
        drop(Box::from_raw(tok));
    }
}

/// Count tokens in a UTF-8 string
///
/// Returns the token count, or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn fi_tokenizer_count(
    tok: *const FiTokenizer,
    text: *const c_char,
    _text_len: usize,
    err: *mut FiError,
) -> usize {
    let tok = match tok.as_ref() {
        Some(t) => t,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "tokenizer handle is null");
            }
            return 0;
        }
    };

    let text_str = match cstr_to_str(text) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "text is null or invalid UTF-8");
            }
            return 0;
        }
    };

    tok.inner.count(text_str)
}

/// Encode text to token IDs
///
/// Returns a FiTokenArray that must be freed with `fi_token_array_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_tokenizer_encode(
    tok: *const FiTokenizer,
    text: *const c_char,
    err: *mut FiError,
) -> FiTokenArray {
    let tok = match tok.as_ref() {
        Some(t) => t,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "tokenizer handle is null");
            }
            return FiTokenArray::null();
        }
    };

    let text_str = match cstr_to_str(text) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "text is null or invalid UTF-8");
            }
            return FiTokenArray::null();
        }
    };

    let tokens = tok.inner.encode(text_str);
    FiTokenArray::from_vec(tokens)
}

/// Decode token IDs back to text
///
/// Returns a FiBuffer that must be freed with `fi_buffer_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_tokenizer_decode(
    tok: *const FiTokenizer,
    tokens: *const u32,
    tokens_len: usize,
    err: *mut FiError,
) -> FiBuffer {
    let tok = match tok.as_ref() {
        Some(t) => t,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "tokenizer handle is null");
            }
            return FiBuffer::null();
        }
    };

    if tokens.is_null() || tokens_len == 0 {
        return FiBuffer::from_string(String::new());
    }

    let token_slice = std::slice::from_raw_parts(tokens, tokens_len);

    match tok.inner.decode(token_slice) {
        Ok(text) => FiBuffer::from_string(text),
        Err(e) => {
            if let Some(err) = err.as_mut() {
                err.set(FiErrorCode::Failed, &e.to_string());
            }
            FiBuffer::null()
        }
    }
}

/// Truncate text to fit within max_tokens
///
/// Returns a FiBuffer that must be freed with `fi_buffer_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_tokenizer_truncate(
    tok: *const FiTokenizer,
    text: *const c_char,
    max_tokens: usize,
    err: *mut FiError,
) -> FiBuffer {
    let tok = match tok.as_ref() {
        Some(t) => t,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "tokenizer handle is null");
            }
            return FiBuffer::null();
        }
    };

    let text_str = match cstr_to_str(text) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "text is null or invalid UTF-8");
            }
            return FiBuffer::null();
        }
    };

    let truncated = tok.inner.truncate(text_str, max_tokens);
    FiBuffer::from_string(truncated)
}
