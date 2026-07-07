//! Event — core → client state unit.

use serde::{Deserialize, Serialize};

use crate::event_msg::EventMsg;

/// Sentinel for events that are not tied to a specific submission
/// (e.g. SessionConfigured, ShutdownComplete).
pub const EVENT_ID_NONE: &str = "";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Matches `Submission.id` when applicable; `EVENT_ID_NONE` otherwise.
    pub id: String,
    pub msg: EventMsg,
}

impl Event {
    pub fn new(id: impl Into<String>, msg: EventMsg) -> Self {
        Self { id: id.into(), msg }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_msg::EventMsg;

    #[test]
    fn event_id_none_sentinel() {
        let e = Event::new(EVENT_ID_NONE, EventMsg::ShutdownComplete);
        assert_eq!(e.id, "");
    }

    #[test]
    fn serde_roundtrip() {
        let e = Event::new("sub-1", EventMsg::ShutdownComplete);
        let json = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "sub-1");
    }
}
