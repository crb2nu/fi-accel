//! FFI bindings for the diff/similarity module

use std::ffi::c_char;

use crate::types::*;

/// Compute string similarity between two UTF-8 strings using Myers diff.
///
/// Returns a value in [0.0, 1.0] where 1.0 means identical.
/// Returns 0.0 and sets error on invalid input.
#[no_mangle]
pub unsafe extern "C" fn fi_string_similarity(
    a: *const c_char,
    b: *const c_char,
    err: *mut FiError,
) -> f32 {
    let a_str = match cstr_to_str(a) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "string a is null or invalid UTF-8");
            }
            return 0.0;
        }
    };

    let b_str = match cstr_to_str(b) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "string b is null or invalid UTF-8");
            }
            return 0.0;
        }
    };

    let matcher = fi_accel_core::diff::FuzzyMatcher::default();
    matcher.similarity(a_str, b_str)
}

/// Compute line-level similarity between two multiline UTF-8 strings.
///
/// Each string is treated as a sequence of lines (split by newline).
/// Returns a value in [0.0, 1.0] where 1.0 means identical.
#[no_mangle]
pub unsafe extern "C" fn fi_line_similarity(
    a: *const c_char,
    b: *const c_char,
    err: *mut FiError,
) -> f32 {
    let a_str = match cstr_to_str(a) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "string a is null or invalid UTF-8");
            }
            return 0.0;
        }
    };

    let b_str = match cstr_to_str(b) {
        Some(s) => s,
        None => {
            if let Some(e) = err.as_mut() {
                e.set(FiErrorCode::InvalidArg, "string b is null or invalid UTF-8");
            }
            return 0.0;
        }
    };

    let a_lines: Vec<&str> = a_str.lines().collect();
    let b_lines: Vec<&str> = b_str.lines().collect();

    let matcher = fi_accel_core::diff::FuzzyMatcher::default();
    matcher.line_similarity(&a_lines, &b_lines)
}
