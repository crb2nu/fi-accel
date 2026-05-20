//! Acceleration scouting helpers for service hot-path adoption.
//!
//! These helpers summarize lightweight operation observations so services can
//! identify which repeated CPU-side work is worth replacing with `fi-accel`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccelScoutConfig {
    pub operation_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_field: Option<String>,
    pub duration_ms_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_bytes_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl Default for AccelScoutConfig {
    fn default() -> Self {
        Self {
            operation_field: "operation".to_string(),
            service_field: Some("service".to_string()),
            duration_ms_field: "duration_ms".to_string(),
            input_bytes_field: Some("input_bytes".to_string()),
            fallback_field: Some("fallback".to_string()),
            limit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccelOperationSummary {
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    pub sample_count: usize,
    pub total_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub max_duration_ms: f64,
    pub total_input_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_input_bytes: Option<f64>,
    pub fallback_count: usize,
    pub fallback_rate: f64,
    pub candidate_score: f64,
}

#[derive(Debug, Default)]
struct Accumulator {
    durations: Vec<f64>,
    total_duration_ms: f64,
    max_duration_ms: f64,
    total_input_bytes: u64,
    input_sample_count: usize,
    fallback_count: usize,
}

pub fn summarize_accel_observations(
    values: &[serde_json::Value],
    config: &AccelScoutConfig,
) -> Vec<AccelOperationSummary> {
    let iter: Box<dyn Iterator<Item = &serde_json::Value>> = match config.limit {
        Some(limit) => Box::new(values.iter().take(limit)),
        None => Box::new(values.iter()),
    };

    let mut grouped: BTreeMap<(Option<String>, String), Accumulator> = BTreeMap::new();

    for value in iter {
        let Some(operation) = get_string_field(value, &config.operation_field) else {
            continue;
        };
        let Some(duration_ms) = get_f64_field(value, &config.duration_ms_field) else {
            continue;
        };
        if !duration_ms.is_finite() || duration_ms < 0.0 {
            continue;
        }

        let service = config
            .service_field
            .as_deref()
            .and_then(|field| get_string_field(value, field));
        let input_bytes = config
            .input_bytes_field
            .as_deref()
            .and_then(|field| get_u64_field(value, field));
        let fallback = config
            .fallback_field
            .as_deref()
            .and_then(|field| get_bool_field(value, field))
            .unwrap_or(false);

        let group = grouped.entry((service, operation)).or_default();
        group.durations.push(duration_ms);
        group.total_duration_ms += duration_ms;
        group.max_duration_ms = group.max_duration_ms.max(duration_ms);
        if let Some(bytes) = input_bytes {
            group.total_input_bytes += bytes;
            group.input_sample_count += 1;
        }
        if fallback {
            group.fallback_count += 1;
        }
    }

    let mut summaries: Vec<_> = grouped
        .into_iter()
        .map(|((service, operation), mut group)| {
            group.durations.sort_by(|left, right| left.total_cmp(right));
            let sample_count = group.durations.len();
            let avg_duration_ms = group.total_duration_ms / sample_count as f64;
            let p95_duration_ms = percentile(&group.durations, 0.95);
            let avg_input_bytes = (group.input_sample_count > 0)
                .then(|| group.total_input_bytes as f64 / group.input_sample_count as f64);
            let fallback_rate = group.fallback_count as f64 / sample_count as f64;
            let candidate_score = group.total_duration_ms * (1.0 + fallback_rate);

            AccelOperationSummary {
                operation,
                service,
                sample_count,
                total_duration_ms: group.total_duration_ms,
                avg_duration_ms,
                p95_duration_ms,
                max_duration_ms: group.max_duration_ms,
                total_input_bytes: group.total_input_bytes,
                avg_input_bytes,
                fallback_count: group.fallback_count,
                fallback_rate,
                candidate_score,
            }
        })
        .collect();

    summaries.sort_by(|left, right| {
        right
            .candidate_score
            .total_cmp(&left.candidate_score)
            .then_with(|| right.total_duration_ms.total_cmp(&left.total_duration_ms))
            .then_with(|| right.sample_count.cmp(&left.sample_count))
            .then_with(|| left.operation.cmp(&right.operation))
    });

    summaries
}

pub fn summarize_accel_observation_ndjson(
    data: &[u8],
    config: &AccelScoutConfig,
) -> Vec<AccelOperationSummary> {
    let values = crate::json::parse_ndjson(data);
    summarize_accel_observations(&values, config)
}

fn percentile(sorted_values: &[f64], quantile: f64) -> f64 {
    debug_assert!(!sorted_values.is_empty());
    let index = ((sorted_values.len() as f64 * quantile).ceil() as usize)
        .saturating_sub(1)
        .min(sorted_values.len() - 1);
    sorted_values[index]
}

fn get_string_field(value: &serde_json::Value, field_path: &str) -> Option<String> {
    get_value_field(value, field_path).and_then(|value| match value {
        serde_json::Value::String(text) if !text.is_empty() => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    })
}

fn get_f64_field(value: &serde_json::Value, field_path: &str) -> Option<f64> {
    get_value_field(value, field_path).and_then(|value| match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.parse().ok(),
        _ => None,
    })
}

fn get_u64_field(value: &serde_json::Value, field_path: &str) -> Option<u64> {
    get_value_field(value, field_path).and_then(|value| match value {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.parse().ok(),
        _ => None,
    })
}

fn get_bool_field(value: &serde_json::Value, field_path: &str) -> Option<bool> {
    get_value_field(value, field_path).and_then(|value| match value {
        serde_json::Value::Bool(flag) => Some(*flag),
        serde_json::Value::String(text) => match text.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        serde_json::Value::Number(number) => number.as_u64().map(|value| value != 0),
        _ => None,
    })
}

fn get_value_field<'a>(
    value: &'a serde_json::Value,
    field_path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in field_path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_operation_hot_paths() {
        let data = br#"{"service":"gateway","operation":"parse_jsonrpc","duration_ms":10.0,"input_bytes":100,"fallback":false}
{"service":"gateway","operation":"parse_jsonrpc","duration_ms":20.0,"input_bytes":200,"fallback":true}
{"service":"gateway","operation":"policy_check","duration_ms":2.0,"input_bytes":10,"fallback":false}"#;

        let summaries = summarize_accel_observation_ndjson(data, &AccelScoutConfig::default());

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].operation, "parse_jsonrpc");
        assert_eq!(summaries[0].service.as_deref(), Some("gateway"));
        assert_eq!(summaries[0].sample_count, 2);
        assert_eq!(summaries[0].total_duration_ms, 30.0);
        assert_eq!(summaries[0].avg_duration_ms, 15.0);
        assert_eq!(summaries[0].p95_duration_ms, 20.0);
        assert_eq!(summaries[0].max_duration_ms, 20.0);
        assert_eq!(summaries[0].total_input_bytes, 300);
        assert_eq!(summaries[0].avg_input_bytes, Some(150.0));
        assert_eq!(summaries[0].fallback_count, 1);
        assert_eq!(summaries[0].fallback_rate, 0.5);
    }

    #[test]
    fn supports_custom_nested_fields_and_limits() {
        let data =
            br#"{"svc":"news","name":"chunk","metrics":{"ms":"4","bytes":"1000","fallback":"yes"}}
{"svc":"news","name":"chunk","metrics":{"ms":6,"bytes":2000,"fallback":false}}
{"svc":"news","name":"embed","metrics":{"ms":100,"bytes":3000,"fallback":false}}"#;
        let config = AccelScoutConfig {
            operation_field: "name".to_string(),
            service_field: Some("svc".to_string()),
            duration_ms_field: "metrics.ms".to_string(),
            input_bytes_field: Some("metrics.bytes".to_string()),
            fallback_field: Some("metrics.fallback".to_string()),
            limit: Some(2),
        };

        let summaries = summarize_accel_observation_ndjson(data, &config);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].operation, "chunk");
        assert_eq!(summaries[0].sample_count, 2);
        assert_eq!(summaries[0].total_duration_ms, 10.0);
        assert_eq!(summaries[0].total_input_bytes, 3000);
        assert_eq!(summaries[0].fallback_count, 1);
    }

    #[test]
    fn skips_unusable_observations() {
        let data = br#"{"operation":"ok","duration_ms":1}
{"operation":"missing-duration"}
{"operation":"negative","duration_ms":-1}
{"duration_ms":10}"#;

        let summaries = summarize_accel_observation_ndjson(data, &AccelScoutConfig::default());

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].operation, "ok");
    }
}
