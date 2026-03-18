# fi-accel (Python)

Python bindings for `fi-accel` implemented with PyO3 + maturin.

The package is built from the Rust crate at `crates/fi-accel-py`.

Current Python surface:

- tokenizer helpers: `count_tokens`, `truncate_to_tokens`, `split_by_tokens`, `Tokenizer`
- embedding helpers: `normalize_l2`, `cosine_similarity`, `cosine_similarity_matrix`, `top_k_similar`, `compute_centroid`, `prepare_embedding_batch`
- diff helpers: `FuzzyMatcher`, `parse_unified_diff`
- log helpers: `LogReader`
- JSON helpers: `parse_ndjson`, `parse_jsonrpc_batch`, `parse_jsonrpc_lines`, `serialize_jsonrpc_lines`
- transport helpers: `parse_sse_events`, `extract_mcp_http_meta`
- config helpers: `parse_toml`, `validate_toml`, `json_to_toml`, `merge_toml`, `parse_toon`, `to_toon`, `validate_toon`, `json_to_toon`
- chunking helpers: `extract_calls`, `chunk_code`, `chunk_markdown`, `chunk_text`, `chunk_auto`, `chunk_batch`, `detect_content_type`
- chunk metadata now includes additive indexing fields: `file_path`, `start_column`, `end_column`, `imports`, `parent_type`, `symbol_kind`, `container_name`, `container_kind`, `symbol_path`, `calls`, `definitions`
- runtime capability report: `capabilities()`
