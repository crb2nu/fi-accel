//! AST-aware code chunking using tree-sitter
//!
//! Walks the syntax tree and extracts semantic chunks (functions, classes, methods).
//! Ported from py-codebase-memory/rust/src/chunker.rs with adaptations for fi-accel.

use tree_sitter::Node;

use super::language::Language;
use super::parser::{
    class_body_kind, extract_calls_from_node, extract_imports, get_docstring,
    get_function_signature, get_node_name, is_class_def, is_function_def, node_text, parse_code,
};
use super::{
    count_or_estimate_tokens, generate_chunk_id, symbol_path, Chunk, ChunkConfig, ChunkKind,
    ChunkerError,
};

/// Chunk source code using tree-sitter AST analysis.
///
/// Extracts functions, classes, and methods as semantic chunks. Falls back to
/// block-based splitting for content that doesn't fit into semantic categories.
pub fn chunk_code(
    content: &str,
    lang: &Language,
    config: &ChunkConfig,
) -> Result<Vec<Chunk>, ChunkerError> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let tree = parse_code(content, lang)?;
    let imports = extract_imports(content, lang).unwrap_or_default();
    let mut chunks = Vec::new();

    walk_tree(
        tree.root_node(),
        content,
        lang,
        config,
        &imports,
        &mut chunks,
        None,
    );

    // py-codebase-memory's PythonIndexer only emits semantic class/function
    // chunks. Preserve that contract for Python so accelerated chunk IDs and
    // cardinality line up with existing downstream indexes.
    if chunks.is_empty() && matches!(lang, Language::Python) {
        return Ok(chunks);
    }

    // If AST walking produced no chunks for other languages, fall back to
    // treating the file as a single module block.
    if chunks.is_empty() && !content.trim().is_empty() {
        let token_count = count_or_estimate_tokens(content, config.model.as_deref());
        chunks.push(Chunk {
            id: generate_chunk_id(config.file_path.as_deref(), 0, content),
            content: content.to_string(),
            kind: ChunkKind::Module,
            chunk_type: ChunkKind::Module.as_str().to_string(),
            language: lang.as_str().to_string(),
            file_path: config.file_path.clone(),
            start_line: 1,
            end_line: content.matches('\n').count() + 1,
            start_column: 0,
            end_column: 0,
            token_count,
            name: None,
            parent_name: None,
            signature: None,
            docstring: None,
            imports: Some(imports),
            parent_type: None,
            symbol_kind: Some(ChunkKind::Module.as_str().to_string()),
            container_name: None,
            container_kind: None,
            symbol_path: None,
            calls: extract_calls_from_node(tree.root_node(), content, lang),
            definitions: Vec::new(),
        });
    }

    Ok(chunks)
}

/// Recursively walk the AST and extract chunks
fn walk_tree(
    node: Node,
    content: &str,
    lang: &Language,
    config: &ChunkConfig,
    imports: &[String],
    chunks: &mut Vec<Chunk>,
    parent_name: Option<&str>,
) {
    // Handle function definitions
    if is_function_def(node, lang) {
        let chunk = extract_function(node, content, lang, config, imports, parent_name);
        chunks.push(chunk);
        return; // Don't recurse into function body
    }

    // Handle class definitions
    if is_class_def(node, lang) {
        let class_chunk = extract_class(node, content, lang, config);
        let class_name = get_node_name(node, content);
        chunks.push(class_chunk);

        // Extract methods from class body
        extract_class_methods(
            node,
            content,
            lang,
            config,
            imports,
            class_name.as_deref(),
            chunks,
        );
        return;
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tree(child, content, lang, config, imports, chunks, parent_name);
    }
}

/// Extract a function/method as a chunk
fn extract_function(
    node: Node,
    content: &str,
    lang: &Language,
    config: &ChunkConfig,
    imports: &[String],
    parent_name: Option<&str>,
) -> Chunk {
    let name = get_node_name(node, content);
    let code = node_text(node, content);
    let signature = get_function_signature(node, content, lang);
    let docstring = get_docstring(node, content, lang);

    let kind = if parent_name.is_some() {
        ChunkKind::Method
    } else {
        ChunkKind::Function
    };
    let chunk_type = kind.as_str().to_string();
    let symbol_kind = kind.as_str().to_string();
    let symbol_path = symbol_path(name.as_deref(), parent_name);
    let calls = extract_calls_from_node(node, content, lang);
    let definitions = name.clone().map(|n| vec![n]).unwrap_or_default();

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let start_column = node.start_position().column;
    let end_column = node.end_position().column;
    let token_count = count_or_estimate_tokens(code, config.model.as_deref());

    Chunk {
        id: generate_chunk_id(config.file_path.as_deref(), node.start_position().row, code),
        content: code.to_string(),
        kind,
        chunk_type,
        language: lang.as_str().to_string(),
        file_path: config.file_path.clone(),
        start_line,
        end_line,
        start_column,
        end_column,
        token_count,
        name,
        parent_name: parent_name.map(|s| s.to_string()),
        signature,
        docstring,
        imports: Some(imports.to_vec()),
        parent_type: parent_name.map(|_| ChunkKind::Class.as_str().to_string()),
        symbol_kind: Some(symbol_kind),
        container_name: parent_name.map(|s| s.to_string()),
        container_kind: parent_name.map(|_| ChunkKind::Class.as_str().to_string()),
        symbol_path,
        calls,
        definitions,
    }
}

/// Extract a class definition as a chunk (definition only, not methods)
fn extract_class(node: Node, content: &str, lang: &Language, config: &ChunkConfig) -> Chunk {
    let name = get_node_name(node, content);

    // Get class definition up to first method
    let class_start = node.start_byte();
    let mut class_end = node.end_byte();

    // Find the first function in the class body and truncate there
    find_first_method(node, lang, |method_node| {
        class_end = method_node.start_byte();
    });

    let code = content[class_start..class_end].trim();
    let docstring = get_docstring(node, content, lang);
    let symbol_path = symbol_path(name.as_deref(), None);
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let token_count = count_or_estimate_tokens(code, config.model.as_deref());

    Chunk {
        id: generate_chunk_id(config.file_path.as_deref(), node.start_position().row, code),
        content: code.to_string(),
        kind: ChunkKind::Class,
        chunk_type: ChunkKind::Class.as_str().to_string(),
        language: lang.as_str().to_string(),
        file_path: config.file_path.clone(),
        start_line,
        end_line,
        start_column: node.start_position().column,
        end_column: node.end_position().column,
        token_count,
        name: name.clone(),
        parent_name: None,
        signature: None,
        docstring,
        imports: None,
        parent_type: None,
        symbol_kind: Some(ChunkKind::Class.as_str().to_string()),
        container_name: None,
        container_kind: Some(ChunkKind::Module.as_str().to_string()),
        symbol_path,
        calls: Vec::new(),
        definitions: name.map(|n| vec![n]).unwrap_or_default(),
    }
}

/// Find the first method in a class body
fn find_first_method<F>(node: Node, lang: &Language, mut callback: F)
where
    F: FnMut(Node),
{
    let body_kind = class_body_kind(lang);
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == body_kind {
            let mut body_cursor = child.walk();
            for body_child in child.children(&mut body_cursor) {
                if is_function_def(body_child, lang) {
                    callback(body_child);
                    return;
                }
            }
        }
    }
}

/// Extract methods from a class body
fn extract_class_methods(
    node: Node,
    content: &str,
    lang: &Language,
    config: &ChunkConfig,
    imports: &[String],
    class_name: Option<&str>,
    chunks: &mut Vec<Chunk>,
) {
    let body_kind = class_body_kind(lang);
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == body_kind {
            let mut body_cursor = child.walk();
            for body_child in child.children(&mut body_cursor) {
                if is_function_def(body_child, lang) {
                    let method_chunk =
                        extract_function(body_child, content, lang, config, imports, class_name);
                    chunks.push(method_chunk);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_code_python_function() {
        let content = r#"
import os

def hello(name: str) -> str:
    """Say hello."""
    return f"Hello, {name}!"
"#;
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Python, &config).unwrap();

        let func = chunks.iter().find(|c| c.name.as_deref() == Some("hello"));
        assert!(func.is_some(), "Should find hello function");
        let f = func.unwrap();
        assert_eq!(f.kind, ChunkKind::Function);
        assert_eq!(f.chunk_type, "function");
        assert_eq!(f.start_column, 0);
        assert_eq!(f.symbol_kind.as_deref(), Some("function"));
        assert_eq!(f.container_kind, None);
        assert_eq!(f.symbol_path.as_deref(), Some("hello"));
        assert_eq!(f.definitions, vec!["hello".to_string()]);
        assert!(f
            .docstring
            .as_ref()
            .is_some_and(|d| d.contains("Say hello")));
    }

    #[test]
    fn test_chunk_code_python_class() {
        let content = r#"
class Greeter:
    """A greeter."""

    def __init__(self, prefix: str):
        self.prefix = prefix

    def greet(self, name: str) -> str:
        return f"{self.prefix} {name}"
"#;
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Python, &config).unwrap();

        let class = chunks.iter().find(|c| c.kind == ChunkKind::Class);
        assert!(class.is_some(), "Should find class");
        let class = class.unwrap();
        assert_eq!(class.name.as_deref(), Some("Greeter"));
        assert_eq!(class.chunk_type, "class");
        assert_eq!(class.definitions, vec!["Greeter".to_string()]);

        let methods: Vec<_> = chunks
            .iter()
            .filter(|c| c.kind == ChunkKind::Method)
            .collect();
        assert_eq!(methods.len(), 2);

        for method in &methods {
            assert_eq!(method.parent_name.as_deref(), Some("Greeter"));
            assert_eq!(method.parent_type.as_deref(), Some("class"));
            assert_eq!(method.container_name.as_deref(), Some("Greeter"));
            assert_eq!(method.container_kind.as_deref(), Some("class"));
            assert!(method
                .symbol_path
                .as_deref()
                .is_some_and(|path| path.starts_with("Greeter.")));
        }
    }

    #[test]
    fn test_chunk_code_emits_file_location_and_calls() {
        let content = r#"
def helper(name: str) -> str:
    return name.upper()

def hello(name: str) -> str:
    return helper(name)
"#;
        let config = ChunkConfig {
            file_path: Some("demo.py".to_string()),
            ..ChunkConfig::default()
        };

        let chunks = chunk_code(content, &Language::Python, &config).unwrap();
        let func = chunks
            .iter()
            .find(|c| c.name.as_deref() == Some("hello"))
            .unwrap();

        assert_eq!(func.file_path.as_deref(), Some("demo.py"));
        assert_eq!(func.calls, vec!["helper".to_string()]);
        assert_eq!(func.definitions, vec!["hello".to_string()]);
        assert!(func.end_column > 0);
    }

    #[test]
    fn test_chunk_code_go() {
        let content = r#"
package main

import "fmt"

// SayHello greets someone
func SayHello(name string) string {
    return fmt.Sprintf("Hello, %s!", name)
}
"#;
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Go, &config).unwrap();

        let func = chunks
            .iter()
            .find(|c| c.name.as_deref() == Some("SayHello"));
        assert!(func.is_some());
        assert_eq!(func.unwrap().kind, ChunkKind::Function);
        assert!(func
            .unwrap()
            .imports
            .as_ref()
            .is_some_and(|imports| !imports.is_empty()));
    }

    #[test]
    fn test_chunk_code_rust() {
        let content = r#"
use std::fmt;

/// Greet someone by name
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

struct Greeter {
    prefix: String,
}

impl Greeter {
    fn new(prefix: &str) -> Self {
        Self { prefix: prefix.to_string() }
    }
}
"#;
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Rust, &config).unwrap();

        let func = chunks.iter().find(|c| c.name.as_deref() == Some("greet"));
        assert!(func.is_some());
        assert!(func.unwrap().docstring.is_some());
    }

    #[test]
    fn test_chunk_code_includes_call_edges() {
        let content = r#"
def hello(name):
    message = format_name(name)
    logger.info(message)
    return emit(message)
"#;
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Python, &config).unwrap();

        let func = chunks
            .iter()
            .find(|c| c.name.as_deref() == Some("hello"))
            .unwrap();
        assert_eq!(
            func.calls,
            vec![
                "emit".to_string(),
                "format_name".to_string(),
                "logger.info".to_string(),
            ]
        );
    }

    #[test]
    fn test_chunk_code_empty() {
        let config = ChunkConfig::default();
        let chunks = chunk_code("", &Language::Python, &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_code_only_imports() {
        let content = "import os\nimport sys\n";
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Python, &config).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_code_unique_ids() {
        let content = r#"
def foo():
    pass

def bar():
    pass
"#;
        let config = ChunkConfig::default();
        let chunks = chunk_code(content, &Language::Python, &config).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_ne!(chunks[0].id, chunks[1].id);
    }
}
