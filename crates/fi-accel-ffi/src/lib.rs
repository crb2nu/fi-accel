//! fi-accel-ffi: C FFI bindings for fi-accel-core
//!
//! Provides C-compatible function exports for Go, C, and C++ integration.
//! All functions use opaque handles and flat buffers for memory safety.
//!
//! ## Memory Management
//!
//! - Opaque handles (`FiTokenizer*`) are created by `*_new()` and freed by `*_free()`
//! - `FiBuffer` is a Rust-owned byte buffer freed by `fi_buffer_free()`
//! - `FiTokenArray` is a Rust-owned token array freed by `fi_token_array_free()`
//! - `FiError` is stack-allocated by the caller; Rust writes error code + message
//! - Flat arrays (embeddings) use caller-provided output buffers where possible

pub mod types;
pub mod tokenizer;
pub mod embedding;
pub mod diff;
pub mod json;

// Re-export free functions at top level for C visibility
pub use types::{fi_buffer_free, fi_token_array_free, fi_top_k_result_free, fi_float_array_free};
pub use tokenizer::{
    fi_tokenizer_new, fi_tokenizer_free, fi_tokenizer_count,
    fi_tokenizer_encode, fi_tokenizer_decode, fi_tokenizer_truncate,
};
pub use embedding::{
    fi_cosine_similarity, fi_dot_product, fi_normalize_l2, fi_top_k_similar,
    fi_cosine_similarity_matrix, fi_compute_centroid,
};
pub use diff::{fi_string_similarity, fi_line_similarity};
pub use json::{fi_parse_ndjson, fi_to_ndjson};
