use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_index: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Error, Debug)]
pub enum PatternError {
    #[error("invalid regex pattern {index}: {message}")]
    InvalidRegex { index: usize, message: String },
}

pub fn find_literal_matches(patterns: &[String], text: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        if pattern.is_empty() {
            continue;
        }
        for (start, _) in text.match_indices(pattern) {
            matches.push(PatternMatch {
                pattern_index,
                start,
                end: start + pattern.len(),
            });
        }
    }
    matches.sort_by_key(|item| (item.start, item.end, item.pattern_index));
    matches
}

pub fn compile_regex_patterns(patterns: &[String]) -> Result<Vec<Regex>, PatternError> {
    patterns
        .iter()
        .enumerate()
        .map(|(index, pattern)| {
            Regex::new(pattern).map_err(|err| PatternError::InvalidRegex {
                index,
                message: err.to_string(),
            })
        })
        .collect()
}

pub fn find_regex_matches(regexes: &[Regex], text: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    for (pattern_index, regex) in regexes.iter().enumerate() {
        for mat in regex.find_iter(text) {
            matches.push(PatternMatch {
                pattern_index,
                start: mat.start(),
                end: mat.end(),
            });
        }
    }
    matches.sort_by_key(|item| (item.start, item.end, item.pattern_index));
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_literal_matches_in_text_order() {
        let patterns = vec!["foo".to_string(), "bar".to_string()];
        let matches = find_literal_matches(&patterns, "foo bar baz foo");
        let triples: Vec<_> = matches
            .into_iter()
            .map(|m| (m.pattern_index, m.start, m.end))
            .collect();
        assert_eq!(triples, vec![(0, 0, 3), (1, 4, 7), (0, 12, 15)]);
    }

    #[test]
    fn finds_regex_matches() {
        let regexes = compile_regex_patterns(&[r"[A-Z]+-\d+".to_string()]).unwrap();
        let matches = find_regex_matches(&regexes, "See FA-123 and XX-9");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 4);
    }
}
