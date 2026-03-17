//! WASM bindings for fi-accel
//!
//! Exposes high-performance Rust implementations to JavaScript via wasm-bindgen.

use fi_accel_core::{chunker, diff, embedding, json, log, surface, toml, toon, transport};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ============================================================================
// Embedding Module
// ============================================================================

/// Normalize vectors to unit length (L2 normalization)
#[wasm_bindgen]
pub fn normalize_l2(embeddings: Vec<f32>, dimensions: usize) -> Vec<f32> {
    // Convert flat array to Vec<Vec<f32>>
    let mut vectors: Vec<Vec<f32>> = embeddings.chunks(dimensions).map(|c| c.to_vec()).collect();

    embedding::normalize_l2(&mut vectors);

    // Flatten back
    vectors.into_iter().flatten().collect()
}

/// Compute cosine similarity between two vectors
#[wasm_bindgen]
pub fn cosine_similarity(a: Vec<f32>, b: Vec<f32>) -> f32 {
    embedding::cosine_similarity(&a, &b)
}

/// Compute cosine similarity matrix (flattened)
/// Input: two flat arrays with vectors of `dimensions` each
/// Output: flat array of size (a_count * b_count)
#[wasm_bindgen]
pub fn cosine_similarity_matrix_flat(a: Vec<f32>, b: Vec<f32>, dimensions: usize) -> Vec<f32> {
    let a_vecs: Vec<Vec<f32>> = a.chunks(dimensions).map(|c| c.to_vec()).collect();
    let b_vecs: Vec<Vec<f32>> = b.chunks(dimensions).map(|c| c.to_vec()).collect();

    let matrix = embedding::cosine_similarity_matrix(&a_vecs, &b_vecs);

    // Flatten the result
    matrix.into_iter().flatten().collect()
}

/// Compute dot product of two vectors
#[wasm_bindgen]
pub fn dot_product(a: Vec<f32>, b: Vec<f32>) -> f32 {
    embedding::dot_product(&a, &b)
}

// ============================================================================
// Diff Module
// ============================================================================

/// Result of fuzzy patch application
#[derive(Serialize, Deserialize)]
pub struct WasmPatchResult {
    pub content: String,
    pub applied: usize,
    pub fuzzy_applied: usize,
    pub failed: usize,
}

/// Apply a fuzzy patch to a file
#[wasm_bindgen]
pub fn apply_fuzzy_patch(patch: &str, file: &str, threshold: f32) -> Result<JsValue, JsValue> {
    let matcher = diff::FuzzyMatcher::new(threshold);

    let result = matcher
        .apply_patch(patch, file)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let wasm_result = WasmPatchResult {
        content: result.content,
        applied: result.applied,
        fuzzy_applied: result.fuzzy_applied,
        failed: result.failed,
    };

    serde_wasm_bindgen::to_value(&wasm_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compute string similarity using edit distance
#[wasm_bindgen]
pub fn string_similarity(a: &str, b: &str) -> f32 {
    let matcher = diff::FuzzyMatcher::default();
    matcher.similarity(a, b)
}

// ============================================================================
// Log Module
// ============================================================================

/// WASM-friendly log reader
#[wasm_bindgen]
pub struct WasmLogReader {
    inner: log::LogReader,
}

#[derive(Serialize, Deserialize)]
pub struct WasmSurfaceLogFilter {
    #[serde(default)]
    pub levels: Vec<surface::SurfaceLogLevel>,
    pub search_term: Option<String>,
    #[serde(default)]
    pub search_regex: bool,
}

#[derive(Serialize, Deserialize)]
pub struct WasmSurfaceLogAnalysisInput {
    pub lines: Vec<String>,
    #[serde(default)]
    pub filter: Option<WasmSurfaceLogFilter>,
}

#[wasm_bindgen]
impl WasmLogReader {
    /// Create a new log reader from initial content
    #[wasm_bindgen(constructor)]
    pub fn new(content: &str) -> Self {
        Self {
            inner: log::LogReader::new(content),
        }
    }

    /// Create an empty log reader
    pub fn empty() -> Self {
        Self {
            inner: log::LogReader::empty(),
        }
    }

    /// Append new content and return new lines as JSON array
    pub fn append(&mut self, content: &str) -> Vec<String> {
        self.inner.append(content)
    }

    /// Get the last n lines
    pub fn tail(&self, n: usize) -> Vec<String> {
        self.inner.tail(n)
    }

    /// Filter lines by pattern (regex)
    pub fn filter(&self, pattern: &str) -> Vec<String> {
        self.inner.filter(pattern)
    }

    /// Search for lines containing substring, returns JSON array of [lineNum, content]
    pub fn search(&self, substring: &str) -> JsValue {
        let results = self.inner.search(substring);
        serde_wasm_bindgen::to_value(&results).unwrap_or(JsValue::NULL)
    }

    /// Get lines around a specific line number
    pub fn context(&self, line_num: usize, before: usize, after: usize) -> Vec<String> {
        self.inner.context(line_num, before, after)
    }

    /// Get total line count
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get all lines
    pub fn all_lines(&self) -> Vec<String> {
        self.inner.all_lines().to_vec()
    }
}

// ============================================================================
// Surface Helpers
// ============================================================================

/// Detect a log level using FlexDeck-inspired heuristics.
#[wasm_bindgen]
pub fn surface_detect_log_level(line: &str) -> String {
    match surface::detect_log_level(line) {
        surface::SurfaceLogLevel::Error => "error",
        surface::SurfaceLogLevel::Warn => "warn",
        surface::SurfaceLogLevel::Info => "info",
        surface::SurfaceLogLevel::Debug => "debug",
    }
    .to_string()
}

/// Analyze a batch of log lines for filter and search matches.
#[wasm_bindgen]
pub fn surface_analyze_logs(payload: JsValue) -> Result<JsValue, JsValue> {
    let input: WasmSurfaceLogAnalysisInput =
        serde_wasm_bindgen::from_value(payload).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let filter = input
        .filter
        .map(|value| surface::SurfaceLogFilter {
            levels: value.levels,
            search_term: value.search_term,
            search_regex: value.search_regex,
        })
        .unwrap_or_default();
    let matches = surface::analyze_log_lines(&input.lines, &filter);
    serde_wasm_bindgen::to_value(&matches).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Check whether a label set matches a selector map.
#[wasm_bindgen]
pub fn surface_matches_label_selector(selector: JsValue, labels: JsValue) -> Result<bool, JsValue> {
    let selector: std::collections::HashMap<String, String> =
        serde_wasm_bindgen::from_value(selector).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let labels: std::collections::HashMap<String, String> =
        serde_wasm_bindgen::from_value(labels).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(surface::matches_label_selector(&labels, &selector))
}

#[derive(Serialize, Deserialize)]
pub struct WasmLabelSelectorFilterInput {
    pub selector: std::collections::HashMap<String, String>,
    pub label_sets: Vec<std::collections::HashMap<String, String>>,
}

/// Return matching indexes for a selector across many label sets.
#[wasm_bindgen]
pub fn surface_filter_label_selector(payload: JsValue) -> Result<JsValue, JsValue> {
    let input: WasmLabelSelectorFilterInput =
        serde_wasm_bindgen::from_value(payload).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let matches = surface::filter_label_selector_matches(&input.label_sets, &input.selector);
    serde_wasm_bindgen::to_value(&matches).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Normalize CI job status tokens into a small stable vocabulary.
#[wasm_bindgen]
pub fn surface_normalize_ci_job_status(status: &str) -> String {
    match surface::normalize_ci_job_status(status) {
        surface::SurfaceCiStatus::Pending => "pending",
        surface::SurfaceCiStatus::Running => "running",
        surface::SurfaceCiStatus::Success => "success",
        surface::SurfaceCiStatus::Failed => "failed",
        surface::SurfaceCiStatus::Manual => "manual",
        surface::SurfaceCiStatus::Skipped => "skipped",
        surface::SurfaceCiStatus::Canceled => "canceled",
    }
    .to_string()
}

/// Normalize CI pipeline status tokens into a small stable vocabulary.
#[wasm_bindgen]
pub fn surface_normalize_ci_pipeline_status(status: &str) -> String {
    match surface::normalize_ci_pipeline_status(status) {
        surface::SurfaceCiStatus::Pending => "pending",
        surface::SurfaceCiStatus::Running => "running",
        surface::SurfaceCiStatus::Success => "success",
        surface::SurfaceCiStatus::Failed => "failed",
        surface::SurfaceCiStatus::Manual => "manual",
        surface::SurfaceCiStatus::Skipped => "skipped",
        surface::SurfaceCiStatus::Canceled => "canceled",
    }
    .to_string()
}

// ============================================================================
// JSON Module
// ============================================================================

/// Parse newline-delimited JSON (NDJSON)
#[wasm_bindgen]
pub fn parse_ndjson(data: &[u8]) -> JsValue {
    let values = json::parse_ndjson(data);
    serde_wasm_bindgen::to_value(&values).unwrap_or(JsValue::NULL)
}

/// Parse NDJSON from string
#[wasm_bindgen]
pub fn parse_ndjson_str(data: &str) -> JsValue {
    parse_ndjson(data.as_bytes())
}

/// JSON-RPC message structure for WASM
#[derive(Serialize, Deserialize)]
pub struct WasmJsonRpcMessage {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<WasmJsonRpcError>,
}

#[derive(Serialize, Deserialize)]
pub struct WasmJsonRpcError {
    pub code: i32,
    pub message: String,
}

/// Parse JSON-RPC messages from NDJSON
#[wasm_bindgen]
pub fn parse_jsonrpc_batch(data: &[u8]) -> JsValue {
    let messages: Vec<WasmJsonRpcMessage> = json::parse_jsonrpc_batch(data)
        .into_iter()
        .filter_map(|r| r.ok())
        .map(|msg| WasmJsonRpcMessage {
            jsonrpc: msg.jsonrpc,
            id: msg.id,
            method: msg.method,
            params: msg.params,
            result: msg.result,
            error: msg.error.map(|e| WasmJsonRpcError {
                code: e.code,
                message: e.message,
            }),
        })
        .collect();

    serde_wasm_bindgen::to_value(&messages).unwrap_or(JsValue::NULL)
}

/// Parse JSON-RPC messages from string
#[wasm_bindgen]
pub fn parse_jsonrpc_batch_str(data: &str) -> JsValue {
    parse_jsonrpc_batch(data.as_bytes())
}

/// Parse JSON-RPC messages from line-delimited framing.
#[wasm_bindgen]
pub fn parse_jsonrpc_lines(data: &[u8]) -> Result<JsValue, JsValue> {
    let messages =
        transport::parse_jsonrpc_lines(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&messages).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse JSON-RPC messages from string framing.
#[wasm_bindgen]
pub fn parse_jsonrpc_lines_str(data: &str) -> Result<JsValue, JsValue> {
    parse_jsonrpc_lines(data.as_bytes())
}

/// Serialize JSON-RPC messages into line-delimited framing.
#[wasm_bindgen]
pub fn serialize_jsonrpc_lines(messages: JsValue) -> Result<Vec<u8>, JsValue> {
    let messages: Vec<json::JsonRpcMessage> =
        serde_wasm_bindgen::from_value(messages).map_err(|e| JsValue::from_str(&e.to_string()))?;
    transport::serialize_jsonrpc_lines(&messages).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse SSE events from an event stream.
#[wasm_bindgen]
pub fn parse_sse_events(data: &[u8]) -> Result<JsValue, JsValue> {
    let events =
        transport::parse_sse_events(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&events).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse SSE events from string data.
#[wasm_bindgen]
pub fn parse_sse_events_str(data: &str) -> Result<JsValue, JsValue> {
    parse_sse_events(data.as_bytes())
}

/// Extract MCP Streamable HTTP metadata from a raw header block.
#[wasm_bindgen]
pub fn extract_mcp_http_meta(headers: &[u8]) -> Result<JsValue, JsValue> {
    let meta =
        transport::extract_mcp_http_meta(headers).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&meta).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract MCP Streamable HTTP metadata from string headers.
#[wasm_bindgen]
pub fn extract_mcp_http_meta_str(headers: &str) -> Result<JsValue, JsValue> {
    extract_mcp_http_meta(headers.as_bytes())
}

// ============================================================================
// TOML Module
// ============================================================================

/// Parse TOML string and return as JavaScript object
#[wasm_bindgen]
pub fn parse_toml(toml_str: &str) -> Result<JsValue, JsValue> {
    let json_value = toml::parse_toml(toml_str).map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&json_value).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate TOML syntax without fully parsing
#[wasm_bindgen]
pub fn validate_toml(toml_str: &str) -> Result<bool, JsValue> {
    toml::validate_toml(toml_str)
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert a JavaScript object to TOML string
#[wasm_bindgen]
pub fn json_to_toml(obj: JsValue) -> Result<String, JsValue> {
    let json_value: serde_json::Value =
        serde_wasm_bindgen::from_value(obj).map_err(|e| JsValue::from_str(&e.to_string()))?;

    toml::json_to_toml(&json_value).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Merge two TOML strings, with overlay values taking precedence
#[wasm_bindgen]
pub fn merge_toml(base: &str, overlay: &str) -> Result<JsValue, JsValue> {
    let merged = toml::merge_toml(base, overlay).map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&merged).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ============================================================================
// TOON Module
// ============================================================================

/// Parse TOON string and return as JavaScript object
#[wasm_bindgen]
pub fn parse_toon(toon_str: &str) -> Result<JsValue, JsValue> {
    let json_value = toon::parse_toon(toon_str).map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&json_value).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert a JavaScript object to TOON string
#[wasm_bindgen]
pub fn to_toon(obj: JsValue) -> Result<String, JsValue> {
    let json_value: serde_json::Value =
        serde_wasm_bindgen::from_value(obj).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(toon::to_toon(&json_value))
}

/// Validate TOON syntax without fully parsing
#[wasm_bindgen]
pub fn validate_toon(toon_str: &str) -> Result<bool, JsValue> {
    toon::validate_toon(toon_str)
        .map(|_| true)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert JSON value to TOON string
#[wasm_bindgen]
pub fn json_to_toon(obj: JsValue) -> Result<String, JsValue> {
    let json_value: serde_json::Value =
        serde_wasm_bindgen::from_value(obj).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(toon::json_to_toon(&json_value))
}

// ============================================================================
// Chunker Module (markdown + text only, no tree-sitter in WASM)
// ============================================================================

/// Chunk markdown content at heading boundaries
#[wasm_bindgen]
pub fn chunk_markdown(content: &str, max_tokens: Option<usize>) -> Result<JsValue, JsValue> {
    let config = chunker::ChunkConfig {
        max_tokens: max_tokens.unwrap_or(512),
        ..chunker::ChunkConfig::default()
    };

    let chunks =
        chunker::chunk_markdown(content, &config).map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&chunks).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Chunk plain text by paragraphs and sentences
#[wasm_bindgen]
pub fn chunk_text(content: &str, max_tokens: Option<usize>) -> Result<JsValue, JsValue> {
    let config = chunker::ChunkConfig {
        max_tokens: max_tokens.unwrap_or(512),
        ..chunker::ChunkConfig::default()
    };

    let chunks = chunker::chunk_text(content, &config);

    serde_wasm_bindgen::to_value(&chunks).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Auto-detect content type and chunk when supported by the WASM binding
#[wasm_bindgen]
pub fn chunk_auto(
    content: &str,
    file_path: &str,
    max_tokens: Option<usize>,
) -> Result<JsValue, JsValue> {
    let config = chunker::ChunkConfig {
        max_tokens: max_tokens.unwrap_or(512),
        file_path: Some(file_path.to_string()),
        ..chunker::ChunkConfig::default()
    };

    let content_type = chunker::detect_content_type(file_path);
    let chunks = chunker::chunk(content, content_type, &config)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&chunks).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Detect content type from file path
#[wasm_bindgen]
pub fn detect_content_type(file_path: &str) -> String {
    match chunker::detect_content_type(file_path) {
        chunker::ContentType::Code(lang) => format!("code:{}", lang.as_str()),
        chunker::ContentType::Markdown => "markdown".to_string(),
        chunker::ContentType::PlainText => "text".to_string(),
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Report the capabilities exposed by the WASM binding.
#[wasm_bindgen]
pub fn capabilities() -> JsValue {
    serde_wasm_bindgen::to_value(&serde_json::json!({
        "tokenizer": false,
        "simd_json": true,
        "transport": true,
        "chunker_code": false,
        "chunker_markdown": true,
        "chunker_text": true,
        "ffi_available": false,
    }))
    .unwrap_or(JsValue::NULL)
}
