//! Plain text chunking by paragraphs and sentences

use super::{count_or_estimate_tokens, generate_chunk_id, Chunk, ChunkConfig, ChunkKind};

/// Split plain text into chunks by paragraph boundaries, falling back to
/// sentence splitting when paragraphs exceed the token limit.
pub fn chunk_text(content: &str, config: &ChunkConfig) -> Vec<Chunk> {
    if content.trim().is_empty() {
        return vec![];
    }

    let model = config.model.as_deref();
    let file_path = config.file_path.as_deref();
    let mut chunks = Vec::new();

    let paragraphs = split_paragraphs(content);

    for para in &paragraphs {
        let token_count = count_or_estimate_tokens(para.text, model);

        if token_count <= config.max_tokens {
            chunks.push(Chunk {
                id: generate_chunk_id(file_path, para.start_line, para.text),
                content: para.text.to_string(),
                kind: ChunkKind::Paragraph,
                language: "text".to_string(),
                start_line: para.start_line,
                end_line: para.end_line,
                token_count,
                name: None,
                parent_name: None,
                signature: None,
                docstring: None,
            });
        } else {
            // Paragraph is too large — split by sentences
            let sentences = split_sentences(para.text);
            let mut buffer = String::new();
            let mut buf_start_line = para.start_line;
            let mut buf_tokens = 0usize;

            for sent in &sentences {
                let sent_tokens = count_or_estimate_tokens(sent.text, model);

                if buf_tokens + sent_tokens > config.max_tokens && !buffer.is_empty() {
                    let trimmed = buffer.trim_end();
                    chunks.push(Chunk {
                        id: generate_chunk_id(file_path, buf_start_line, trimmed),
                        content: trimmed.to_string(),
                        kind: ChunkKind::Sentence,
                        language: "text".to_string(),
                        start_line: buf_start_line,
                        end_line: para.start_line + sent.line_offset.saturating_sub(1),
                        token_count: count_or_estimate_tokens(trimmed, model),
                        name: None,
                        parent_name: None,
                        signature: None,
                        docstring: None,
                    });
                    buffer.clear();
                    buf_start_line = para.start_line + sent.line_offset;
                    buf_tokens = 0;
                }

                if !buffer.is_empty() {
                    buffer.push(' ');
                }
                buffer.push_str(sent.text);
                buf_tokens += sent_tokens;
            }

            // Flush remaining buffer
            if !buffer.is_empty() {
                let trimmed = buffer.trim_end();
                chunks.push(Chunk {
                    id: generate_chunk_id(file_path, buf_start_line, trimmed),
                    content: trimmed.to_string(),
                    kind: ChunkKind::Sentence,
                    language: "text".to_string(),
                    start_line: buf_start_line,
                    end_line: para.end_line,
                    token_count: count_or_estimate_tokens(trimmed, model),
                    name: None,
                    parent_name: None,
                    signature: None,
                    docstring: None,
                });
            }
        }
    }

    chunks
}

struct Paragraph<'a> {
    text: &'a str,
    start_line: usize,
    end_line: usize,
}

/// Split content into paragraphs (separated by blank lines)
fn split_paragraphs(content: &str) -> Vec<Paragraph<'_>> {
    let mut paragraphs = Vec::new();
    let mut para_start: Option<usize> = None;
    let mut para_start_line = 1usize;
    let mut current_line = 1usize;

    for line in content.split('\n') {
        let is_blank = line.trim().is_empty();

        if is_blank {
            if let Some(_start) = para_start {
                // Find the actual byte range
                let text = extract_lines(content, para_start_line, current_line - 1);
                if !text.trim().is_empty() {
                    paragraphs.push(Paragraph {
                        text,
                        start_line: para_start_line,
                        end_line: current_line - 1,
                    });
                }
                para_start = None;
            }
        } else if para_start.is_none() {
            para_start = Some(0);
            para_start_line = current_line;
        }

        current_line += 1;
    }

    // Final paragraph
    if para_start.is_some() {
        let text = extract_lines(content, para_start_line, current_line - 1);
        if !text.trim().is_empty() {
            paragraphs.push(Paragraph {
                text,
                start_line: para_start_line,
                end_line: current_line - 1,
            });
        }
    }

    paragraphs
}

/// Extract lines from content by 1-indexed line range
fn extract_lines(content: &str, start_line: usize, end_line: usize) -> &str {
    let mut line_num = 1usize;
    let mut start_byte = 0usize;
    let mut end_byte = content.len();
    let mut found_start = false;

    for (i, _) in content.char_indices() {
        if line_num == start_line && !found_start {
            start_byte = i;
            found_start = true;
        }
        if i > 0 && content.as_bytes().get(i - 1) == Some(&b'\n') {
            line_num += 1;
        }
        if line_num > end_line {
            end_byte = i;
            break;
        }
    }

    if !found_start {
        return "";
    }

    content[start_byte..end_byte].trim_end_matches('\n')
}

struct Sentence<'a> {
    text: &'a str,
    line_offset: usize,
}

/// Split a paragraph into sentences using simple heuristics
fn split_sentences(text: &str) -> Vec<Sentence<'_>> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        let b = bytes[i];
        // Sentence-ending punctuation followed by whitespace or end
        if (b == b'.' || b == b'!' || b == b'?')
            && (i + 1 >= len || bytes[i + 1] == b' ' || bytes[i + 1] == b'\n')
        {
            // Skip abbreviations like "e.g." "i.e." "Dr." "Mr."
            let before = &text[start..=i];
            if !looks_like_abbreviation(before) {
                let sent_text = text[start..=i].trim();
                if !sent_text.is_empty() {
                    let line_offset = text[..start].matches('\n').count();
                    sentences.push(Sentence {
                        text: sent_text,
                        line_offset,
                    });
                }
                start = i + 1;
            }
        }
        i += 1;
    }

    // Remaining text
    let rest = text[start..].trim();
    if !rest.is_empty() {
        let line_offset = text[..start].matches('\n').count();
        sentences.push(Sentence {
            text: rest,
            line_offset,
        });
    }

    // If no sentences were found, return the whole text as one
    if sentences.is_empty() {
        sentences.push(Sentence {
            text: text.trim(),
            line_offset: 0,
        });
    }

    sentences
}

/// Check if a period is likely an abbreviation rather than sentence end
fn looks_like_abbreviation(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() < 3 {
        return true; // Too short, likely abbreviation
    }

    // Common abbreviations
    let lower = trimmed.to_lowercase();
    let abbrevs = [
        "e.g.", "i.e.", "etc.", "vs.", "mr.", "mrs.", "dr.", "jr.", "sr.", "st.", "ave.", "blvd.",
        "dept.", "inc.", "ltd.", "corp.",
    ];

    for abbr in &abbrevs {
        if lower.ends_with(abbr) {
            return true;
        }
    }

    // Single letter followed by period (initials like "J.")
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() >= 2 {
        let last = chars[chars.len() - 1];
        let second_last = chars[chars.len() - 2];
        if last == '.' && second_last.is_alphabetic() {
            // Check if preceded by space or start
            if chars.len() == 2 || chars[chars.len() - 3] == ' ' {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_simple() {
        let content = "This is the first paragraph.\n\nThis is the second paragraph.";
        let config = ChunkConfig::default();
        let chunks = chunk_text(content, &config);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].kind, ChunkKind::Paragraph);
        assert_eq!(chunks[1].kind, ChunkKind::Paragraph);
        assert!(chunks[0].content.contains("first"));
        assert!(chunks[1].content.contains("second"));
    }

    #[test]
    fn test_chunk_text_empty() {
        let chunks = chunk_text("", &ChunkConfig::default());
        assert!(chunks.is_empty());

        let chunks = chunk_text("   \n  \n  ", &ChunkConfig::default());
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_text_single_paragraph() {
        let content =
            "Just one paragraph with multiple sentences. This is the second. And a third.";
        let config = ChunkConfig::default();
        let chunks = chunk_text(content, &config);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_line, 1);
    }

    #[test]
    fn test_chunk_text_large_paragraph_splits() {
        // Create a paragraph that exceeds max_tokens
        let sentence = "This is a test sentence with enough words to take up some tokens. ";
        let large_para = sentence.repeat(50);

        let config = ChunkConfig {
            max_tokens: 20,
            ..ChunkConfig::default()
        };

        let chunks = chunk_text(&large_para, &config);
        assert!(chunks.len() > 1, "Large paragraph should be split");
        assert!(chunks.iter().all(|c| c.kind == ChunkKind::Sentence));
    }

    #[test]
    fn test_chunk_text_ids_unique() {
        let content = "First paragraph.\n\nSecond paragraph.";
        let config = ChunkConfig::default();
        let chunks = chunk_text(content, &config);

        assert_ne!(chunks[0].id, chunks[1].id);
    }

    #[test]
    fn test_split_sentences() {
        let text = "First sentence. Second sentence. Third sentence.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0].text, "First sentence.");
        assert_eq!(sentences[1].text, "Second sentence.");
        assert_eq!(sentences[2].text, "Third sentence.");
    }

    #[test]
    fn test_split_sentences_abbreviations() {
        let text = "Dr. Smith went to the store. He bought e.g. apples.";
        let sentences = split_sentences(text);
        // Should not split on "Dr." or "e.g."
        assert_eq!(sentences.len(), 2);
    }

    #[test]
    fn test_chunk_text_line_numbers() {
        let content = "Line one.\n\nLine three.\nLine four.\n\nLine six.";
        let config = ChunkConfig::default();
        let chunks = chunk_text(content, &config);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[1].start_line, 3);
        assert_eq!(chunks[2].start_line, 6);
    }
}
