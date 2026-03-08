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

pub mod diff;
pub mod embedding;
pub mod json;
pub mod transport;
pub mod tokenizer;
pub mod types;

// Re-export free functions at top level for C visibility
pub use diff::{fi_line_similarity, fi_string_similarity};
pub use embedding::{
    fi_compute_centroid, fi_cosine_similarity, fi_cosine_similarity_matrix, fi_dot_product,
    fi_normalize_l2, fi_top_k_similar,
};
pub use json::{fi_parse_ndjson, fi_to_ndjson};
pub use transport::{
    fi_extract_mcp_http_meta, fi_parse_jsonrpc_lines, fi_parse_sse_events,
    fi_serialize_jsonrpc_lines,
};
pub use tokenizer::{
    fi_tokenizer_count, fi_tokenizer_decode, fi_tokenizer_encode, fi_tokenizer_free,
    fi_tokenizer_new, fi_tokenizer_truncate,
};
pub use types::{fi_buffer_free, fi_float_array_free, fi_token_array_free, fi_top_k_result_free};
