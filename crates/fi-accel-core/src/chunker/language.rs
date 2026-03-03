//! Language detection and content type classification

use std::path::Path;

/// Supported programming languages for AST-aware chunking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Python,
    TypeScript,
    JavaScript,
    Go,
    Rust,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Go => "go",
            Self::Rust => "rust",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Some(Self::Python),
            "typescript" | "ts" => Some(Self::TypeScript),
            "javascript" | "js" => Some(Self::JavaScript),
            "go" | "golang" => Some(Self::Go),
            "rust" | "rs" => Some(Self::Rust),
            _ => None,
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" | "pyi" | "pyw" => Some(Self::Python),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "go" => Some(Self::Go),
            "rs" => Some(Self::Rust),
            _ => None,
        }
    }
}

/// Content type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Code(Language),
    Markdown,
    PlainText,
}

/// Detect content type from a file path
pub fn detect_content_type(file_path: &str) -> ContentType {
    let path = Path::new(file_path);

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();

        // Check for markdown
        if matches!(ext_lower.as_str(), "md" | "mdx" | "markdown") {
            return ContentType::Markdown;
        }

        // Check for known code languages
        if let Some(lang) = Language::from_extension(&ext_lower) {
            return ContentType::Code(lang);
        }

        // Check for plain text
        if matches!(ext_lower.as_str(), "txt" | "text" | "log" | "csv") {
            return ContentType::PlainText;
        }
    }

    // Check for common filenames without extension
    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
        let lower = filename.to_lowercase();
        if matches!(
            lower.as_str(),
            "readme" | "changelog" | "license" | "contributing" | "authors"
        ) {
            return ContentType::PlainText;
        }
        if lower.ends_with(".md") {
            return ContentType::Markdown;
        }
    }

    ContentType::PlainText
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("unknown"), None);
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("python"), Some(Language::Python));
        assert_eq!(Language::from_str("Python"), Some(Language::Python));
        assert_eq!(Language::from_str("golang"), Some(Language::Go));
        assert_eq!(Language::from_str("nope"), None);
    }

    #[test]
    fn test_detect_content_type() {
        assert_eq!(
            detect_content_type("src/main.py"),
            ContentType::Code(Language::Python)
        );
        assert_eq!(detect_content_type("README.md"), ContentType::Markdown);
        assert_eq!(detect_content_type("notes.txt"), ContentType::PlainText);
        assert_eq!(
            detect_content_type("src/lib.rs"),
            ContentType::Code(Language::Rust)
        );
    }

    #[test]
    fn test_detect_content_type_case_insensitive() {
        assert_eq!(detect_content_type("DOC.MD"), ContentType::Markdown);
        assert_eq!(
            detect_content_type("main.PY"),
            ContentType::Code(Language::Python)
        );
    }

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::Go.as_str(), "go");
        assert_eq!(Language::Rust.as_str(), "rust");
    }
}
