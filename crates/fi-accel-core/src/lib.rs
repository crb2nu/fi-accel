//! fi-accel-core: High-performance acceleration for agent/MCP workflows
//!
//! This crate provides optimized implementations for common operations in
//! LLM-based agent systems:
//!
//! - **tokenizer**: Fast token counting, truncation, and splitting
//! - **embedding**: SIMD-accelerated vector operations
//! - **diff**: Fuzzy patch matching and application
//! - **log**: Efficient log streaming and filtering
//! - **json**: SIMD-accelerated JSON parsing
//! - **toml**: Fast TOML parsing and conversion
//! - **toon**: TOON (Token Oriented Object Notation) parsing
//! - **chunker**: Syntax-aware content chunking (code, markdown, text)
//! - **ranking**: Hybrid ranking and reciprocal rank fusion helpers

pub mod capabilities;
pub mod diff;
pub mod embedding;
pub mod eventlog;
pub mod hashing;
pub mod json;
pub mod log;
pub mod ranking;
pub mod surface;
pub mod toml;
pub mod toon;
pub mod transport;

#[cfg(feature = "tokenizer")]
pub mod tokenizer;

#[cfg(any(feature = "chunker", feature = "chunker-base"))]
pub mod chunker;

// Re-export commonly used types
pub use capabilities::{capabilities, CapabilitySet};
pub use diff::{DiffError, FuzzyMatcher, PatchResult};
pub use embedding::{cosine_similarity, cosine_similarity_matrix, normalize_l2};
pub use eventlog::{
    project_eventlog_ndjson, project_events, summarize_eventlog_ndjson, summarize_events,
    EventProjectionConfig, EventRecord, EventSummary,
};
pub use hashing::{sha256_batch, verify_hash_chain, HashChainConfig, HashChainVerification};
pub use json::parse_ndjson;
pub use log::LogReader;
pub use ranking::{hybrid_rank, hybrid_rank_by_name, rrf_fuse, HybridRankMethod, RankingError};
pub use surface::{
    analyze_log_lines, detect_log_level, filter_label_selector_matches, matches_label_selector,
    normalize_ci_job_status, normalize_ci_pipeline_status, SurfaceCiStatus, SurfaceLogFilter,
    SurfaceLogLevel, SurfaceLogMatch,
};
pub use toml::{json_to_toml, merge_toml, parse_toml, validate_toml};
pub use toon::{json_to_toon, parse_toon, to_toon, validate_toon};
pub use transport::{
    extract_mcp_http_meta, parse_jsonrpc_lines, parse_sse_events, serialize_jsonrpc_lines,
    McpHttpMeta, SseEvent, TransportError,
};

#[cfg(feature = "tokenizer")]
pub use tokenizer::{count_tokens, split_by_tokens, truncate_to_tokens, Tokenizer};

#[cfg(any(feature = "chunker", feature = "chunker-base"))]
pub use chunker::{
    chunk, chunk_markdown, chunk_text, detect_content_type, Chunk, ChunkConfig, ChunkKind,
    ChunkerError, ContentType, Language,
};

#[cfg(feature = "tree-sitter")]
pub use chunker::extract_calls;

// FFI exports for C/Go integration
#[cfg(feature = "ffi")]
pub mod ffi;
