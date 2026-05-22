//! Fast token counting and manipulation using tiktoken-rs.
//!
//! Gemma support is exact when the optional `gemma-tokenizer` feature is enabled
//! and a local Hugging Face `tokenizer.json` asset is configured.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use thiserror::Error;
use tiktoken_rs::{cl100k_base, o200k_base, p50k_base, p50k_edit, r50k_base, CoreBPE};

#[cfg(feature = "gemma-tokenizer")]
use {
    std::path::{Path, PathBuf},
    tokenizers::Tokenizer as HfTokenizer,
};

#[cfg(feature = "gemma-tokenizer")]
const GEMMA_TOKENIZER_ENV: &str = "FI_ACCEL_GEMMA_TOKENIZER_PATH";

#[derive(Error, Debug)]
pub enum TokenizerError {
    #[error("Unknown model: {0}")]
    UnknownModel(String),
    #[error("Failed to initialize tokenizer: {0}")]
    InitError(String),
    #[error("{0}")]
    MissingGemmaTokenizer(String),
}

/// Model to encoding mapping for tiktoken-backed models.
static MODEL_ENCODINGS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static TIKTOKEN_ENCODING_CACHE: OnceLock<Mutex<HashMap<&'static str, Arc<CoreBPE>>>> =
    OnceLock::new();

fn get_model_encodings() -> &'static HashMap<&'static str, &'static str> {
    MODEL_ENCODINGS.get_or_init(|| {
        let mut m = HashMap::new();
        // GPT-4 and GPT-3.5-turbo models
        m.insert("gpt-4", "cl100k_base");
        m.insert("gpt-4-turbo", "cl100k_base");
        m.insert("gpt-4o", "o200k_base");
        m.insert("gpt-4o-mini", "o200k_base");
        m.insert("gpt-3.5-turbo", "cl100k_base");
        // Claude models (use cl100k as approximation)
        m.insert("claude-3-opus", "cl100k_base");
        m.insert("claude-3-sonnet", "cl100k_base");
        m.insert("claude-3-haiku", "cl100k_base");
        m.insert("claude-3.5-sonnet", "cl100k_base");
        // Embeddings
        m.insert("text-embedding-ada-002", "cl100k_base");
        m.insert("text-embedding-3-small", "cl100k_base");
        m.insert("text-embedding-3-large", "cl100k_base");
        // Legacy
        m.insert("text-davinci-003", "p50k_base");
        m.insert("code-davinci-002", "p50k_base");
        m
    })
}

fn is_gemma_model(model: &str) -> bool {
    matches!(model, "gemma" | "gemma-26b" | "gemma-26b-a4b")
}

fn tiktoken_encoding_cache() -> &'static Mutex<HashMap<&'static str, Arc<CoreBPE>>> {
    TIKTOKEN_ENCODING_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn build_encoding(encoding_name: &str) -> Result<CoreBPE, TokenizerError> {
    match encoding_name {
        "cl100k_base" => cl100k_base().map_err(|e| TokenizerError::InitError(e.to_string())),
        "o200k_base" => o200k_base().map_err(|e| TokenizerError::InitError(e.to_string())),
        "p50k_base" => p50k_base().map_err(|e| TokenizerError::InitError(e.to_string())),
        "p50k_edit" => p50k_edit().map_err(|e| TokenizerError::InitError(e.to_string())),
        "r50k_base" => r50k_base().map_err(|e| TokenizerError::InitError(e.to_string())),
        _ => Err(TokenizerError::UnknownModel(encoding_name.to_string())),
    }
}

fn get_encoding(encoding_name: &'static str) -> Result<Arc<CoreBPE>, TokenizerError> {
    let cache = tiktoken_encoding_cache();
    let mut cache = cache
        .lock()
        .map_err(|_| TokenizerError::InitError("Tiktoken encoding cache lock poisoned".into()))?;

    if let Some(encoding) = cache.get(encoding_name) {
        return Ok(Arc::clone(encoding));
    }

    let encoding = Arc::new(build_encoding(encoding_name)?);
    cache.insert(encoding_name, Arc::clone(&encoding));
    Ok(encoding)
}

fn get_encoding_name_for_model(model: &str) -> &'static str {
    let encodings = get_model_encodings();
    encodings.get(model).copied().unwrap_or({
        // Default to cl100k_base for unknown models (most common)
        "cl100k_base"
    })
}

#[cfg(feature = "gemma-tokenizer")]
static GEMMA_TOKENIZER_CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<HfTokenizer>>>> = OnceLock::new();

#[cfg(feature = "gemma-tokenizer")]
fn gemma_tokenizer_cache() -> &'static Mutex<HashMap<PathBuf, Arc<HfTokenizer>>> {
    GEMMA_TOKENIZER_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(feature = "gemma-tokenizer")]
fn default_gemma_tokenizer_path() -> Option<PathBuf> {
    if let Some(cache_home) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(
            PathBuf::from(cache_home)
                .join("fi-accel")
                .join("gemma")
                .join("tokenizer.json"),
        );
    }

    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".cache")
            .join("fi-accel")
            .join("gemma")
            .join("tokenizer.json")
    })
}

#[cfg(feature = "gemma-tokenizer")]
fn configured_gemma_tokenizer_path() -> Result<PathBuf, TokenizerError> {
    if let Some(path) = std::env::var_os(GEMMA_TOKENIZER_ENV) {
        return Ok(PathBuf::from(path));
    }

    let Some(path) = default_gemma_tokenizer_path() else {
        return Err(gemma_asset_error(None));
    };

    Ok(path)
}

#[cfg(feature = "gemma-tokenizer")]
fn gemma_asset_error(path: Option<&Path>) -> TokenizerError {
    let location = path
        .map(|p| format!(" at {}", p.display()))
        .unwrap_or_default();
    TokenizerError::MissingGemmaTokenizer(format!(
        "Gemma tokenization requires a local Hugging Face tokenizer.json{location}. \
         Set {GEMMA_TOKENIZER_ENV} or place the official Gemma tokenizer at the default cache path; \
         fi-accel does not download model assets at runtime."
    ))
}

#[cfg(not(feature = "gemma-tokenizer"))]
fn missing_gemma_feature_error() -> TokenizerError {
    TokenizerError::MissingGemmaTokenizer(
        "Gemma tokenization requires the optional `gemma-tokenizer` feature and a local Hugging Face tokenizer.json; fi-accel no longer approximates Gemma with cl100k_base.".to_string(),
    )
}

#[cfg(feature = "gemma-tokenizer")]
fn load_gemma_tokenizer(path: &Path) -> Result<Arc<HfTokenizer>, TokenizerError> {
    if !path.is_file() {
        return Err(gemma_asset_error(Some(path)));
    }

    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let cache = gemma_tokenizer_cache();
    let mut cache = cache
        .lock()
        .map_err(|_| TokenizerError::InitError("Gemma tokenizer cache lock poisoned".into()))?;

    if let Some(tokenizer) = cache.get(&canonical_path) {
        return Ok(Arc::clone(tokenizer));
    }

    let tokenizer = HfTokenizer::from_file(&canonical_path)
        .map_err(|e| TokenizerError::InitError(e.to_string()))?;
    let tokenizer = Arc::new(tokenizer);
    cache.insert(canonical_path, Arc::clone(&tokenizer));
    Ok(tokenizer)
}

enum TokenizerBackend {
    Tiktoken(Arc<CoreBPE>),
    #[cfg(feature = "gemma-tokenizer")]
    Gemma(Arc<HfTokenizer>),
}

impl TokenizerBackend {
    fn count(&self, text: &str) -> usize {
        match self {
            Self::Tiktoken(bpe) => bpe.encode_ordinary(text).len(),
            #[cfg(feature = "gemma-tokenizer")]
            Self::Gemma(tokenizer) => tokenizer
                .encode(text, false)
                .map(|encoding| encoding.len())
                .unwrap_or(0),
        }
    }

    fn encode(&self, text: &str) -> Vec<u32> {
        match self {
            Self::Tiktoken(bpe) => bpe.encode_ordinary(text),
            #[cfg(feature = "gemma-tokenizer")]
            Self::Gemma(tokenizer) => tokenizer
                .encode(text, false)
                .map(|encoding| encoding.get_ids().to_vec())
                .unwrap_or_default(),
        }
    }

    fn decode(&self, tokens: &[u32]) -> Result<String, TokenizerError> {
        match self {
            Self::Tiktoken(bpe) => bpe
                .decode(tokens.to_vec())
                .map_err(|e| TokenizerError::InitError(e.to_string())),
            #[cfg(feature = "gemma-tokenizer")]
            Self::Gemma(tokenizer) => tokenizer
                .decode(tokens, false)
                .map_err(|e| TokenizerError::InitError(e.to_string())),
        }
    }
}

/// Tokenizer wrapper with cached tokenizer instance
pub struct Tokenizer {
    backend: TokenizerBackend,
    model: String,
}

impl Tokenizer {
    /// Create a new tokenizer for the given model.
    ///
    /// Gemma models require the `gemma-tokenizer` feature and a local
    /// `tokenizer.json` configured through `FI_ACCEL_GEMMA_TOKENIZER_PATH` or
    /// the default cache path.
    pub fn new(model: &str) -> Result<Self, TokenizerError> {
        if is_gemma_model(model) {
            #[cfg(feature = "gemma-tokenizer")]
            {
                return Self::new_gemma_with_tokenizer_path(
                    model,
                    configured_gemma_tokenizer_path()?,
                );
            }

            #[cfg(not(feature = "gemma-tokenizer"))]
            {
                return Err(missing_gemma_feature_error());
            }
        }

        let bpe = get_encoding(get_encoding_name_for_model(model))?;
        Ok(Self {
            backend: TokenizerBackend::Tiktoken(bpe),
            model: model.to_string(),
        })
    }

    /// Create a Gemma tokenizer using an explicit local Hugging Face tokenizer asset.
    #[cfg(feature = "gemma-tokenizer")]
    pub fn new_gemma_with_tokenizer_path(
        model: &str,
        tokenizer_path: impl AsRef<Path>,
    ) -> Result<Self, TokenizerError> {
        if !is_gemma_model(model) {
            return Err(TokenizerError::UnknownModel(model.to_string()));
        }

        let tokenizer = load_gemma_tokenizer(tokenizer_path.as_ref())?;
        Ok(Self {
            backend: TokenizerBackend::Gemma(tokenizer),
            model: model.to_string(),
        })
    }

    /// Count tokens in text
    pub fn count(&self, text: &str) -> usize {
        self.backend.count(text)
    }

    /// Encode text to token IDs
    pub fn encode(&self, text: &str) -> Vec<u32> {
        self.backend.encode(text)
    }

    /// Decode token IDs to text
    pub fn decode(&self, tokens: &[u32]) -> Result<String, TokenizerError> {
        self.backend.decode(tokens)
    }

    /// Truncate text to fit within max_tokens
    pub fn truncate(&self, text: &str, max_tokens: usize) -> String {
        let tokens = self.backend.encode(text);
        if tokens.len() <= max_tokens {
            return text.to_string();
        }

        let truncated = &tokens[..max_tokens];
        self.backend
            .decode(truncated)
            .unwrap_or_else(|_| text[..text.len().min(max_tokens * 4)].to_string())
    }

    /// Split text into chunks of approximately chunk_size tokens with overlap
    pub fn split(&self, text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
        if chunk_size == 0 {
            return vec![];
        }

        let tokens = self.backend.encode(text);
        if tokens.is_empty() {
            return vec![];
        }

        let mut chunks = Vec::new();
        let step = chunk_size.saturating_sub(overlap).max(1);
        let mut start = 0;

        while start < tokens.len() {
            let end = (start + chunk_size).min(tokens.len());
            let chunk_tokens = &tokens[start..end];

            if let Ok(chunk_text) = self.backend.decode(chunk_tokens) {
                chunks.push(chunk_text);
            }

            if end >= tokens.len() {
                break;
            }
            start += step;
        }

        chunks
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.model
    }
}

// Convenience functions that create tokenizer on each call
// For performance, prefer creating a Tokenizer instance and reusing it

/// Count tokens in text for a given model
pub fn count_tokens(text: &str, model: &str) -> Result<usize, TokenizerError> {
    let tokenizer = Tokenizer::new(model)?;
    Ok(tokenizer.count(text))
}

/// Truncate text to max_tokens for a given model
pub fn truncate_to_tokens(
    text: &str,
    max_tokens: usize,
    model: &str,
) -> Result<String, TokenizerError> {
    let tokenizer = Tokenizer::new(model)?;
    Ok(tokenizer.truncate(text, max_tokens))
}

/// Split text into chunks for a given model
pub fn split_by_tokens(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    model: &str,
) -> Result<Vec<String>, TokenizerError> {
    let tokenizer = Tokenizer::new(model)?;
    Ok(tokenizer.split(text, chunk_size, overlap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens() {
        let tokenizer = Tokenizer::new("gpt-4").unwrap();
        let count = tokenizer.count("Hello, world!");
        assert!(count > 0);
        assert!(count < 10); // Should be around 4 tokens
    }

    #[test]
    fn test_truncate() {
        let tokenizer = Tokenizer::new("gpt-4").unwrap();
        let text = "This is a longer text that should be truncated to fit within the limit.";
        let truncated = tokenizer.truncate(text, 5);
        let truncated_count = tokenizer.count(&truncated);
        assert!(truncated_count <= 5);
    }

    #[test]
    fn test_split() {
        let tokenizer = Tokenizer::new("gpt-4").unwrap();
        let text = "One two three four five six seven eight nine ten eleven twelve.";
        let chunks = tokenizer.split(text, 3, 1);
        assert!(!chunks.is_empty());

        // Each chunk should have roughly 3 tokens (with some variation due to BPE)
        for chunk in &chunks {
            let count = tokenizer.count(chunk);
            assert!(count <= 4); // Allow some flexibility
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let tokenizer = Tokenizer::new("gpt-4").unwrap();
        let text = "Hello, world!";
        let tokens: Vec<u32> = tokenizer.encode(text);
        let decoded = tokenizer.decode(&tokens).unwrap();
        assert_eq!(text, decoded);
    }

    #[test]
    fn test_unknown_model_defaults() {
        // Unknown models should default to cl100k_base
        let result = Tokenizer::new("unknown-model-xyz");
        assert!(result.is_ok());
    }

    #[test]
    fn test_gemma_model_names_are_recognized() {
        for model in ["gemma", "gemma-26b", "gemma-26b-a4b"] {
            assert!(is_gemma_model(model));
        }
    }

    #[test]
    #[cfg(not(feature = "gemma-tokenizer"))]
    fn test_gemma_models_require_feature() {
        for model in ["gemma", "gemma-26b", "gemma-26b-a4b"] {
            let err = match Tokenizer::new(model) {
                Ok(_) => panic!("{model} unexpectedly initialized without gemma-tokenizer"),
                Err(err) => err.to_string(),
            };
            assert!(err.contains("gemma-tokenizer"));
            assert!(err.contains("no longer approximates Gemma"));
        }
    }

    #[test]
    #[cfg(feature = "gemma-tokenizer")]
    fn test_gemma_models_require_local_asset() {
        let previous = std::env::var_os(GEMMA_TOKENIZER_ENV);
        std::env::set_var(
            GEMMA_TOKENIZER_ENV,
            "/path/that/does/not/exist/tokenizer.json",
        );

        let err = match Tokenizer::new("gemma") {
            Ok(_) => panic!("gemma unexpectedly initialized without a local tokenizer asset"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("requires a local Hugging Face tokenizer.json"));
        assert!(err.contains(GEMMA_TOKENIZER_ENV));

        if let Some(previous) = previous {
            std::env::set_var(GEMMA_TOKENIZER_ENV, previous);
        } else {
            std::env::remove_var(GEMMA_TOKENIZER_ENV);
        }
    }

    #[test]
    #[cfg(feature = "gemma-tokenizer")]
    fn test_gemma_fixture_counts_match_hf_tokenizer_asset() {
        let Some(tokenizer_path) = std::env::var_os(GEMMA_TOKENIZER_ENV) else {
            eprintln!("skipping Gemma exactness fixture; {GEMMA_TOKENIZER_ENV} is not set");
            return;
        };

        let tokenizer_path = PathBuf::from(tokenizer_path);
        assert!(
            tokenizer_path.is_file(),
            "{GEMMA_TOKENIZER_ENV} must point at a local Hugging Face tokenizer.json"
        );

        let reference = HfTokenizer::from_file(&tokenizer_path)
            .unwrap_or_else(|err| panic!("failed to load reference Gemma tokenizer: {err}"));
        let tokenizer = Tokenizer::new("gemma")
            .unwrap_or_else(|err| panic!("failed to create fi-accel Gemma tokenizer: {err}"));
        let fixture = gemma_exactness_fixture();
        assert_eq!(fixture.len(), 100);

        for text in fixture {
            let expected = reference
                .encode(text.as_str(), false)
                .unwrap_or_else(|err| panic!("reference tokenizer failed for {text:?}: {err}"))
                .len();
            let actual = tokenizer.count(&text);
            assert_eq!(actual, expected, "token count mismatch for {text:?}");
            assert_eq!(tokenizer.encode(&text).len(), expected);
        }
    }

    #[cfg(feature = "gemma-tokenizer")]
    fn gemma_exactness_fixture() -> Vec<String> {
        let seeds = [
            "Hello, Gemma.",
            "Summarize the deployment plan in three bullets.",
            "tools/call mcp.gateway.route_preview payload",
            "fn main() { println!(\"hi\"); }",
            "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"id\":\"abc-123\"}",
            "Path: /Users/cblevins/workspace/libs/fi-accel/src/lib.rs",
            "The quick brown fox jumps over thirteen lazy logs.",
            "Rank documents by BM25, vector score, and recency.",
            "Email subject: Re: quarterly invoice attachment review",
            "Chunk markdown headings without losing source offsets.",
            "Detect UTF-8, Windows-1252, and ISO-8859-1 inputs.",
            "A short sentence.",
            "Whitespace     should    remain     tokenizer-visible.",
            "Line one\nLine two\nLine three",
            "snake_case camelCase PascalCase kebab-case",
            "2026-05-20T19:30:00-04:00",
            "SELECT id, name FROM tools WHERE enabled = true;",
            "Patch @@ -10,7 +10,8 @@ with context lines.",
            "alpha beta gamma delta epsilon zeta eta theta",
            "When fallback mode is active, report capabilities honestly.",
        ];

        let mut fixture = Vec::with_capacity(100);
        for round in 0..5 {
            for (index, seed) in seeds.iter().enumerate() {
                fixture.push(format!("[case {round:02}-{index:02}] {seed}"));
            }
        }
        fixture
    }

    #[test]
    fn test_existing_gpt_and_claude_model_mappings() {
        let expected = [
            ("gpt-4", "cl100k_base"),
            ("gpt-4-turbo", "cl100k_base"),
            ("gpt-4o", "o200k_base"),
            ("gpt-4o-mini", "o200k_base"),
            ("gpt-3.5-turbo", "cl100k_base"),
            ("claude-3-opus", "cl100k_base"),
            ("claude-3-sonnet", "cl100k_base"),
            ("claude-3-haiku", "cl100k_base"),
            ("claude-3.5-sonnet", "cl100k_base"),
        ];

        for (model, encoding) in expected {
            assert_eq!(get_encoding_name_for_model(model), encoding);
            assert!(Tokenizer::new(model).is_ok());
        }
    }
}
