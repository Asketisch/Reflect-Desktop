//! Submission — client → core command unit.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::op::Op;

/// A Submission is a single command from the client to the core.
///
/// The `id` correlates the resulting events (which all carry the same `id`)
/// back to the originating submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: String,
    pub op: Op,
    /// Client-supplied user message ID (for tracking across the rollout log).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_user_message_id: Option<String>,
    /// W3C trace context for cross-process tracing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<W3cTraceContext>,
}

impl Submission {
    /// Construct a UserInput submission with auto-generated id.
    pub fn user_input(text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            op: crate::op::Op::user_input_text(text),
            client_user_message_id: None,
            trace: None,
        }
    }

    /// Construct any submission with a pre-assigned id.
    pub fn with_id(id: impl Into<String>, op: Op) -> Self {
        Self {
            id: id.into(),
            op,
            client_user_message_id: None,
            trace: None,
        }
    }
}

/// W3C trace context (subset of W3C Trace Context spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct W3cTraceContext {
    pub trace_id: String,
    pub span_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_flags: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op::Op;

    #[test]
    fn user_input_helper_creates_unique_ids() {
        let a = Submission::user_input("hi");
        let b = Submission::user_input("hi");
        assert_ne!(a.id, b.id);
        assert!(matches!(a.op, Op::UserInput { .. }));
    }

    #[test]
    fn serde_roundtrip() {
        let s = Submission::user_input("hello");
        let json = serde_json::to_string(&s).unwrap();
        let back: Submission = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, s.id);
        assert_eq!(back.op.discriminant(), s.op.discriminant());
    }
}
