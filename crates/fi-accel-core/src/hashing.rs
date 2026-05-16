//! Hashing and append-only audit chain helpers.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashChainConfig {
    pub hash_field: String,
    pub prev_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_fields: Option<Vec<String>>,
    pub seed: String,
}

impl Default for HashChainConfig {
    fn default() -> Self {
        Self {
            hash_field: "entry_hash".to_string(),
            prev_field: "prev_hash".to_string(),
            payload_fields: None,
            seed: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashChainVerification {
    pub valid: bool,
    pub total_events: usize,
    pub verified_events: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_prev_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_prev_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn sha256_batch<T: AsRef<[u8]> + Sync>(items: &[T]) -> Vec<String> {
    items
        .par_iter()
        .map(|item| sha256_hex(item.as_ref()))
        .collect()
}

pub fn verify_hash_chain(
    events: &[serde_json::Value],
    config: &HashChainConfig,
) -> HashChainVerification {
    let mut previous_hash = config.seed.clone();

    for (index, event) in events.iter().enumerate() {
        let Some(object) = event.as_object() else {
            return invalid_shape(events.len(), index, "event is not a JSON object");
        };

        let actual_prev_hash = object
            .get(&config.prev_field)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        if actual_prev_hash.as_deref() != Some(previous_hash.as_str()) {
            return HashChainVerification {
                valid: false,
                total_events: events.len(),
                verified_events: index,
                mismatch_index: Some(index),
                expected_hash: None,
                actual_hash: None,
                expected_prev_hash: Some(previous_hash),
                actual_prev_hash,
                error: Some(format!("{} mismatch", config.prev_field)),
            };
        }

        let actual_hash = object
            .get(&config.hash_field)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let payload = payload_for_event(event, config);
        let expected_hash = canonical_sha256(&payload);

        if actual_hash.as_deref() != Some(expected_hash.as_str()) {
            return HashChainVerification {
                valid: false,
                total_events: events.len(),
                verified_events: index,
                mismatch_index: Some(index),
                expected_hash: Some(expected_hash),
                actual_hash,
                expected_prev_hash: Some(previous_hash),
                actual_prev_hash,
                error: Some(format!("{} mismatch", config.hash_field)),
            };
        }

        previous_hash = expected_hash;
    }

    HashChainVerification {
        valid: true,
        total_events: events.len(),
        verified_events: events.len(),
        mismatch_index: None,
        expected_hash: None,
        actual_hash: None,
        expected_prev_hash: None,
        actual_prev_hash: None,
        error: None,
    }
}

pub fn payload_for_event(event: &serde_json::Value, config: &HashChainConfig) -> serde_json::Value {
    let Some(object) = event.as_object() else {
        return serde_json::Value::Null;
    };

    let payload = match &config.payload_fields {
        Some(fields) => fields
            .iter()
            .filter_map(|field| {
                object
                    .get(field)
                    .map(|value| (field.clone(), value.clone()))
            })
            .collect::<serde_json::Map<String, serde_json::Value>>(),
        None => object
            .iter()
            .filter(|(key, _)| key.as_str() != config.hash_field)
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<serde_json::Map<String, serde_json::Value>>(),
    };

    serde_json::Value::Object(payload)
}

pub fn canonical_sha256(value: &serde_json::Value) -> String {
    let canonical = serde_json::to_vec(value).expect("serializing serde_json::Value cannot fail");
    sha256_hex(&canonical)
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn invalid_shape(total_events: usize, index: usize, error: &str) -> HashChainVerification {
    HashChainVerification {
        valid: false,
        total_events,
        verified_events: index,
        mismatch_index: Some(index),
        expected_hash: None,
        actual_hash: None,
        expected_prev_hash: None,
        actual_prev_hash: None,
        error: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn chain_events() -> Vec<serde_json::Value> {
        let mut first = json!({
            "prev_hash": "",
            "event_type": "created",
            "actor": "codex",
            "payload": {"count": 1}
        });
        let first_hash = canonical_sha256(&first);
        first["entry_hash"] = json!(first_hash);

        let mut second = json!({
            "prev_hash": first["entry_hash"].as_str().unwrap(),
            "event_type": "updated",
            "actor": "codex",
            "payload": {"count": 2}
        });
        let second_hash = canonical_sha256(&second);
        second["entry_hash"] = json!(second_hash);

        vec![first, second]
    }

    #[test]
    fn hashes_batch_with_sha256_hex() {
        assert_eq!(
            sha256_batch(&[b"hello".as_slice(), b"world".as_slice()]),
            vec![
                "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
                "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7"
            ]
        );
    }

    #[test]
    fn verifies_valid_hash_chain() {
        let result = verify_hash_chain(&chain_events(), &HashChainConfig::default());
        assert!(result.valid);
        assert_eq!(result.total_events, 2);
        assert_eq!(result.verified_events, 2);
    }

    #[test]
    fn rejects_tampered_payload() {
        let mut events = chain_events();
        events[1]["payload"]["count"] = json!(3);

        let result = verify_hash_chain(&events, &HashChainConfig::default());
        assert!(!result.valid);
        assert_eq!(result.mismatch_index, Some(1));
        assert_eq!(result.error.as_deref(), Some("entry_hash mismatch"));
    }

    #[test]
    fn accepts_empty_chain() {
        let result = verify_hash_chain(&[], &HashChainConfig::default());
        assert!(result.valid);
        assert_eq!(result.total_events, 0);
        assert_eq!(result.verified_events, 0);
    }

    #[test]
    fn verifies_configured_payload_fields_and_seed() {
        let config = HashChainConfig {
            payload_fields: Some(vec!["prev_hash".to_string(), "payload".to_string()]),
            seed: "root".to_string(),
            ..HashChainConfig::default()
        };
        let mut event = json!({"prev_hash": "root", "payload": {"ok": true}, "ignored": "x"});
        let payload = payload_for_event(&event, &config);
        event["entry_hash"] = json!(canonical_sha256(&payload));

        let result = verify_hash_chain(&[event], &config);
        assert!(result.valid);
    }
}
