//! Tree-sitter parser setup and AST utility functions
//!
//! Ported from py-codebase-memory/rust/src/parser.rs

use super::language::Language;
use super::ChunkerError;
use tree_sitter::{Node, Parser, Tree};

/// Create a tree-sitter parser configured for the given language
pub fn create_parser(lang: &Language) -> Result<Parser, ChunkerError> {
    let mut parser = Parser::new();
    let language = get_language(lang)?;
    parser.set_language(&language).map_err(|e| ChunkerError::ParseError {
        line: 0,
        message: format!("Failed to set language: {}", e),
    })?;
    Ok(parser)
}

/// Parse source code into a tree-sitter AST
pub fn parse_code(content: &str, lang: &Language) -> Result<Tree, ChunkerError> {
    let mut parser = create_parser(lang)?;
    parser.parse(content, None).ok_or_else(|| ChunkerError::ParseError {
        line: 0,
        message: "Tree-sitter parse returned None".to_string(),
    })
}

/// Get the tree-sitter Language for a given language enum
fn get_language(lang: &Language) -> Result<tree_sitter::Language, ChunkerError> {
    match lang {
        Language::Python => Ok(tree_sitter_python::LANGUAGE.into()),
        Language::TypeScript => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        Language::JavaScript => Ok(tree_sitter_javascript::LANGUAGE.into()),
        Language::Go => Ok(tree_sitter_go::LANGUAGE.into()),
        Language::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
    }
}

/// Extract text content from a tree-sitter node
pub fn node_text<'a>(node: Node<'a>, content: &'a str) -> &'a str {
    &content[node.start_byte()..node.end_byte()]
}

/// Check if a node is a function definition
pub fn is_function_def(node: Node, lang: &Language) -> bool {
    match lang {
        Language::Python => node.kind() == "function_definition",
        Language::TypeScript | Language::JavaScript => {
            matches!(
                node.kind(),
                "function_declaration"
                    | "method_definition"
                    | "arrow_function"
                    | "function"
            )
        }
        Language::Go => node.kind() == "function_declaration" || node.kind() == "method_declaration",
        Language::Rust => node.kind() == "function_item",
    }
}

/// Check if a node is a class/struct/interface definition
pub fn is_class_def(node: Node, lang: &Language) -> bool {
    match lang {
        Language::Python => node.kind() == "class_definition",
        Language::TypeScript | Language::JavaScript => {
            matches!(node.kind(), "class_declaration" | "interface_declaration")
        }
        Language::Go => node.kind() == "type_declaration",
        Language::Rust => {
            matches!(node.kind(), "struct_item" | "enum_item" | "impl_item" | "trait_item")
        }
    }
}

/// Extract the name of a node (function, class, etc.)
pub fn get_node_name<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| node_text(n, content).to_string())
}

/// Extract function signature (params + return type)
pub fn get_function_signature(node: Node, content: &str, lang: &Language) -> Option<String> {
    match lang {
        Language::Python => {
            let name = get_node_name(node, content)?;
            let params = node
                .child_by_field_name("parameters")
                .map(|n| node_text(n, content))
                .unwrap_or("()");
            let return_type = node
                .child_by_field_name("return_type")
                .map(|n| format!(" -> {}", node_text(n, content)))
                .unwrap_or_default();
            Some(format!("def {}{}{}", name, params, return_type))
        }
        Language::TypeScript | Language::JavaScript => {
            let name = get_node_name(node, content)?;
            let params = node
                .child_by_field_name("parameters")
                .map(|n| node_text(n, content))
                .unwrap_or("()");
            Some(format!("function {}{}", name, params))
        }
        Language::Go => {
            let name = get_node_name(node, content)?;
            let params = node
                .child_by_field_name("parameters")
                .map(|n| node_text(n, content))
                .unwrap_or("()");
            let result = node
                .child_by_field_name("result")
                .map(|n| format!(" {}", node_text(n, content)))
                .unwrap_or_default();
            Some(format!("func {}{}{}", name, params, result))
        }
        Language::Rust => {
            let name = get_node_name(node, content)?;
            let params = node
                .child_by_field_name("parameters")
                .map(|n| node_text(n, content))
                .unwrap_or("()");
            let return_type = node
                .child_by_field_name("return_type")
                .map(|n| format!(" -> {}", node_text(n, content)))
                .unwrap_or_default();
            Some(format!("fn {}{}{}", name, params, return_type))
        }
    }
}

/// Extract docstring from a function or class node
pub fn get_docstring(node: Node, content: &str, lang: &Language) -> Option<String> {
    match lang {
        Language::Python => {
            // Look for first expression_statement containing a string in the body
            let body = node.child_by_field_name("body")?;
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "expression_statement" {
                    let mut inner_cursor = child.walk();
                    for stmt_child in child.children(&mut inner_cursor) {
                        if stmt_child.kind() == "string" {
                            let docstring = node_text(stmt_child, content);
                            // Strip triple quotes
                            let trimmed = docstring
                                .trim_start_matches("\"\"\"")
                                .trim_start_matches("'''")
                                .trim_end_matches("\"\"\"")
                                .trim_end_matches("'''")
                                .trim();
                            return Some(trimmed.to_string());
                        }
                    }
                }
                // Only check first non-comment statement
                if child.kind() != "comment" {
                    break;
                }
            }
            None
        }
        Language::TypeScript | Language::JavaScript => {
            // Look for JSDoc comment before the function
            if let Some(prev) = node.prev_sibling() {
                if prev.kind() == "comment" {
                    let comment = node_text(prev, content);
                    if comment.starts_with("/**") {
                        return Some(
                            comment[3..comment.len().saturating_sub(2)]
                                .trim()
                                .to_string(),
                        );
                    }
                }
            }
            None
        }
        Language::Go => {
            if let Some(prev) = node.prev_sibling() {
                if prev.kind() == "comment" {
                    return Some(
                        node_text(prev, content)
                            .trim_start_matches("//")
                            .trim()
                            .to_string(),
                    );
                }
            }
            None
        }
        Language::Rust => {
            let mut doc_lines = Vec::new();
            let mut current = node.prev_sibling();
            while let Some(prev) = current {
                if prev.kind() == "line_comment" {
                    let comment = node_text(prev, content);
                    if comment.starts_with("///") {
                        doc_lines.push(comment.trim_start_matches("///").trim().to_string());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
                current = prev.prev_sibling();
            }
            if !doc_lines.is_empty() {
                doc_lines.reverse();
                Some(doc_lines.join("\n"))
            } else {
                None
            }
        }
    }
}

/// Extract import statements from source code
pub fn extract_imports(content: &str, lang: &Language) -> Result<Vec<String>, ChunkerError> {
    let tree = parse_code(content, lang)?;
    let root = tree.root_node();
    let mut imports = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match lang {
            Language::Python => {
                if child.kind() == "import_statement" || child.kind() == "import_from_statement" {
                    imports.push(node_text(child, content).to_string());
                }
            }
            Language::TypeScript | Language::JavaScript => {
                if child.kind() == "import_statement" {
                    if let Some(source) = child.child_by_field_name("source") {
                        let path = node_text(source, content);
                        imports.push(path.trim_matches(|c| c == '\'' || c == '"').to_string());
                    }
                }
            }
            Language::Go => {
                if child.kind() == "import_declaration" {
                    let mut inner_cursor = child.walk();
                    for spec_child in child.children(&mut inner_cursor) {
                        if spec_child.kind() == "import_spec" || spec_child.kind() == "import_spec_list" {
                            let mut spec_cursor = spec_child.walk();
                            for spec in spec_child.children(&mut spec_cursor) {
                                if spec.kind() == "import_spec" || spec.kind() == "interpreted_string_literal" {
                                    let path = node_text(spec, content);
                                    imports.push(
                                        path.trim_matches(|c| c == '"' || c == '`').to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Language::Rust => {
                if child.kind() == "use_declaration" {
                    imports.push(node_text(child, content).to_string());
                }
            }
        }
    }

    Ok(imports)
}

/// Get the body node kind for a language's class construct
pub fn class_body_kind(lang: &Language) -> &'static str {
    match lang {
        Language::Python => "block",
        Language::TypeScript | Language::JavaScript => "class_body",
        Language::Go => "field_declaration_list",
        Language::Rust => "declaration_list",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python() {
        let content = "def hello():\n    pass\n";
        let tree = parse_code(content, &Language::Python).unwrap();
        assert_eq!(tree.root_node().kind(), "module");
    }

    #[test]
    fn test_parse_go() {
        let content = "package main\n\nfunc main() {}\n";
        let tree = parse_code(content, &Language::Go).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_is_function_def_python() {
        let content = "def hello():\n    pass\n";
        let tree = parse_code(content, &Language::Python).unwrap();
        let root = tree.root_node();
        let mut cursor = root.walk();
        let func_node = root.children(&mut cursor).find(|n| n.kind() == "function_definition");
        assert!(func_node.is_some());
        assert!(is_function_def(func_node.unwrap(), &Language::Python));
    }

    #[test]
    fn test_get_node_name() {
        let content = "def hello():\n    pass\n";
        let tree = parse_code(content, &Language::Python).unwrap();
        let root = tree.root_node();
        let mut cursor = root.walk();
        let func_node = root.children(&mut cursor).find(|n| n.kind() == "function_definition").unwrap();
        assert_eq!(get_node_name(func_node, content), Some("hello".to_string()));
    }

    #[test]
    fn test_extract_imports_python() {
        let content = "import os\nfrom pathlib import Path\n\ndef main():\n    pass\n";
        let imports = extract_imports(content, &Language::Python).unwrap();
        assert_eq!(imports.len(), 2);
    }
}
