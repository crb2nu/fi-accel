//! Generic event-log helpers for NDJSON/JSONL timeline streams.
//!
//! These helpers normalize append-only event streams into a stable shape that
//! frontends and services can summarize without bespoke per-repo parsing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventProjectionConfig {
    pub timestamp_field: String,
    pub event_type_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_type_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl Default for EventProjectionConfig {
    fn default() -> Self {
        Self {
            timestamp_field: "timestamp".to_string(),
            event_type_field: "event_type".to_string(),
            actor_id_field: Some("agent_id".to_string()),
            actor_type_field: Some("agent_type".to_string()),
            data_field: None,
            limit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventSummary {
    pub total_events: usize,
    pub by_event_type: BTreeMap<String, usize>,
    pub by_actor_id: BTreeMap<String, usize>,
    pub by_actor_type: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_timestamp: Option<String>,
}

pub fn project_events(
    values: &[serde_json::Value],
    config: &EventProjectionConfig,
) -> Vec<EventRecord> {
    let mut events = Vec::new();

    for value in values {
        let Some(event_type) = get_string_field(value, &config.event_type_field) else {
            continue;
        };

        let timestamp = get_string_field(value, &config.timestamp_field);
        let actor_id = config
            .actor_id_field
            .as_deref()
            .and_then(|field| get_string_field(value, field));
        let actor_type = config
            .actor_type_field
            .as_deref()
            .and_then(|field| get_string_field(value, field));
        let data = config
            .data_field
            .as_deref()
            .and_then(|field| get_value_field(value, field).cloned())
            .or_else(|| {
                let payload = value.as_object().map(|object| {
                    object
                        .iter()
                        .filter(|(key, _)| {
                            key.as_str() != config.timestamp_field
                                && key.as_str() != config.event_type_field
                                && config.actor_id_field.as_deref() != Some(key.as_str())
                                && config.actor_type_field.as_deref() != Some(key.as_str())
                        })
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect::<serde_json::Map<String, serde_json::Value>>()
                });

                match payload {
                    Some(map) if !map.is_empty() => Some(serde_json::Value::Object(map)),
                    _ => None,
                }
            });

        events.push(EventRecord {
            timestamp,
            event_type,
            actor_id,
            actor_type,
            data,
        });
    }

    if let Some(limit) = config.limit {
        if events.len() > limit {
            return events[events.len() - limit..].to_vec();
        }
    }

    events
}

pub fn project_eventlog_ndjson(data: &[u8], config: &EventProjectionConfig) -> Vec<EventRecord> {
    let values = crate::json::parse_ndjson(data);
    project_events(&values, config)
}

pub fn summarize_events(events: &[EventRecord]) -> EventSummary {
    let mut by_event_type = BTreeMap::new();
    let mut by_actor_id = BTreeMap::new();
    let mut by_actor_type = BTreeMap::new();
    let mut oldest_timestamp: Option<String> = None;
    let mut newest_timestamp: Option<String> = None;

    for event in events {
        *by_event_type.entry(event.event_type.clone()).or_insert(0) += 1;

        if let Some(actor_id) = &event.actor_id {
            *by_actor_id.entry(actor_id.clone()).or_insert(0) += 1;
        }

        if let Some(actor_type) = &event.actor_type {
            *by_actor_type.entry(actor_type.clone()).or_insert(0) += 1;
        }

        if let Some(timestamp) = &event.timestamp {
            if oldest_timestamp
                .as_ref()
                .is_none_or(|current| timestamp < current)
            {
                oldest_timestamp = Some(timestamp.clone());
            }
            if newest_timestamp
                .as_ref()
                .is_none_or(|current| timestamp > current)
            {
                newest_timestamp = Some(timestamp.clone());
            }
        }
    }

    EventSummary {
        total_events: events.len(),
        by_event_type,
        by_actor_id,
        by_actor_type,
        oldest_timestamp,
        newest_timestamp,
    }
}

pub fn summarize_eventlog_ndjson(data: &[u8], config: &EventProjectionConfig) -> EventSummary {
    let events = project_eventlog_ndjson(data, config);
    summarize_events(&events)
}

fn get_string_field(value: &serde_json::Value, field_path: &str) -> Option<String> {
    get_value_field(value, field_path).and_then(|value| match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
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
    fn projects_timeline_entries_with_defaults() {
        let data = br#"{"timestamp":"2026-03-18T10:00:00Z","event_type":"agent.started","agent_id":"codex","agent_type":"worker","detail":"warm"}
{"timestamp":"2026-03-18T10:01:00Z","event_type":"agent.finished","agent_id":"codex","agent_type":"worker","detail":"done"}"#;

        let events = project_eventlog_ndjson(data, &EventProjectionConfig::default());
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "agent.started");
        assert_eq!(events[0].actor_id.as_deref(), Some("codex"));
        assert!(events[0].data.is_some());
    }

    #[test]
    fn projects_custom_fields_for_audit_logs() {
        let data = br#"{"timestamp":"2026-03-18T10:00:00Z","status":"success","agent_id":"codex","agent_type":"worker","tool":"git.status"}
{"timestamp":"2026-03-18T10:01:00Z","status":"denied","agent_id":"claude","agent_type":"reviewer","tool":"git.push"}"#;

        let config = EventProjectionConfig {
            event_type_field: "status".to_string(),
            ..EventProjectionConfig::default()
        };

        let events = project_eventlog_ndjson(data, &config);
        assert_eq!(events[0].event_type, "success");
        assert_eq!(events[1].event_type, "denied");
    }

    #[test]
    fn summarizes_projected_events() {
        let data = br#"{"timestamp":"2026-03-18T10:00:00Z","event_type":"started","agent_id":"codex","agent_type":"worker"}
{"timestamp":"2026-03-18T10:01:00Z","event_type":"started","agent_id":"codex","agent_type":"worker"}
{"timestamp":"2026-03-18T10:02:00Z","event_type":"finished","agent_id":"claude","agent_type":"reviewer"}"#;

        let summary = summarize_eventlog_ndjson(data, &EventProjectionConfig::default());
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.by_event_type.get("started"), Some(&2));
        assert_eq!(summary.by_actor_id.get("codex"), Some(&2));
        assert_eq!(summary.by_actor_type.get("worker"), Some(&2));
        assert_eq!(
            summary.oldest_timestamp.as_deref(),
            Some("2026-03-18T10:00:00Z")
        );
        assert_eq!(
            summary.newest_timestamp.as_deref(),
            Some("2026-03-18T10:02:00Z")
        );
    }

    #[test]
    fn applies_limit_to_most_recent_events() {
        let data = br#"{"timestamp":"2026-03-18T10:00:00Z","event_type":"one"}
{"timestamp":"2026-03-18T10:01:00Z","event_type":"two"}
{"timestamp":"2026-03-18T10:02:00Z","event_type":"three"}"#;

        let config = EventProjectionConfig {
            limit: Some(2),
            ..EventProjectionConfig::default()
        };

        let events = project_eventlog_ndjson(data, &config);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "two");
        assert_eq!(events[1].event_type, "three");
    }
}
