//! Python bindings for fi-accel
//!
//! Exposes high-performance Rust implementations to Python via PyO3.

use fi_accel_core::{
    chunker, diff, embedding, encoding, eventlog, hashing, json, log, ranking, text, tokenizer,
    toml, toon, transport,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rayon::prelude::*;

// ============================================================================
// Tokenizer Module
// ============================================================================

/// Count tokens in text for a given model
#[pyfunction]
#[pyo3(signature = (text, model = "gpt-4"))]
fn count_tokens(text: &str, model: &str) -> PyResult<usize> {
    tokenizer::count_tokens(text, model).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Truncate text to fit within max_tokens
#[pyfunction]
#[pyo3(signature = (text, max_tokens, model = "gpt-4"))]
fn truncate_to_tokens(text: &str, max_tokens: usize, model: &str) -> PyResult<String> {
    tokenizer::truncate_to_tokens(text, max_tokens, model)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Split text into chunks of approximately chunk_size tokens with overlap
#[pyfunction]
#[pyo3(signature = (text, chunk_size, overlap = 0, model = "gpt-4"))]
fn split_by_tokens(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    model: &str,
) -> PyResult<Vec<String>> {
    tokenizer::split_by_tokens(text, chunk_size, overlap, model)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Tokenizer class for reusable token operations
#[pyclass]
struct Tokenizer {
    inner: tokenizer::Tokenizer,
}

#[pymethods]
impl Tokenizer {
    #[new]
    #[pyo3(signature = (model = "gpt-4"))]
    fn new(model: &str) -> PyResult<Self> {
        let inner =
            tokenizer::Tokenizer::new(model).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    fn count(&self, text: &str) -> usize {
        self.inner.count(text)
    }

    fn truncate(&self, text: &str, max_tokens: usize) -> String {
        self.inner.truncate(text, max_tokens)
    }

    fn split(&self, text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
        self.inner.split(text, chunk_size, overlap)
    }

    fn encode(&self, text: &str) -> Vec<u32> {
        self.inner.encode(text)
    }

    fn decode(&self, tokens: Vec<u32>) -> PyResult<String> {
        self.inner
            .decode(&tokens)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

// ============================================================================
// Embedding Module
// ============================================================================

/// Normalize vectors to unit length (L2 normalization) in-place
#[pyfunction]
fn normalize_l2(embeddings: Vec<Vec<f32>>) -> Vec<Vec<f32>> {
    let mut result = embeddings;
    embedding::normalize_l2(&mut result);
    result
}

/// Compute cosine similarity between two vectors (assumes normalized)
#[pyfunction]
fn cosine_similarity(a: Vec<f32>, b: Vec<f32>) -> PyResult<f32> {
    if a.len() != b.len() {
        return Err(PyValueError::new_err("Vectors must have same length"));
    }
    Ok(embedding::cosine_similarity(&a, &b))
}

/// Compute cosine similarity matrix between two sets of vectors
#[pyfunction]
fn cosine_similarity_matrix(a: Vec<Vec<f32>>, b: Vec<Vec<f32>>) -> Vec<Vec<f32>> {
    embedding::cosine_similarity_matrix(&a, &b)
}

/// Compute top-k most similar vectors from b for each vector in a
#[pyfunction]
fn top_k_similar(a: Vec<Vec<f32>>, b: Vec<Vec<f32>>, k: usize) -> Vec<Vec<(usize, f32)>> {
    embedding::top_k_similar(&a, &b, k)
}

/// Compute centroid (average) of embeddings
#[pyfunction]
fn compute_centroid(embeddings: Vec<Vec<f32>>) -> Option<Vec<f32>> {
    embedding::compute_centroid(&embeddings)
}

/// Prepare texts for embedding by truncating to max_tokens
#[pyfunction]
#[pyo3(signature = (texts, max_tokens, model = "gpt-4"))]
fn prepare_embedding_batch(
    texts: Vec<String>,
    max_tokens: usize,
    model: &str,
) -> PyResult<Vec<(String, usize, bool)>> {
    let prepared = embedding::prepare_embedding_batch(texts, max_tokens, model)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(prepared
        .into_iter()
        .map(|p| (p.text, p.token_count, p.truncated))
        .collect())
}

// ============================================================================
// Ranking Module
// ============================================================================

/// Combine lexical ranks and cosine scores into a stable ranked list.
#[pyfunction]
#[pyo3(signature = (lex_ranks, cosine_scores, alpha = 0.5, method = "weighted_sum", k = 60))]
fn hybrid_rank(
    lex_ranks: Vec<usize>,
    cosine_scores: Vec<f32>,
    alpha: f32,
    method: &str,
    k: usize,
) -> PyResult<Vec<(usize, f32)>> {
    ranking::hybrid_rank_by_name(&lex_ranks, &cosine_scores, alpha, method, k)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Fuse ranked item-id lists with reciprocal rank fusion.
#[pyfunction]
#[pyo3(signature = (ranked_lists, k = 60))]
fn rrf_fuse(ranked_lists: Vec<Vec<usize>>, k: usize) -> PyResult<Vec<(usize, f32)>> {
    ranking::rrf_fuse(&ranked_lists, k).map_err(|e| PyValueError::new_err(e.to_string()))
}

// ============================================================================
// Diff Module
// ============================================================================

/// Fuzzy matcher for intelligent patch application
#[pyclass]
struct FuzzyMatcher {
    inner: diff::FuzzyMatcher,
}

#[pymethods]
impl FuzzyMatcher {
    #[new]
    #[pyo3(signature = (threshold = 0.6, max_offset = 100))]
    fn new(threshold: f32, max_offset: usize) -> Self {
        Self {
            inner: diff::FuzzyMatcher {
                threshold,
                max_offset,
                parallel: true,
            },
        }
    }

    /// Compute similarity between two strings
    fn similarity(&self, a: &str, b: &str) -> f32 {
        self.inner.similarity(a, b)
    }

    /// Apply a patch to a file, returning the result
    fn apply_patch(&self, py: Python<'_>, patch: &str, file: &str) -> PyResult<PyObject> {
        let result = self
            .inner
            .apply_patch(patch, file)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let dict = PyDict::new_bound(py);
        dict.set_item("content", result.content)?;
        dict.set_item("applied", result.applied)?;
        dict.set_item("fuzzy_applied", result.fuzzy_applied)?;
        dict.set_item("failed", result.failed)?;

        let details: Vec<PyObject> = result
            .details
            .into_iter()
            .map(|d| {
                let detail = PyDict::new_bound(py);
                detail.set_item("patch_line", d.patch_line).unwrap();
                detail.set_item("applied_at", d.applied_at).unwrap();
                detail.set_item("similarity", d.similarity).unwrap();
                detail.set_item("success", d.success).unwrap();
                detail.set_item("error", d.error).unwrap();
                detail.into()
            })
            .collect();
        dict.set_item("details", details)?;

        Ok(dict.into())
    }
}

/// Parse a unified diff into hunks
#[pyfunction]
fn parse_unified_diff(py: Python<'_>, patch: &str) -> PyResult<PyObject> {
    let hunks =
        diff::parse_unified_diff(patch).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let result: Vec<PyObject> = hunks
        .into_iter()
        .map(|h| {
            let dict = PyDict::new_bound(py);
            dict.set_item("old_start", h.old_start).unwrap();
            dict.set_item("old_count", h.old_count).unwrap();
            dict.set_item("new_start", h.new_start).unwrap();
            dict.set_item("new_count", h.new_count).unwrap();
            dict.into()
        })
        .collect();

    Ok(PyList::new_bound(py, result).into())
}

// ============================================================================
// Log Module
// ============================================================================

/// Efficient log reader with incremental updates
#[pyclass]
struct LogReader {
    inner: log::LogReader,
}

#[pymethods]
impl LogReader {
    #[new]
    #[pyo3(signature = (content = ""))]
    fn new(content: &str) -> Self {
        Self {
            inner: log::LogReader::new(content),
        }
    }

    /// Append new content and return new lines
    fn append(&mut self, content: &str) -> Vec<String> {
        self.inner.append(content)
    }

    /// Get the last n lines
    fn tail(&self, n: usize) -> Vec<String> {
        self.inner.tail(n)
    }

    /// Filter lines by pattern (regex)
    fn filter(&self, pattern: &str) -> Vec<String> {
        self.inner.filter(pattern)
    }

    /// Search for lines containing substring
    fn search(&self, substring: &str) -> Vec<(usize, String)> {
        self.inner.search(substring)
    }

    /// Get lines around a specific line number
    fn context(&self, line_num: usize, before: usize, after: usize) -> Vec<String> {
        self.inner.context(line_num, before, after)
    }

    /// Get total line count
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Get all lines
    fn all_lines(&self) -> Vec<String> {
        self.inner.all_lines().to_vec()
    }
}

// ============================================================================
// JSON Module
// ============================================================================

fn json_str_to_py(py: Python<'_>, json_str: String) -> PyResult<PyObject> {
    let json_module = py.import_bound("json")?;
    Ok(json_module.call_method1("loads", (json_str,))?.unbind())
}

fn json_value_to_py(py: Python<'_>, value: serde_json::Value) -> PyResult<PyObject> {
    let json_str =
        serde_json::to_string(&value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, json_str)
}

fn json_values_to_py(py: Python<'_>, values: Vec<serde_json::Value>) -> PyResult<Vec<PyObject>> {
    values
        .into_iter()
        .map(|value| json_value_to_py(py, value))
        .collect()
}

/// Parse newline-delimited JSON (NDJSON)
#[pyfunction]
fn parse_ndjson(py: Python<'_>, data: &[u8]) -> PyResult<Vec<PyObject>> {
    let values = json::parse_ndjson(data);
    json_values_to_py(py, values)
}

/// Strict incremental NDJSON reader with partial-line buffering
#[pyclass]
struct NDJSONReader {
    inner: json::NDJSONReader,
}

#[pymethods]
impl NDJSONReader {
    #[new]
    #[pyo3(signature = (initial_content = None))]
    fn new(initial_content: Option<&[u8]>) -> PyResult<Self> {
        let inner = json::NDJSONReader::new(initial_content.unwrap_or_default())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Append bytes and return newly parsed JSON values
    fn append(&mut self, py: Python<'_>, content: &[u8]) -> PyResult<Vec<PyObject>> {
        let values = self
            .inner
            .append(content)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        json_values_to_py(py, values)
    }

    /// Get the last n parsed values
    fn tail(&self, py: Python<'_>, n: usize) -> PyResult<Vec<PyObject>> {
        json_values_to_py(py, self.inner.tail(n))
    }

    /// Get all parsed values
    fn all_values(&self, py: Python<'_>) -> PyResult<Vec<PyObject>> {
        json_values_to_py(py, self.inner.all_values().to_vec())
    }

    /// Return buffered byte count for the current partial line
    fn buffered_len(&self) -> usize {
        self.inner.buffered_len()
    }

    /// Get total parsed value count
    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

/// Parse JSON-RPC messages from NDJSON
#[pyfunction]
fn parse_jsonrpc_batch(py: Python<'_>, data: &[u8]) -> PyResult<Vec<PyObject>> {
    let messages = json::parse_jsonrpc_batch(data);
    messages
        .into_iter()
        .map(|result| match result {
            Ok(msg) => {
                let dict = PyDict::new_bound(py);
                dict.set_item("jsonrpc", msg.jsonrpc)?;
                dict.set_item("id", msg.id.map(|v| v.to_string()))?;
                dict.set_item("method", msg.method)?;
                dict.set_item("params", msg.params.map(|v| v.to_string()))?;
                dict.set_item("result", msg.result.map(|v| v.to_string()))?;
                if let Some(err) = msg.error {
                    let err_dict = PyDict::new_bound(py);
                    err_dict.set_item("code", err.code)?;
                    err_dict.set_item("message", err.message)?;
                    dict.set_item("error", err_dict)?;
                } else {
                    dict.set_item("error", py.None())?;
                }
                Ok(dict.into())
            }
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        })
        .collect()
}

/// Parse JSON-RPC messages from line-delimited framing.
#[pyfunction]
fn parse_jsonrpc_lines(py: Python<'_>, data: &[u8]) -> PyResult<PyObject> {
    let messages =
        transport::parse_jsonrpc_lines(data).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let payload =
        serde_json::to_string(&messages).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, payload)
}

/// Serialize JSON-RPC messages to line-delimited framing.
#[pyfunction]
fn serialize_jsonrpc_lines(py: Python<'_>, messages: PyObject) -> PyResult<Vec<u8>> {
    let json_module = py.import_bound("json")?;
    let json_str: String = json_module.call_method1("dumps", (messages,))?.extract()?;
    let messages: Vec<json::JsonRpcMessage> =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
    transport::serialize_jsonrpc_lines(&messages).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Parse SSE events from an MCP event stream.
#[pyfunction]
fn parse_sse_events(py: Python<'_>, data: &[u8]) -> PyResult<PyObject> {
    let events =
        transport::parse_sse_events(data).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let payload =
        serde_json::to_string(&events).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, payload)
}

/// Extract MCP Streamable HTTP metadata from a raw header block.
#[pyfunction]
fn extract_mcp_http_meta(py: Python<'_>, headers: &[u8]) -> PyResult<PyObject> {
    let meta = transport::extract_mcp_http_meta(headers)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let payload = serde_json::to_string(&meta).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, payload)
}

/// Project NDJSON/JSONL event records into a normalized timeline shape.
#[pyfunction]
#[pyo3(signature = (data, timestamp_field = "timestamp", event_type_field = "event_type", actor_id_field = Some("agent_id".to_string()), actor_type_field = Some("agent_type".to_string()), data_field = None, limit = None))]
fn project_eventlog(
    py: Python<'_>,
    data: &[u8],
    timestamp_field: &str,
    event_type_field: &str,
    actor_id_field: Option<String>,
    actor_type_field: Option<String>,
    data_field: Option<String>,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let config = eventlog::EventProjectionConfig {
        timestamp_field: timestamp_field.to_string(),
        event_type_field: event_type_field.to_string(),
        actor_id_field,
        actor_type_field,
        data_field,
        limit,
    };
    let events = eventlog::project_eventlog_ndjson(data, &config);
    let payload =
        serde_json::to_string(&events).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, payload)
}

/// Summarize NDJSON/JSONL event records by event and actor dimensions.
#[pyfunction]
#[pyo3(signature = (data, timestamp_field = "timestamp", event_type_field = "event_type", actor_id_field = Some("agent_id".to_string()), actor_type_field = Some("agent_type".to_string()), data_field = None, limit = None))]
fn summarize_eventlog(
    py: Python<'_>,
    data: &[u8],
    timestamp_field: &str,
    event_type_field: &str,
    actor_id_field: Option<String>,
    actor_type_field: Option<String>,
    data_field: Option<String>,
    limit: Option<usize>,
) -> PyResult<PyObject> {
    let config = eventlog::EventProjectionConfig {
        timestamp_field: timestamp_field.to_string(),
        event_type_field: event_type_field.to_string(),
        actor_id_field,
        actor_type_field,
        data_field,
        limit,
    };
    let summary = eventlog::summarize_eventlog_ndjson(data, &config);
    let payload =
        serde_json::to_string(&summary).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, payload)
}

// ============================================================================
// Hashing Module
// ============================================================================

/// Compute SHA-256 hex digests for a batch of byte strings.
#[pyfunction]
fn sha256_batch(items: Vec<Vec<u8>>) -> Vec<String> {
    hashing::sha256_batch(&items)
}

/// Verify an append-only hash chain over JSON event objects.
#[pyfunction]
#[pyo3(signature = (events, hash_field = "entry_hash", prev_field = "prev_hash", payload_fields = None, seed = ""))]
fn verify_hash_chain(
    py: Python<'_>,
    events: PyObject,
    hash_field: &str,
    prev_field: &str,
    payload_fields: Option<Vec<String>>,
    seed: &str,
) -> PyResult<PyObject> {
    let json_module = py.import_bound("json")?;
    let json_str: String = json_module
        .call_method1("dumps", (events,))
        .map_err(|e| PyValueError::new_err(e.to_string()))?
        .extract()?;
    let events: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let config = hashing::HashChainConfig {
        hash_field: hash_field.to_string(),
        prev_field: prev_field.to_string(),
        payload_fields,
        seed: seed.to_string(),
    };
    let result = hashing::verify_hash_chain(&events, &config);
    let payload =
        serde_json::to_string(&result).map_err(|e| PyValueError::new_err(e.to_string()))?;
    json_str_to_py(py, payload)
}

// ============================================================================
// Text / Encoding Module
// ============================================================================

/// Convert HTML to plain text for ingestion.
#[pyfunction]
#[pyo3(signature = (html, preserve_links = false))]
fn html_to_text(html: &str, preserve_links: bool) -> String {
    text::html_to_text(html, preserve_links)
}

/// Detect likely character encoding from a byte sample.
#[pyfunction]
#[pyo3(signature = (data, sample_bytes = 8192))]
fn detect_encoding(py: Python<'_>, data: &[u8], sample_bytes: usize) -> PyResult<PyObject> {
    let detected = encoding::detect_encoding(data, sample_bytes);
    let dict = PyDict::new_bound(py);
    dict.set_item("encoding", detected.encoding)?;
    dict.set_item("confidence", detected.confidence)?;
    dict.set_item("bom", detected.bom)?;
    Ok(dict.into())
}

// ============================================================================
// TOML Module
// ============================================================================

/// Parse TOML string to a Python dict (via JSON conversion)
#[pyfunction]
fn parse_toml(py: Python<'_>, toml_str: &str) -> PyResult<PyObject> {
    let json_value =
        toml::parse_toml(toml_str).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json_str =
        serde_json::to_string(&json_value).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json_module = py.import_bound("json")?;
    let result = json_module.call_method1("loads", (json_str,))?;
    Ok(result.unbind())
}

/// Validate TOML syntax without fully parsing
#[pyfunction]
fn validate_toml(toml_str: &str) -> PyResult<bool> {
    match toml::validate_toml(toml_str) {
        Ok(()) => Ok(true),
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

/// Convert a Python dict (via JSON) to TOML string
#[pyfunction]
fn json_to_toml(py: Python<'_>, obj: PyObject) -> PyResult<String> {
    let json_module = py.import_bound("json")?;
    let json_str: String = json_module.call_method1("dumps", (obj,))?.extract()?;

    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;

    toml::json_to_toml(&json_value).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Merge two TOML strings, with overlay overriding base
#[pyfunction]
fn merge_toml(py: Python<'_>, base: &str, overlay: &str) -> PyResult<PyObject> {
    let merged =
        toml::merge_toml(base, overlay).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json_str =
        serde_json::to_string(&merged).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json_module = py.import_bound("json")?;
    let result = json_module.call_method1("loads", (json_str,))?;
    Ok(result.unbind())
}

// ============================================================================
// TOON Module
// ============================================================================

/// Parse TOON (Token Oriented Object Notation) string to a Python dict
#[pyfunction]
fn parse_toon(py: Python<'_>, toon_str: &str) -> PyResult<PyObject> {
    let json_value =
        toon::parse_toon(toon_str).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json_str =
        serde_json::to_string(&json_value).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json_module = py.import_bound("json")?;
    let result = json_module.call_method1("loads", (json_str,))?;
    Ok(result.unbind())
}

/// Convert a Python dict to TOON string
#[pyfunction]
fn to_toon(py: Python<'_>, obj: PyObject) -> PyResult<String> {
    let json_module = py.import_bound("json")?;
    let json_str: String = json_module.call_method1("dumps", (obj,))?.extract()?;

    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(toon::to_toon(&json_value))
}

/// Validate TOON syntax
#[pyfunction]
fn validate_toon(toon_str: &str) -> PyResult<bool> {
    match toon::validate_toon(toon_str) {
        Ok(()) => Ok(true),
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

/// Convert a Python dict (via JSON) to TOON string (alias for to_toon)
#[pyfunction]
fn json_to_toon(py: Python<'_>, obj: PyObject) -> PyResult<String> {
    let json_module = py.import_bound("json")?;
    let json_str: String = json_module.call_method1("dumps", (obj,))?.extract()?;

    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(toon::json_to_toon(&json_value))
}

// ============================================================================
// Chunker Module
// ============================================================================

fn chunk_to_pydict(py: Python<'_>, chunk: &chunker::Chunk) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    dict.set_item("id", &chunk.id)?;
    dict.set_item("content", &chunk.content)?;
    dict.set_item("kind", chunk.kind.as_str())?;
    dict.set_item("chunk_type", &chunk.chunk_type)?;
    dict.set_item("language", &chunk.language)?;
    dict.set_item("file_path", &chunk.file_path)?;
    dict.set_item("start_line", chunk.start_line)?;
    dict.set_item("end_line", chunk.end_line)?;
    dict.set_item("start_column", chunk.start_column)?;
    dict.set_item("end_column", chunk.end_column)?;
    dict.set_item("token_count", chunk.token_count)?;
    dict.set_item("name", &chunk.name)?;
    dict.set_item("parent_name", &chunk.parent_name)?;
    dict.set_item("signature", &chunk.signature)?;
    dict.set_item("docstring", &chunk.docstring)?;
    dict.set_item("imports", &chunk.imports)?;
    dict.set_item("parent_type", &chunk.parent_type)?;
    dict.set_item("symbol_kind", &chunk.symbol_kind)?;
    dict.set_item("container_name", &chunk.container_name)?;
    dict.set_item("container_kind", &chunk.container_kind)?;
    dict.set_item("symbol_path", &chunk.symbol_path)?;
    dict.set_item("calls", &chunk.calls)?;
    dict.set_item("definitions", &chunk.definitions)?;
    Ok(dict.into())
}

/// Extract function, method, and macro calls from source code.
#[pyfunction]
fn extract_calls(language: &str, content: &str) -> PyResult<Vec<String>> {
    let lang = chunker::Language::from_str(language)
        .ok_or_else(|| PyValueError::new_err(format!("Unsupported language: {}", language)))?;
    chunker::extract_calls(content, &lang).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Chunk source code using tree-sitter AST analysis
#[pyfunction]
#[pyo3(signature = (content, language, max_tokens = 512, overlap_tokens = 50, file_path = None, model = None))]
fn chunk_code(
    py: Python<'_>,
    content: &str,
    language: &str,
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: Option<String>,
    model: Option<String>,
) -> PyResult<Vec<PyObject>> {
    let lang = chunker::Language::from_str(language)
        .ok_or_else(|| PyValueError::new_err(format!("Unsupported language: {}", language)))?;

    let config = chunker::ChunkConfig {
        max_tokens,
        overlap_tokens,
        file_path,
        model,
    };

    let chunks = chunker::chunk_code(content, &lang, &config)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    chunks.iter().map(|c| chunk_to_pydict(py, c)).collect()
}

/// Chunk markdown content at heading boundaries
#[pyfunction]
#[pyo3(signature = (content, max_tokens = 512, overlap_tokens = 50, file_path = None, model = None))]
fn chunk_markdown(
    py: Python<'_>,
    content: &str,
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: Option<String>,
    model: Option<String>,
) -> PyResult<Vec<PyObject>> {
    let config = chunker::ChunkConfig {
        max_tokens,
        overlap_tokens,
        file_path,
        model,
    };

    let chunks = chunker::chunk_markdown(content, &config)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    chunks.iter().map(|c| chunk_to_pydict(py, c)).collect()
}

/// Chunk plain text by paragraphs and sentences
#[pyfunction]
#[pyo3(signature = (content, max_tokens = 512, overlap_tokens = 50, file_path = None, model = None))]
fn chunk_text(
    py: Python<'_>,
    content: &str,
    max_tokens: usize,
    overlap_tokens: usize,
    file_path: Option<String>,
    model: Option<String>,
) -> PyResult<Vec<PyObject>> {
    let config = chunker::ChunkConfig {
        max_tokens,
        overlap_tokens,
        file_path,
        model,
    };

    let chunks = chunker::chunk_text(content, &config);
    chunks.iter().map(|c| chunk_to_pydict(py, c)).collect()
}

/// Auto-detect content type and chunk accordingly
#[pyfunction]
#[pyo3(signature = (content, file_path, max_tokens = 512, overlap_tokens = 50, model = None))]
fn chunk_auto(
    py: Python<'_>,
    content: &str,
    file_path: &str,
    max_tokens: usize,
    overlap_tokens: usize,
    model: Option<String>,
) -> PyResult<Vec<PyObject>> {
    let content_type = chunker::detect_content_type(file_path);
    let config = chunker::ChunkConfig {
        max_tokens,
        overlap_tokens,
        file_path: Some(file_path.to_string()),
        model,
    };

    let chunks = chunker::chunk(content, content_type, &config)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    chunks.iter().map(|c| chunk_to_pydict(py, c)).collect()
}

/// Chunk multiple files in parallel using Rayon
#[pyfunction]
#[pyo3(signature = (files, max_tokens = 512, overlap_tokens = 50, model = None))]
fn chunk_batch(
    py: Python<'_>,
    files: Vec<(String, String)>, // (file_path, content)
    max_tokens: usize,
    overlap_tokens: usize,
    model: Option<String>,
) -> PyResult<Vec<PyObject>> {
    // Process files in parallel using Rayon
    let results: Vec<Result<Vec<chunker::Chunk>, String>> = files
        .par_iter()
        .map(|(file_path, content)| {
            let content_type = chunker::detect_content_type(file_path);
            let config = chunker::ChunkConfig {
                max_tokens,
                overlap_tokens,
                file_path: Some(file_path.clone()),
                model: model.clone(),
            };

            chunker::chunk(content, content_type, &config)
                .map_err(|e| format!("{}: {}", file_path, e))
        })
        .collect();

    // Flatten results and convert to Python dicts
    let mut all_chunks = Vec::new();
    for result in results {
        match result {
            Ok(chunks) => {
                for chunk in &chunks {
                    all_chunks.push(chunk_to_pydict(py, chunk)?);
                }
            }
            Err(e) => return Err(PyRuntimeError::new_err(e)),
        }
    }

    Ok(all_chunks)
}

/// Detect content type from file path
#[pyfunction]
fn detect_content_type(file_path: &str) -> String {
    match chunker::detect_content_type(file_path) {
        chunker::ContentType::Code(lang) => format!("code:{}", lang.as_str()),
        chunker::ContentType::Markdown => "markdown".to_string(),
        chunker::ContentType::PlainText => "text".to_string(),
    }
}

/// Report the capabilities exposed by the Python binding.
#[pyfunction]
fn capabilities(py: Python<'_>) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    dict.set_item("tokenizer", true)?;
    dict.set_item("simd_json", true)?;
    dict.set_item("transport", true)?;
    dict.set_item("eventlog", true)?;
    dict.set_item("ranking", true)?;
    dict.set_item("hashing", true)?;
    dict.set_item("text", true)?;
    dict.set_item("encoding", true)?;
    dict.set_item("chunker_code", true)?;
    dict.set_item("chunker_markdown", true)?;
    dict.set_item("chunker_text", true)?;
    dict.set_item("ffi_available", true)?;
    Ok(dict.into())
}

// ============================================================================
// Module Definition
// ============================================================================

/// High-performance acceleration library for agent/MCP workflows
#[pymodule]
fn fi_accel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Tokenizer functions
    m.add_function(wrap_pyfunction!(count_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(truncate_to_tokens, m)?)?;
    m.add_function(wrap_pyfunction!(split_by_tokens, m)?)?;
    m.add_class::<Tokenizer>()?;

    // Embedding functions
    m.add_function(wrap_pyfunction!(normalize_l2, m)?)?;
    m.add_function(wrap_pyfunction!(cosine_similarity, m)?)?;
    m.add_function(wrap_pyfunction!(cosine_similarity_matrix, m)?)?;
    m.add_function(wrap_pyfunction!(top_k_similar, m)?)?;
    m.add_function(wrap_pyfunction!(compute_centroid, m)?)?;
    m.add_function(wrap_pyfunction!(prepare_embedding_batch, m)?)?;

    // Ranking functions
    m.add_function(wrap_pyfunction!(hybrid_rank, m)?)?;
    m.add_function(wrap_pyfunction!(rrf_fuse, m)?)?;

    // Diff functions
    m.add_class::<FuzzyMatcher>()?;
    m.add_function(wrap_pyfunction!(parse_unified_diff, m)?)?;

    // Log functions
    m.add_class::<LogReader>()?;

    // JSON functions
    m.add_function(wrap_pyfunction!(parse_ndjson, m)?)?;
    m.add_class::<NDJSONReader>()?;
    m.add_function(wrap_pyfunction!(parse_jsonrpc_batch, m)?)?;
    m.add_function(wrap_pyfunction!(parse_jsonrpc_lines, m)?)?;
    m.add_function(wrap_pyfunction!(serialize_jsonrpc_lines, m)?)?;
    m.add_function(wrap_pyfunction!(parse_sse_events, m)?)?;
    m.add_function(wrap_pyfunction!(extract_mcp_http_meta, m)?)?;
    m.add_function(wrap_pyfunction!(project_eventlog, m)?)?;
    m.add_function(wrap_pyfunction!(summarize_eventlog, m)?)?;

    // Hashing functions
    m.add_function(wrap_pyfunction!(sha256_batch, m)?)?;
    m.add_function(wrap_pyfunction!(verify_hash_chain, m)?)?;

    // Text / encoding functions
    m.add_function(wrap_pyfunction!(html_to_text, m)?)?;
    m.add_function(wrap_pyfunction!(detect_encoding, m)?)?;

    // TOML functions
    m.add_function(wrap_pyfunction!(parse_toml, m)?)?;
    m.add_function(wrap_pyfunction!(validate_toml, m)?)?;
    m.add_function(wrap_pyfunction!(json_to_toml, m)?)?;
    m.add_function(wrap_pyfunction!(merge_toml, m)?)?;

    // TOON functions
    m.add_function(wrap_pyfunction!(parse_toon, m)?)?;
    m.add_function(wrap_pyfunction!(to_toon, m)?)?;
    m.add_function(wrap_pyfunction!(validate_toon, m)?)?;
    m.add_function(wrap_pyfunction!(json_to_toon, m)?)?;

    // Chunker functions
    m.add_function(wrap_pyfunction!(extract_calls, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_code, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_text, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_auto, m)?)?;
    m.add_function(wrap_pyfunction!(chunk_batch, m)?)?;
    m.add_function(wrap_pyfunction!(detect_content_type, m)?)?;
    m.add_function(wrap_pyfunction!(capabilities, m)?)?;

    Ok(())
}
