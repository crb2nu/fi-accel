//! Syntax-aware chunking for code, markdown, and plain text
//!
//! Provides intelligent content splitting that respects structural boundaries:
//! - **Code**: Tree-sitter AST-aware chunking (functions, classes, methods)
//! - **Markdown**: Heading-boundary chunking with section awareness
//! - **Text**: Paragraph and sentence-based chunking
//!
//! Chunks include SHA-256 IDs for deduplication and optional exact token counts
//! when the `tokenizer` feature is also enabled.

mod language;
mod markdown;
mod text;

#[cfg(feature = "tree-sitter")]
mod code;
#[cfg(feature = "tree-sitter")]
mod parser;

pub use language::{detect_content_type, ContentType, Language};
pub use markdown::chunk_markdown;
pub use text::chunk_text;

#[cfg(feature = "tree-sitter")]
pub use code::chunk_code;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChunkerError {
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("Content too large: {0} bytes")]
    ContentTooLarge(usize),
}

/// The kind of chunk extracted
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkKind {
    Function,
    Method,
    Class,
    Module,
    Import,
    Block,
    Section,
    Paragraph,
    Sentence,
}

impl ChunkKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Module => "module",
            Self::Import => "import",
            Self::Block => "block",
            Self::Section => "section",
            Self::Paragraph => "paragraph",
            Self::Sentence => "sentence",
        }
    }
}

/// Configuration for chunking operations
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum tokens per chunk (default: 512)
    pub max_tokens: usize,
    /// Overlap tokens between adjacent chunks (default: 50)
    pub overlap_tokens: usize,
    /// Include file path in chunk ID hashing
    pub file_path: Option<String>,
    /// Model name for token counting (requires `tokenizer` feature)
    pub model: Option<String>,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 50,
            file_path: None,
            model: None,
        }
    }
}

/// A single chunk of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique ID (SHA-256 of content + location)
    pub id: String,
    /// The chunk content
    pub content: String,
    /// Kind of chunk
    pub kind: ChunkKind,
    /// Language of the source content
    pub language: String,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// Estimated or exact token count
    pub token_count: usize,
    /// Optional name (function name, class name, section heading)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional parent name (class name for methods)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
    /// Optional function signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Optional docstring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docstring: Option<String>,
    /// Optional file-level import summary for indexing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<String>>,
    /// Optional semantic symbol kind for indexing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<String>,
    /// Optional enclosing container name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    /// Optional enclosing container kind
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_kind: Option<String>,
    /// Optional fully-qualified symbol path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_path: Option<String>,
}

pub(crate) fn symbol_path(name: Option<&str>, container_name: Option<&str>) -> Option<String> {
    match (container_name, name) {
        (Some(container), Some(name)) => Some(format!("{}.{}", container, name)),
        (None, Some(name)) => Some(name.to_string()),
        _ => None,
    }
}

/// Generate a unique chunk ID from file path, line, and content
pub(crate) fn generate_chunk_id(file_path: Option<&str>, line: usize, content: &str) -> String {
    let path = file_path.unwrap_or("<unknown>");
    let prefix = &content[..content.len().min(100)];
    let data = format!("{}:{}:{}", path, line, prefix);

    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();

    hex::encode(&result[..8])
}

/// Estimate tokens using 4-chars-per-token heuristic
pub(crate) fn estimate_tokens(content: &str) -> usize {
    content.len().div_ceil(4)
}

/// Count tokens exactly if the tokenizer feature is enabled, otherwise estimate
pub(crate) fn count_or_estimate_tokens(content: &str, _model: Option<&str>) -> usize {
    #[cfg(feature = "tokenizer")]
    {
        if let Some(model) = _model {
            if let Ok(tokenizer) = crate::tokenizer::Tokenizer::new(model) {
                return tokenizer.count(content);
            }
        }
    }
    estimate_tokens(content)
}

/// Auto-detect content type and chunk accordingly
pub fn chunk(
    content: &str,
    content_type: ContentType,
    config: &ChunkConfig,
) -> Result<Vec<Chunk>, ChunkerError> {
    match content_type {
        ContentType::Markdown => chunk_markdown(content, config),
        ContentType::PlainText => Ok(chunk_text(content, config)),
        #[cfg(feature = "tree-sitter")]
        ContentType::Code(lang) => chunk_code(content, &lang, config),
        #[cfg(not(feature = "tree-sitter"))]
        ContentType::Code(_) => Err(ChunkerError::UnsupportedLanguage(
            "tree-sitter feature not enabled".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_chunk_id_deterministic() {
        let id1 = generate_chunk_id(Some("test.py"), 10, "def foo(): pass");
        let id2 = generate_chunk_id(Some("test.py"), 10, "def foo(): pass");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 16);
    }

    #[test]
    fn test_generate_chunk_id_different_line() {
        let id1 = generate_chunk_id(Some("test.py"), 10, "def foo(): pass");
        let id2 = generate_chunk_id(Some("test.py"), 11, "def foo(): pass");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn test_chunk_kind_as_str() {
        assert_eq!(ChunkKind::Function.as_str(), "function");
        assert_eq!(ChunkKind::Section.as_str(), "section");
        assert_eq!(ChunkKind::Paragraph.as_str(), "paragraph");
    }

    #[test]
    fn test_default_config() {
        let config = ChunkConfig::default();
        assert_eq!(config.max_tokens, 512);
        assert_eq!(config.overlap_tokens, 50);
        assert!(config.file_path.is_none());
    }
}
