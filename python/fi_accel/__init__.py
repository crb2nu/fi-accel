"""
fi-accel: High-performance acceleration library for agent/MCP workflows

This library provides Rust-accelerated implementations of common operations
used in LLM-based agent systems:

- Token counting and manipulation (10-50x faster than tiktoken)
- SIMD-accelerated embedding operations (50-100x faster similarity)
- Fuzzy diff matching for patch application (10-100x faster for large files)
- Efficient log streaming and filtering
- SIMD JSON parsing
- Syntax-aware chunking for code, markdown, and text

Example usage:
    >>> from fi_accel import count_tokens, cosine_similarity_matrix
    >>> count_tokens("Hello, world!", model="gpt-4")
    4
    >>> matrix = cosine_similarity_matrix([[1, 0], [0, 1]], [[1, 0], [0, 1]])
"""

import hashlib

# Import all public functions from the Rust extension
from fi_accel.fi_accel import (
    # Tokenizer
    count_tokens,
    truncate_to_tokens,
    split_by_tokens,
    Tokenizer,
    # Embeddings
    normalize_l2,
    cosine_similarity,
    cosine_similarity_matrix,
    top_k_similar,
    compute_centroid,
    prepare_embedding_batch,
    # Ranking
    hybrid_rank,
    rrf_fuse,
    # Diff
    FuzzyMatcher,
    parse_unified_diff,
    # Log
    LogReader,
    # JSON
    parse_ndjson,
    parse_jsonrpc_batch,
    parse_jsonrpc_lines,
    serialize_jsonrpc_lines,
    parse_sse_events,
    extract_mcp_http_meta,
    project_eventlog,
    summarize_eventlog,
    sha256_batch,
    verify_hash_chain,
    # TOML
    parse_toml,
    validate_toml,
    json_to_toml,
    merge_toml,
    # TOON
    parse_toon,
    to_toon,
    validate_toon,
    json_to_toon,
    # Chunker
    extract_calls,
    chunk_code,
    chunk_markdown,
    chunk_text,
    chunk_auto,
    chunk_batch,
    detect_content_type,
    capabilities,
)

__version__ = "0.1.0"


def sha256_stream(reader, chunk_size=65536):
    """Compute the SHA-256 hex digest for a binary file-like reader."""
    hasher = hashlib.sha256()
    while True:
        chunk = reader.read(chunk_size)
        if not chunk:
            break
        hasher.update(chunk)
    return hasher.hexdigest()


__all__ = [
    # Tokenizer
    "count_tokens",
    "truncate_to_tokens",
    "split_by_tokens",
    "Tokenizer",
    # Embeddings
    "normalize_l2",
    "cosine_similarity",
    "cosine_similarity_matrix",
    "top_k_similar",
    "compute_centroid",
    "prepare_embedding_batch",
    # Ranking
    "hybrid_rank",
    "rrf_fuse",
    # Diff
    "FuzzyMatcher",
    "parse_unified_diff",
    # Log
    "LogReader",
    # JSON
    "parse_ndjson",
    "parse_jsonrpc_batch",
    "parse_jsonrpc_lines",
    "serialize_jsonrpc_lines",
    "parse_sse_events",
    "extract_mcp_http_meta",
    "project_eventlog",
    "summarize_eventlog",
    "sha256_batch",
    "sha256_stream",
    "verify_hash_chain",
    # TOML
    "parse_toml",
    "validate_toml",
    "json_to_toml",
    "merge_toml",
    # TOON
    "parse_toon",
    "to_toon",
    "validate_toon",
    "json_to_toon",
    # Chunker
    "extract_calls",
    "chunk_code",
    "chunk_markdown",
    "chunk_text",
    "chunk_auto",
    "chunk_batch",
    "detect_content_type",
    "capabilities",
]
