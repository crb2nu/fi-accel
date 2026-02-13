//! Efficient log streaming and filtering
//!
//! Provides incremental log reading with O(1) append operations and
//! regex-based filtering.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Efficient log reader with incremental updates
///
/// Maintains internal state for efficient tail() and append() operations
/// without re-reading the entire log.
#[derive(Debug, Clone)]
pub struct LogReader {
    /// All lines stored
    lines: Vec<String>,
    /// Compiled regex for filtering (if set)
    filter_regex: Option<Regex>,
}

impl LogReader {
    /// Create a new log reader from initial content
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Self {
            lines,
            filter_regex: None,
        }
    }

    /// Create an empty log reader
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            filter_regex: None,
        }
    }

    /// Set a filter pattern (regex)
    pub fn set_filter(&mut self, pattern: &str) -> Result<(), regex::Error> {
        self.filter_regex = Some(Regex::new(pattern)?);
        Ok(())
    }

    /// Clear the filter
    pub fn clear_filter(&mut self) {
        self.filter_regex = None;
    }

    /// Append new content and return the new lines
    pub fn append(&mut self, content: &str) -> Vec<String> {
        let new_lines: Vec<String> = content.lines().map(String::from).collect();
        let result = new_lines.clone();
        self.lines.extend(new_lines);
        result
    }

    /// Get the last n lines
    pub fn tail(&self, n: usize) -> Vec<String> {
        let start = self.lines.len().saturating_sub(n);
        self.lines[start..].to_vec()
    }

    /// Get all lines matching the current filter (or all lines if no filter)
    pub fn filter(&self, pattern: &str) -> Vec<String> {
        match Regex::new(pattern) {
            Ok(re) => self.lines.iter().filter(|line| re.is_match(line)).cloned().collect(),
            Err(_) => {
                // Fall back to substring match
                self.lines
                    .iter()
                    .filter(|line| line.contains(pattern))
                    .cloned()
                    .collect()
            }
        }
    }

    /// Get lines matching the stored filter
    pub fn get_filtered(&self) -> Vec<String> {
        match &self.filter_regex {
            Some(re) => self.lines.iter().filter(|line| re.is_match(line)).cloned().collect(),
            None => self.lines.clone(),
        }
    }

    /// Get total line count
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get all lines
    pub fn all_lines(&self) -> &[String] {
        &self.lines
    }

    /// Get lines in a range
    pub fn range(&self, start: usize, end: usize) -> Vec<String> {
        let end = end.min(self.lines.len());
        let start = start.min(end);
        self.lines[start..end].to_vec()
    }

    /// Search for lines containing a substring (faster than regex for simple cases)
    pub fn search(&self, substring: &str) -> Vec<(usize, String)> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(substring))
            .map(|(i, line)| (i, line.clone()))
            .collect()
    }

    /// Get lines around a specific line number (context)
    pub fn context(&self, line_num: usize, before: usize, after: usize) -> Vec<String> {
        let start = line_num.saturating_sub(before);
        let end = (line_num + after + 1).min(self.lines.len());
        self.lines[start..end].to_vec()
    }
}

/// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub line_number: usize,
    pub content: String,
    pub timestamp: Option<String>,
    pub level: Option<String>,
    pub message: Option<String>,
}

/// Parse common log formats
pub fn parse_log_line(line: &str, line_number: usize) -> LogEntry {
    // Try to parse common formats
    // Format 1: [TIMESTAMP] [LEVEL] message
    // Format 2: TIMESTAMP LEVEL message
    // Format 3: Just the line

    let mut timestamp = None;
    let mut level = None;
    let mut message = None;

    // Try bracket format: [2024-01-15 10:30:00] [INFO] message
    if line.starts_with('[') {
        let parts: Vec<&str> = line.splitn(3, ']').collect();
        if parts.len() >= 2 {
            timestamp = Some(parts[0].trim_start_matches('[').to_string());

            if parts.len() >= 3 {
                let rest = parts[1].trim();
                if rest.starts_with('[') {
                    level = Some(rest.trim_matches(|c| c == '[' || c == ' ').to_string());
                    message = Some(parts[2].trim().to_string());
                } else {
                    message = Some(format!("{} {}", rest, parts[2]).trim().to_string());
                }
            }
        }
    }
    // Try space-separated: 2024-01-15T10:30:00Z INFO message
    else if line.len() > 20 {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            // Check if first part looks like a timestamp
            if parts[0].contains(':') || parts[0].contains('T') {
                timestamp = Some(parts[0].to_string());
                level = Some(parts[1].to_string());
                message = Some(parts[2].to_string());
            }
        }
    }

    LogEntry {
        line_number,
        content: line.to_string(),
        timestamp,
        level,
        message,
    }
}

/// Parse multiple log lines
pub fn parse_log(content: &str) -> Vec<LogEntry> {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| parse_log_line(line, i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_reader_new() {
        let content = "line1\nline2\nline3";
        let reader = LogReader::new(content);
        assert_eq!(reader.len(), 3);
    }

    #[test]
    fn test_log_reader_tail() {
        let content = "line1\nline2\nline3\nline4\nline5";
        let reader = LogReader::new(content);
        let tail = reader.tail(2);
        assert_eq!(tail, vec!["line4", "line5"]);
    }

    #[test]
    fn test_log_reader_append() {
        let mut reader = LogReader::new("line1\nline2");
        let new_lines = reader.append("line3\nline4");
        assert_eq!(new_lines, vec!["line3", "line4"]);
        assert_eq!(reader.len(), 4);
    }

    #[test]
    fn test_log_reader_filter() {
        let content = "error: something failed\ninfo: all good\nerror: another failure";
        let reader = LogReader::new(content);
        let errors = reader.filter("error");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_log_reader_filter_regex() {
        let content = "ERROR: something failed\nINFO: all good\nWARN: warning";
        let reader = LogReader::new(content);
        let matches = reader.filter(r"ERROR|WARN");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_log_reader_search() {
        let content = "line1\nfound here\nline3\nalso found";
        let reader = LogReader::new(content);
        let results = reader.search("found");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1); // line number
        assert_eq!(results[1].0, 3);
    }

    #[test]
    fn test_log_reader_context() {
        let content = "line0\nline1\nline2\nline3\nline4";
        let reader = LogReader::new(content);
        let ctx = reader.context(2, 1, 1);
        assert_eq!(ctx, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_parse_bracket_format() {
        let line = "[2024-01-15 10:30:00] [INFO] Application started";
        let entry = parse_log_line(line, 0);
        assert_eq!(entry.timestamp.as_deref(), Some("2024-01-15 10:30:00"));
        assert_eq!(entry.level.as_deref(), Some("INFO"));
    }

    #[test]
    fn test_parse_iso_format() {
        let line = "2024-01-15T10:30:00Z INFO Application started";
        let entry = parse_log_line(line, 0);
        assert_eq!(entry.timestamp.as_deref(), Some("2024-01-15T10:30:00Z"));
        assert_eq!(entry.level.as_deref(), Some("INFO"));
    }
}
