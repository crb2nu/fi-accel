//! Fuzzy diff matching and patch application
//!
//! Provides intelligent patch application that can handle files that have
//! drifted from their expected state. Uses similarity-based anchor finding
//! to locate the best match for each hunk.

use serde::{Deserialize, Serialize};
use similar::{Algorithm, DiffTag, TextDiff};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiffError {
    #[error("Failed to parse patch: {0}")]
    ParseError(String),
    #[error("No suitable match found for hunk at line {0}")]
    NoMatch(usize),
    #[error("Conflicting changes detected")]
    Conflict,
    #[error("Invalid patch format")]
    InvalidFormat,
}

/// Result of a patch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    /// The resulting text after patch application
    pub content: String,
    /// Number of hunks successfully applied
    pub applied: usize,
    /// Number of hunks that required fuzzy matching
    pub fuzzy_applied: usize,
    /// Number of hunks that failed to apply
    pub failed: usize,
    /// Details about each hunk application
    pub details: Vec<HunkResult>,
}

/// Result of applying a single hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunkResult {
    /// Original line number in patch
    pub patch_line: usize,
    /// Actual line number where applied (or attempted)
    pub applied_at: Option<usize>,
    /// Similarity score (1.0 = exact match)
    pub similarity: f32,
    /// Whether application succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// A parsed hunk from a unified diff
#[derive(Debug, Clone)]
pub struct Hunk {
    /// Original start line
    pub old_start: usize,
    /// Original line count
    pub old_count: usize,
    /// New start line
    pub new_start: usize,
    /// New line count
    pub new_count: usize,
    /// Context and change lines
    pub lines: Vec<HunkLine>,
}

/// A line within a hunk
#[derive(Debug, Clone)]
pub enum HunkLine {
    Context(String),
    Add(String),
    Remove(String),
}

/// Anchor point for fuzzy matching
#[derive(Debug, Clone)]
pub struct Anchor {
    /// Line number in the target file
    pub line: usize,
    /// Similarity score
    pub score: f32,
    /// Offset from expected position
    pub offset: i32,
}

/// Fuzzy matcher configuration and state
#[derive(Debug, Clone)]
pub struct FuzzyMatcher {
    /// Minimum similarity threshold (0.0 - 1.0)
    pub threshold: f32,
    /// Maximum search distance from expected location
    pub max_offset: usize,
    /// Whether to use parallel processing
    pub parallel: bool,
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            max_offset: 100,
            parallel: true,
        }
    }
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher with the given threshold
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            ..Default::default()
        }
    }

    /// Compute similarity between two strings using normalized edit distance
    pub fn similarity(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let diff = TextDiff::configure()
            .algorithm(Algorithm::Myers)
            .diff_chars(a, b);

        let total = a.len().max(b.len()) as f32;
        let changes: usize = diff
            .ops()
            .iter()
            .map(|op| match op.tag() {
                DiffTag::Equal => 0,
                DiffTag::Insert | DiffTag::Delete | DiffTag::Replace => {
                    op.new_range().len().max(op.old_range().len())
                }
            })
            .sum();

        1.0 - (changes as f32 / total)
    }

    /// Compute similarity between line sequences
    pub fn line_similarity(&self, a: &[&str], b: &[&str]) -> f32 {
        if a == b {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_joined = a.join("\n");
        let b_joined = b.join("\n");
        self.similarity(&a_joined, &b_joined)
    }

    /// Find the best anchor point for a hunk in the target file
    pub fn find_anchor(&self, hunk: &Hunk, file_lines: &[&str]) -> Option<Anchor> {
        // Extract context lines from hunk (lines that should exist)
        let context: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                HunkLine::Context(s) | HunkLine::Remove(s) => Some(s.as_str()),
                HunkLine::Add(_) => None,
            })
            .collect();

        if context.is_empty() {
            return None;
        }

        let expected_line = hunk.old_start.saturating_sub(1);
        let search_start = expected_line.saturating_sub(self.max_offset);
        let search_end = (expected_line + self.max_offset).min(file_lines.len());

        // Search for best match
        let candidates: Vec<(usize, f32)> = (search_start..search_end)
            .filter_map(|start| {
                let end = (start + context.len()).min(file_lines.len());
                if end - start < context.len() {
                    return None;
                }

                let window = &file_lines[start..end];
                let score = self.line_similarity(&context, window);

                if score >= self.threshold {
                    Some((start, score))
                } else {
                    None
                }
            })
            .collect();

        // Find best match (highest score, preferring closer to expected)
        candidates
            .into_iter()
            .max_by(|(line_a, score_a), (line_b, score_b)| {
                // Primary: higher score
                // Secondary: closer to expected position
                let cmp = score_a
                    .partial_cmp(score_b)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }

                let dist_a = (*line_a as i32 - expected_line as i32).abs();
                let dist_b = (*line_b as i32 - expected_line as i32).abs();
                dist_b.cmp(&dist_a) // Prefer smaller distance
            })
            .map(|(line, score)| Anchor {
                line,
                score,
                offset: line as i32 - expected_line as i32,
            })
    }

    /// Apply a fuzzy patch to a file
    pub fn apply_patch(&self, patch: &str, file: &str) -> Result<PatchResult, DiffError> {
        let hunks = parse_unified_diff(patch)?;
        let mut lines: Vec<String> = file.lines().map(String::from).collect();
        let mut details = Vec::new();
        let mut applied = 0;
        let mut fuzzy_applied = 0;
        let mut failed = 0;
        let mut offset: i32 = 0; // Cumulative offset from insertions/deletions

        for hunk in hunks {
            let file_lines: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

            // Adjust expected line by cumulative offset
            let adjusted_hunk = Hunk {
                old_start: ((hunk.old_start as i32 + offset).max(1)) as usize,
                ..hunk.clone()
            };

            match self.find_anchor(&adjusted_hunk, &file_lines) {
                Some(anchor) => {
                    let is_fuzzy = anchor.score < 0.999;

                    // Apply the hunk at the anchor position
                    let result = apply_hunk_at(&mut lines, &hunk, anchor.line);

                    match result {
                        Ok(line_delta) => {
                            offset += line_delta;
                            if is_fuzzy {
                                fuzzy_applied += 1;
                            }
                            applied += 1;
                            details.push(HunkResult {
                                patch_line: hunk.old_start,
                                applied_at: Some(anchor.line + 1),
                                similarity: anchor.score,
                                success: true,
                                error: None,
                            });
                        }
                        Err(e) => {
                            failed += 1;
                            details.push(HunkResult {
                                patch_line: hunk.old_start,
                                applied_at: None,
                                similarity: anchor.score,
                                success: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                None => {
                    failed += 1;
                    details.push(HunkResult {
                        patch_line: hunk.old_start,
                        applied_at: None,
                        similarity: 0.0,
                        success: false,
                        error: Some(format!("No match found within {} lines", self.max_offset)),
                    });
                }
            }
        }

        Ok(PatchResult {
            content: lines.join("\n"),
            applied,
            fuzzy_applied,
            failed,
            details,
        })
    }
}

/// Apply a hunk at a specific line position
fn apply_hunk_at(
    lines: &mut Vec<String>,
    hunk: &Hunk,
    start_line: usize,
) -> Result<i32, DiffError> {
    let mut removals = 0;
    let mut additions = 0;
    let mut current = start_line;

    for line in &hunk.lines {
        match line {
            HunkLine::Context(_) => {
                current += 1;
            }
            HunkLine::Remove(_) => {
                if current < lines.len() {
                    lines.remove(current);
                    removals += 1;
                }
            }
            HunkLine::Add(content) => {
                if current <= lines.len() {
                    lines.insert(current, content.clone());
                    current += 1;
                    additions += 1;
                }
            }
        }
    }

    Ok(additions - removals)
}

/// Parse a unified diff into hunks
pub fn parse_unified_diff(patch: &str) -> Result<Vec<Hunk>, DiffError> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<Hunk> = None;

    for line in patch.lines() {
        if line.starts_with("@@") {
            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                return Err(DiffError::ParseError("Invalid hunk header".to_string()));
            }

            let old_range = parts[1].trim_start_matches('-');
            let new_range = parts[2].trim_start_matches('+');

            let (old_start, old_count) = parse_range(old_range)?;
            let (new_start, new_count) = parse_range(new_range)?;

            current_hunk = Some(Hunk {
                old_start,
                old_count,
                new_start,
                new_count,
                lines: Vec::new(),
            });
        } else if let Some(ref mut hunk) = current_hunk {
            if let Some(content) = line.strip_prefix('+') {
                hunk.lines.push(HunkLine::Add(content.to_string()));
            } else if let Some(content) = line.strip_prefix('-') {
                hunk.lines.push(HunkLine::Remove(content.to_string()));
            } else if let Some(content) = line.strip_prefix(' ') {
                hunk.lines.push(HunkLine::Context(content.to_string()));
            } else if !line.starts_with('\\') && !line.is_empty() {
                // Treat as context if no prefix (some patches omit space prefix)
                hunk.lines.push(HunkLine::Context(line.to_string()));
            }
        }
    }

    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    Ok(hunks)
}

/// Parse a range like "10,5" or "10" into (start, count)
fn parse_range(range: &str) -> Result<(usize, usize), DiffError> {
    let parts: Vec<&str> = range.split(',').collect();
    let start: usize = parts[0]
        .parse()
        .map_err(|_| DiffError::ParseError("Invalid line number".to_string()))?;
    let count: usize = if parts.len() > 1 {
        parts[1]
            .parse()
            .map_err(|_| DiffError::ParseError("Invalid line count".to_string()))?
    } else {
        1
    };
    Ok((start, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_identical() {
        let matcher = FuzzyMatcher::default();
        assert!((matcher.similarity("hello", "hello") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_similarity_similar() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.similarity("hello world", "hello word");
        assert!(score > 0.8);
    }

    #[test]
    fn test_similarity_different() {
        let matcher = FuzzyMatcher::default();
        let score = matcher.similarity("hello", "goodbye");
        assert!(score < 0.5);
    }

    #[test]
    fn test_parse_hunk_header() {
        let patch = "@@ -10,5 +10,7 @@\n context\n-removed\n+added\n";
        let hunks = parse_unified_diff(patch).unwrap();

        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 10);
        assert_eq!(hunks[0].old_count, 5);
        assert_eq!(hunks[0].new_start, 10);
        assert_eq!(hunks[0].new_count, 7);
    }

    #[test]
    fn test_apply_simple_patch() {
        let file = "line1\nline2\nline3\nline4\nline5";
        let patch = "@@ -2,2 +2,2 @@\n line2\n-line3\n+line3_modified\n";

        let matcher = FuzzyMatcher::new(0.5);
        let result = matcher.apply_patch(patch, file).unwrap();

        assert!(result.content.contains("line3_modified"));
        assert!(!result.content.contains("\nline3\n"));
        assert_eq!(result.applied, 1);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_fuzzy_match_with_offset() {
        // File has extra lines at the beginning
        let file = "extra1\nextra2\nline1\nline2\nline3\nline4\nline5";
        // Patch expects line3 at line 3, but it's actually at line 5
        let patch = "@@ -2,2 +2,2 @@\n line2\n-line3\n+line3_modified\n";

        let matcher = FuzzyMatcher::new(0.5);
        let result = matcher.apply_patch(patch, file).unwrap();

        assert!(result.content.contains("line3_modified"));
        assert_eq!(result.applied, 1);
        // Should have been a fuzzy match due to offset
        assert!(result.fuzzy_applied > 0 || result.details[0].similarity == 1.0);
    }
}
