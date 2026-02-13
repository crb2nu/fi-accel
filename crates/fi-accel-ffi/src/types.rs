//! Common FFI types shared across modules

use std::ffi::{c_char, CStr};
use std::ptr;

/// Error codes returned by FFI functions
#[repr(C)]
pub enum FiErrorCode {
    /// No error
    Ok = 0,
    /// Invalid argument (null pointer, invalid UTF-8, etc.)
    InvalidArg = 1,
    /// Operation failed
    Failed = 2,
    /// Resource not found
    NotFound = 3,
    /// Out of memory
    OutOfMemory = 4,
}

/// Error information written by FFI functions
#[repr(C)]
pub struct FiError {
    pub code: FiErrorCode,
    pub message: [u8; 256],
    pub message_len: u32,
}

impl FiError {
    pub fn set(&mut self, code: FiErrorCode, msg: &str) {
        self.code = code;
        let bytes = msg.as_bytes();
        let len = bytes.len().min(255);
        self.message[..len].copy_from_slice(&bytes[..len]);
        self.message_len = len as u32;
    }
}

/// Rust-owned byte buffer returned to the caller
///
/// Must be freed with `fi_buffer_free`.
#[repr(C)]
pub struct FiBuffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl FiBuffer {
    /// Create a FiBuffer from a Rust String
    pub fn from_string(s: String) -> Self {
        let mut bytes = s.into_bytes();
        let buf = FiBuffer {
            data: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        };
        std::mem::forget(bytes);
        buf
    }

    /// Create a null/empty FiBuffer
    pub fn null() -> Self {
        FiBuffer {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

/// Array of u32 token IDs returned to the caller
///
/// Must be freed with `fi_token_array_free`.
#[repr(C)]
pub struct FiTokenArray {
    pub data: *mut u32,
    pub len: usize,
    pub capacity: usize,
}

impl FiTokenArray {
    pub fn from_vec(mut v: Vec<u32>) -> Self {
        let arr = FiTokenArray {
            data: v.as_mut_ptr(),
            len: v.len(),
            capacity: v.capacity(),
        };
        std::mem::forget(v);
        arr
    }

    pub fn null() -> Self {
        FiTokenArray {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

/// Top-k similarity result entry
#[repr(C)]
pub struct FiTopKEntry {
    pub index: usize,
    pub score: f32,
}

/// Top-k similarity result array
///
/// Must be freed with `fi_top_k_result_free`.
#[repr(C)]
pub struct FiTopKResult {
    pub entries: *mut FiTopKEntry,
    pub len: usize,
    pub capacity: usize,
}

impl FiTopKResult {
    pub fn from_vec(entries: Vec<(usize, f32)>) -> Self {
        let mut mapped: Vec<FiTopKEntry> = entries
            .into_iter()
            .map(|(index, score)| FiTopKEntry { index, score })
            .collect();
        let result = FiTopKResult {
            entries: mapped.as_mut_ptr(),
            len: mapped.len(),
            capacity: mapped.capacity(),
        };
        std::mem::forget(mapped);
        result
    }

    pub fn null() -> Self {
        FiTopKResult {
            entries: ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

/// Rust-owned f32 array returned to the caller
///
/// Must be freed with `fi_float_array_free`.
#[repr(C)]
pub struct FiFloatArray {
    pub data: *mut f32,
    pub len: usize,
    pub capacity: usize,
}

impl FiFloatArray {
    pub fn from_vec(mut v: Vec<f32>) -> Self {
        let arr = FiFloatArray {
            data: v.as_mut_ptr(),
            len: v.len(),
            capacity: v.capacity(),
        };
        std::mem::forget(v);
        arr
    }

    pub fn null() -> Self {
        FiFloatArray {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

// --- Free functions ---

/// Free a FiBuffer allocated by fi-accel
#[no_mangle]
pub unsafe extern "C" fn fi_buffer_free(buf: FiBuffer) {
    if !buf.data.is_null() {
        drop(Vec::from_raw_parts(buf.data, buf.len, buf.capacity));
    }
}

/// Free a FiTokenArray allocated by fi-accel
#[no_mangle]
pub unsafe extern "C" fn fi_token_array_free(arr: FiTokenArray) {
    if !arr.data.is_null() {
        drop(Vec::from_raw_parts(arr.data, arr.len, arr.capacity));
    }
}

/// Free a FiTopKResult allocated by fi-accel
#[no_mangle]
pub unsafe extern "C" fn fi_top_k_result_free(res: FiTopKResult) {
    if !res.entries.is_null() {
        drop(Vec::from_raw_parts(res.entries, res.len, res.capacity));
    }
}

/// Free a FiFloatArray allocated by fi-accel
#[no_mangle]
pub unsafe extern "C" fn fi_float_array_free(arr: FiFloatArray) {
    if !arr.data.is_null() {
        drop(Vec::from_raw_parts(arr.data, arr.len, arr.capacity));
    }
}

// --- Helpers ---

/// Convert a C string pointer to a Rust &str, returning None on null/invalid UTF-8
pub(crate) unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}
