//! Markdown chunking using pulldown-cmark for heading-boundary splitting

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use super::{Chunk, ChunkConfig, ChunkKind, ChunkerError, count_or_estimate_tokens, generate_chunk_id};

/// Split markdown content into chunks at heading boundaries.
///
/// Sections defined by headings become individual chunks. Sections that
/// exceed `max_tokens` are further split at paragraph boundaries.
pub fn chunk_markdown(content: &str, config: &ChunkConfig) -> Result<Vec<Chunk>, ChunkerError> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let sections = extract_sections(content);
    let model = config.model.as_deref();
    let file_path = config.file_path.as_deref();
    let mut chunks = Vec::new();

    for section in &sections {
        let token_count = count_or_estimate_tokens(section.content, model);

        if token_count <= config.max_tokens {
            chunks.push(Chunk {
                id: generate_chunk_id(file_path, section.start_line, section.content),
                content: section.content.to_string(),
                kind: ChunkKind::Section,
                language: "markdown".to_string(),
                start_line: section.start_line,
                end_line: section.end_line,
                token_count,
                name: section.heading.clone(),
                parent_name: None,
                signature: None,
                docstring: None,
            });
        } else {
            // Section is too large — split at paragraph boundaries
            let sub_chunks = split_large_section(section, config);
            chunks.extend(sub_chunks);
        }
    }

    Ok(chunks)
}

struct Section<'a> {
    heading: Option<String>,
    content: &'a str,
    start_line: usize,
    end_line: usize,
    #[allow(dead_code)]
    level: Option<u8>,
}

/// Extract sections from markdown using pulldown-cmark events to find
/// heading byte offsets, then slice the source string.
fn extract_sections(content: &str) -> Vec<Section<'_>> {
    let mut heading_positions: Vec<(usize, String, u8)> = Vec::new();

    let parser = Parser::new(content);
    let mut in_heading = false;
    let mut current_heading = String::new();
    let mut heading_byte_start = 0usize;
    let mut heading_level = 0u8;

    // pulldown-cmark doesn't give byte offsets directly in all versions,
    // so we use the offset-based parser
    let parser_with_offsets = Parser::new(content).into_offset_iter();

    for (event, range) in parser_with_offsets {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_heading.clear();
                heading_byte_start = range.start;
                heading_level = heading_level_to_u8(level);
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                heading_positions.push((
                    heading_byte_start,
                    current_heading.clone(),
                    heading_level,
                ));
            }
            Event::Text(text) if in_heading => {
                current_heading.push_str(&text);
            }
            Event::Code(code) if in_heading => {
                current_heading.push('`');
                current_heading.push_str(&code);
                current_heading.push('`');
            }
            _ => {}
        }
    }

    // Drop the unused parser
    drop(parser);

    // Build sections from heading positions
    let mut sections = Vec::new();

    if heading_positions.is_empty() {
        // No headings — entire content is one section
        let end_line = content.matches('\n').count() + 1;
        return vec![Section {
            heading: None,
            content: content.trim_end(),
            start_line: 1,
            end_line,
            level: None,
        }];
    }

    // Content before first heading (if any)
    if heading_positions[0].0 > 0 {
        let preamble = &content[..heading_positions[0].0];
        if !preamble.trim().is_empty() {
            let end_line = preamble.matches('\n').count() + 1;
            sections.push(Section {
                heading: None,
                content: preamble.trim_end(),
                start_line: 1,
                end_line,
                level: None,
            });
        }
    }

    // Each heading starts a section that extends to the next heading
    for i in 0..heading_positions.len() {
        let (byte_start, ref heading, level) = heading_positions[i];
        let byte_end = if i + 1 < heading_positions.len() {
            heading_positions[i + 1].0
        } else {
            content.len()
        };

        let section_content = &content[byte_start..byte_end];
        let trimmed = section_content.trim_end();
        if trimmed.is_empty() {
            continue;
        }

        let start_line = content[..byte_start].matches('\n').count() + 1;
        let end_line = content[..byte_end].matches('\n').count() + 1;

        sections.push(Section {
            heading: Some(heading.clone()),
            content: trimmed,
            start_line,
            end_line,
            level: Some(level),
        });
    }

    sections
}

/// Split a large section into smaller chunks at paragraph boundaries
fn split_large_section(section: &Section<'_>, config: &ChunkConfig) -> Vec<Chunk> {
    let model = config.model.as_deref();
    let file_path = config.file_path.as_deref();
    let mut chunks = Vec::new();

    let mut buffer = String::new();
    let mut buf_start_line = section.start_line;
    let mut buf_tokens = 0usize;
    let mut current_line = section.start_line;

    // Split on double newlines (paragraph boundaries)
    for part in section.content.split("\n\n") {
        let part_trimmed = part.trim();
        if part_trimmed.is_empty() {
            current_line += part.matches('\n').count() + 2;
            continue;
        }

        let part_tokens = count_or_estimate_tokens(part_trimmed, model);

        if buf_tokens + part_tokens > config.max_tokens && !buffer.is_empty() {
            let trimmed = buffer.trim();
            chunks.push(Chunk {
                id: generate_chunk_id(file_path, buf_start_line, trimmed),
                content: trimmed.to_string(),
                kind: ChunkKind::Section,
                language: "markdown".to_string(),
                start_line: buf_start_line,
                end_line: current_line.saturating_sub(1),
                token_count: count_or_estimate_tokens(trimmed, model),
                name: section.heading.clone(),
                parent_name: None,
                signature: None,
                docstring: None,
            });
            buffer.clear();
            buf_start_line = current_line;
            buf_tokens = 0;
        }

        if !buffer.is_empty() {
            buffer.push_str("\n\n");
        }
        buffer.push_str(part_trimmed);
        buf_tokens += part_tokens;
        current_line += part.matches('\n').count() + 2;
    }

    // Flush remaining
    if !buffer.is_empty() {
        let trimmed = buffer.trim();
        chunks.push(Chunk {
            id: generate_chunk_id(file_path, buf_start_line, trimmed),
            content: trimmed.to_string(),
            kind: ChunkKind::Section,
            language: "markdown".to_string(),
            start_line: buf_start_line,
            end_line: section.end_line,
            token_count: count_or_estimate_tokens(trimmed, model),
            name: section.heading.clone(),
            parent_name: None,
            signature: None,
            docstring: None,
        });
    }

    chunks
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_markdown_simple() {
        let content = "# Title\n\nSome intro text.\n\n## Section 1\n\nContent one.\n\n## Section 2\n\nContent two.\n";
        let config = ChunkConfig::default();
        let chunks = chunk_markdown(content, &config).unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].name.as_deref(), Some("Title"));
        assert_eq!(chunks[1].name.as_deref(), Some("Section 1"));
        assert_eq!(chunks[2].name.as_deref(), Some("Section 2"));
    }

    #[test]
    fn test_chunk_markdown_no_headings() {
        let content = "Just some plain text without any headings.\n\nAnother paragraph.";
        let config = ChunkConfig::default();
        let chunks = chunk_markdown(content, &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].name.is_none());
    }

    #[test]
    fn test_chunk_markdown_empty() {
        let chunks = chunk_markdown("", &ChunkConfig::default()).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_markdown_preamble() {
        let content = "Some preamble text.\n\n# Actual Heading\n\nHeading content.";
        let config = ChunkConfig::default();
        let chunks = chunk_markdown(content, &config).unwrap();

        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].name.is_none()); // Preamble
        assert_eq!(chunks[1].name.as_deref(), Some("Actual Heading"));
    }

    #[test]
    fn test_chunk_markdown_large_section_splits() {
        let paragraph = "This is a paragraph with enough text to use tokens. ";
        let mut content = String::from("# Big Section\n\n");
        for _ in 0..30 {
            content.push_str(paragraph);
            content.push_str("\n\n");
        }

        let config = ChunkConfig {
            max_tokens: 30,
            ..ChunkConfig::default()
        };

        let chunks = chunk_markdown(&content, &config).unwrap();
        assert!(chunks.len() > 1, "Large section should be split");
    }

    #[test]
    fn test_chunk_markdown_all_sections() {
        let content = "# H1\n\nH1 content.\n\n## H2\n\nH2 content.\n\n### H3\n\nH3 content.";
        let config = ChunkConfig::default();
        let chunks = chunk_markdown(content, &config).unwrap();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].name.as_deref(), Some("H1"));
        assert_eq!(chunks[1].name.as_deref(), Some("H2"));
        assert_eq!(chunks[2].name.as_deref(), Some("H3"));
    }
}
