//! Surface helpers inspired by interactive observability and delivery UIs.
//!
//! These helpers were shaped around FlexDeck's needs, but they stay generic
//! enough to reuse in other dashboards and control planes.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SurfaceLogFilter {
    #[serde(default)]
    pub levels: Vec<SurfaceLogLevel>,
    pub search_term: Option<String>,
    #[serde(default)]
    pub search_regex: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceLogMatch {
    pub index: usize,
    pub level: SurfaceLogLevel,
    pub matches_filter: bool,
    pub matches_search: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceCiStatus {
    Pending,
    Running,
    Success,
    Failed,
    Manual,
    Skipped,
    Canceled,
}

pub fn detect_log_level(line: &str) -> SurfaceLogLevel {
    let lower = line.to_lowercase();

    if lower.contains("error")
        || lower.contains("fatal")
        || lower.contains("panic")
        || lower.contains("exception")
        || lower.contains("fail")
    {
        return SurfaceLogLevel::Error;
    }

    if lower.contains("warn") || lower.contains("warning") {
        return SurfaceLogLevel::Warn;
    }

    if lower.contains("debug") || lower.contains("trace") {
        return SurfaceLogLevel::Debug;
    }

    SurfaceLogLevel::Info
}

pub fn analyze_log_lines(lines: &[String], filter: &SurfaceLogFilter) -> Vec<SurfaceLogMatch> {
    let regex = match filter.search_term.as_deref() {
        Some(pattern) if filter.search_regex => Regex::new(pattern).ok(),
        _ => None,
    };
    let normalized_search = filter.search_term.as_ref().map(|value| value.to_lowercase());
    let has_level_filter = !filter.levels.is_empty();

    lines.iter()
        .enumerate()
        .map(|(index, line)| {
            let level = detect_log_level(line);
            let matches_filter = !has_level_filter || filter.levels.contains(&level);
            let matches_search = match normalized_search.as_deref() {
                Some(pattern) if !pattern.is_empty() => {
                    if let Some(regex) = regex.as_ref() {
                        regex.is_match(line)
                    } else {
                        line.to_lowercase().contains(pattern)
                    }
                }
                _ => false,
            };

            SurfaceLogMatch {
                index,
                level,
                matches_filter,
                matches_search,
            }
        })
        .collect()
}

pub fn matches_label_selector(
    labels: &HashMap<String, String>,
    selector: &HashMap<String, String>,
) -> bool {
    selector
        .iter()
        .all(|(key, value)| labels.get(key).map(|candidate| candidate == value).unwrap_or(false))
}

pub fn filter_label_selector_matches(
    label_sets: &[HashMap<String, String>],
    selector: &HashMap<String, String>,
) -> Vec<usize> {
    label_sets
        .iter()
        .enumerate()
        .filter_map(|(index, labels)| matches_label_selector(labels, selector).then_some(index))
        .collect()
}

pub fn normalize_ci_job_status(status: &str) -> SurfaceCiStatus {
    match status.trim().to_lowercase().as_str() {
        "running" => SurfaceCiStatus::Running,
        "success" => SurfaceCiStatus::Success,
        "failed" => SurfaceCiStatus::Failed,
        "manual" => SurfaceCiStatus::Manual,
        "skipped" => SurfaceCiStatus::Skipped,
        "canceled" | "cancelled" | "canceling" => SurfaceCiStatus::Failed,
        "pending" | "created" | "preparing" | "waiting_for_resource" | "scheduled" => {
            SurfaceCiStatus::Pending
        }
        _ => SurfaceCiStatus::Pending,
    }
}

pub fn normalize_ci_pipeline_status(status: &str) -> SurfaceCiStatus {
    match status.trim().to_lowercase().as_str() {
        "running" => SurfaceCiStatus::Running,
        "success" => SurfaceCiStatus::Success,
        "failed" => SurfaceCiStatus::Failed,
        "canceled" | "cancelled" => SurfaceCiStatus::Canceled,
        "skipped" => SurfaceCiStatus::Canceled,
        "pending" | "created" | "preparing" | "waiting_for_resource" | "scheduled" | "manual" => {
            SurfaceCiStatus::Pending
        }
        _ => SurfaceCiStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_log_levels_with_flexdeck_style_rules() {
        assert_eq!(detect_log_level("ERROR failed to connect"), SurfaceLogLevel::Error);
        assert_eq!(detect_log_level("warning: backpressure rising"), SurfaceLogLevel::Warn);
        assert_eq!(detect_log_level("DEBUG scheduling tick"), SurfaceLogLevel::Debug);
        assert_eq!(detect_log_level("service warmed successfully"), SurfaceLogLevel::Info);
    }

    #[test]
    fn analyzes_log_lines_with_level_filter_and_search_highlight() {
        let lines = vec![
            "INFO flexinfer proxy ready".to_string(),
            "WARN gpu queue depth rising".to_string(),
            "ERROR stream failed for worker-2".to_string(),
        ];
        let filter = SurfaceLogFilter {
            levels: vec![SurfaceLogLevel::Warn, SurfaceLogLevel::Error],
            search_term: Some("worker".to_string()),
            search_regex: false,
        };

        let matches = analyze_log_lines(&lines, &filter);
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].matches_filter, false);
        assert_eq!(matches[1].matches_filter, true);
        assert_eq!(matches[1].matches_search, false);
        assert_eq!(matches[2].matches_filter, true);
        assert_eq!(matches[2].matches_search, true);
    }

    #[test]
    fn falls_back_to_substring_when_regex_is_invalid() {
        let lines = vec!["ERROR [worker] exploded".to_string()];
        let filter = SurfaceLogFilter {
            levels: vec![],
            search_term: Some("[worker".to_string()),
            search_regex: true,
        };

        let matches = analyze_log_lines(&lines, &filter);
        assert!(matches[0].matches_search);
    }

    #[test]
    fn matches_kubernetes_style_label_selectors() {
        let labels = HashMap::from([
            ("app".to_string(), "proxy".to_string()),
            ("component".to_string(), "gateway".to_string()),
        ]);
        let selector = HashMap::from([
            ("app".to_string(), "proxy".to_string()),
            ("component".to_string(), "gateway".to_string()),
        ]);
        let miss = HashMap::from([("app".to_string(), "worker".to_string())]);

        assert!(matches_label_selector(&labels, &selector));
        assert!(!matches_label_selector(&labels, &miss));
    }

    #[test]
    fn filters_matching_label_sets() {
        let label_sets = vec![
            HashMap::from([
                ("app".to_string(), "proxy".to_string()),
                ("tier".to_string(), "edge".to_string()),
            ]),
            HashMap::from([
                ("app".to_string(), "worker".to_string()),
                ("tier".to_string(), "batch".to_string()),
            ]),
            HashMap::from([
                ("app".to_string(), "proxy".to_string()),
                ("tier".to_string(), "edge".to_string()),
            ]),
        ];
        let selector = HashMap::from([
            ("app".to_string(), "proxy".to_string()),
            ("tier".to_string(), "edge".to_string()),
        ]);

        assert_eq!(filter_label_selector_matches(&label_sets, &selector), vec![0, 2]);
    }

    #[test]
    fn normalizes_ci_job_status_for_pipeline_widgets() {
        assert_eq!(normalize_ci_job_status("waiting_for_resource"), SurfaceCiStatus::Pending);
        assert_eq!(normalize_ci_job_status("running"), SurfaceCiStatus::Running);
        assert_eq!(normalize_ci_job_status("manual"), SurfaceCiStatus::Manual);
        assert_eq!(normalize_ci_job_status("canceling"), SurfaceCiStatus::Failed);
    }

    #[test]
    fn normalizes_ci_pipeline_status_for_overview_cards() {
        assert_eq!(normalize_ci_pipeline_status("manual"), SurfaceCiStatus::Pending);
        assert_eq!(normalize_ci_pipeline_status("success"), SurfaceCiStatus::Success);
        assert_eq!(normalize_ci_pipeline_status("cancelled"), SurfaceCiStatus::Canceled);
        assert_eq!(normalize_ci_pipeline_status("skipped"), SurfaceCiStatus::Canceled);
    }
}
