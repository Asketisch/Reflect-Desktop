//! Redaction pass applied to every [`crate::RolloutRecord`] before it is
//! written to disk.
//!
//! Walks a [`serde_json::Value`] recursively; any `String` longer than
//! [`crate::types::MAX_JSONL_FIELD_CHARS`] is truncated to that length with
//! [`crate::types::JSONL_REDACTION_MARKER`] appended. The pass is idempotent
//! (calling it twice produces the same output as calling it once).
//!
//! We do not maintain a key-name blacklist — provider schemas vary and a
//! missed entry leaks. The size cap catches both known and unknown large
//! fields uniformly.

use crate::types::{JSONL_REDACTION_MARKER, MAX_JSONL_FIELD_CHARS};

/// Redact a JSON value in place. Returns the same value for chaining.
pub fn redact_value(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            if s.len() > MAX_JSONL_FIELD_CHARS {
                let cut = MAX_JSONL_FIELD_CHARS.saturating_sub(JSONL_REDACTION_MARKER.len());
                s.truncate(cut);
                s.push_str(JSONL_REDACTION_MARKER);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for (_k, val) in map.iter_mut() {
                redact_value(val);
            }
        }
        _ => {}
    }
}

/// Convenience: redact then serialize a record into a single JSONL line.
pub fn serialize_redacted(r: &crate::RolloutRecord) -> serde_json::Result<String> {
    let mut v = serde_json::to_value(r)?;
    redact_value(&mut v);
    serde_json::to_string(&v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RolloutRecord;
    use reflect_protocol::{MessageRole, ThreadId, TurnId};

    #[test]
    fn truncates_strings_over_16kb() {
        let mut v = serde_json::json!("x".repeat(20_000));
        redact_value(&mut v);
        let s = v.as_str().unwrap();
        assert!(s.ends_with(JSONL_REDACTION_MARKER));
        assert!(s.len() <= MAX_JSONL_FIELD_CHARS + JSONL_REDACTION_MARKER.len());
    }

    #[test]
    fn preserves_strings_under_16kb() {
        let mut v = serde_json::json!("hello world");
        redact_value(&mut v);
        assert_eq!(v, serde_json::json!("hello world"));
    }

    #[test]
    fn recurses_into_arrays_and_objects() {
        let mut v = serde_json::json!({
            "small": "ok",
            "big": "y".repeat(20_000),
            "nested": {
                "inner": "z".repeat(20_000),
            },
            "list": ["a".repeat(20_000), "short"],
        });
        redact_value(&mut v);
        assert_eq!(v["small"], "ok");
        assert_eq!(v["list"][1], "short");
        assert!(v["big"].as_str().unwrap().ends_with(JSONL_REDACTION_MARKER));
        assert!(
            v["nested"]["inner"]
                .as_str()
                .unwrap()
                .ends_with(JSONL_REDACTION_MARKER)
        );
        assert!(
            v["list"][0]
                .as_str()
                .unwrap()
                .ends_with(JSONL_REDACTION_MARKER)
        );
    }

    #[test]
    fn idempotent() {
        let mut v = serde_json::json!("x".repeat(20_000));
        redact_value(&mut v);
        let once = v.clone();
        redact_value(&mut v);
        assert_eq!(v, once);
    }

    #[test]
    fn serialize_redacted_marks_large_message_content() {
        let r = RolloutRecord::message(
            TurnId::new(),
            MessageRole::Assistant,
            serde_json::json!("y".repeat(20_000)),
        );
        let line = serialize_redacted(&r).unwrap();
        // Find the JSON-escaped marker; quotes are escaped so we look for the raw text.
        assert!(
            line.contains(JSONL_REDACTION_MARKER),
            "missing marker in {line}"
        );
        assert!(
            line.len() <= MAX_JSONL_FIELD_CHARS + 200,
            "line too long: {}",
            line.len()
        );
        // Round-trip via the unredacted re-serialization path is not a strict
        // equality (marker is appended), but the record should still parse.
        let _back: RolloutRecord = serde_json::from_str(&line).unwrap();
    }

    #[test]
    fn _session_meta_roundtrips() {
        // Smoke test for the re-export / path wiring.
        let sid = ThreadId::new();
        let r = RolloutRecord::session_meta(sid, "openai/gpt-4o");
        let line = serialize_redacted(&r).unwrap();
        let back: RolloutRecord = serde_json::from_str(&line).unwrap();
        assert_eq!(back, r);
    }
}
