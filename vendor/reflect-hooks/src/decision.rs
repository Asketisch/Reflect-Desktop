//! `HookDecision` — the 5 decisions a hook can return, plus a `Combined` wrapper.
//!
//! See `docs/tools-and-hooks.md §4.2` / `§4.5` for the protocol-level meaning.

use serde::{Deserialize, Serialize};

use reflect_protocol::PermissionMode;

/// A system-level reminder a hook can inject into the next model call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemMessage {
    pub content: String,
}

impl SystemMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

/// What a hook tells the core to do. Multiple `HookDecision`s returned by
/// different hooks for the same event are merged by
/// [`crate::engine::HookEngine::merge`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookDecision {
    /// Allow the operation to proceed.
    Allow,
    /// Reject the operation. For `PreToolUse` the tool does not run; for
    /// `Stop` the turn is forced to continue.
    Deny { reason: String },
    /// Replace the tool's arguments (only meaningful for `PreToolUse`).
    /// Last write wins when multiple hooks return `ModifyArgs`.
    ModifyArgs(serde_json::Value),
    /// Inject a system-level reminder (appended to next model call or to
    /// the tool result). Multiple `InjectMessage`s from different hooks
    /// concatenate.
    InjectMessage(SystemMessage),
    /// Switch the effective permission mode for the upcoming tool call
    /// (only meaningful for `PreToolUse`).
    PermissionOverride(PermissionMode),
    /// Defer to the user: the `ToolExecutionQueue` will route the call
    /// through `ApprovalGate::ask_hook`, emitting `EventMsg::ApprovalRequest`
    /// with `ApprovalKind::Hook`. `reason` is shown in the modal as the
    /// decision preview. M6.
    Ask { reason: String },
    /// Multiple decisions returned by a single hook call.
    Combined(Vec<HookDecision>),
}

impl HookDecision {
    /// Flatten one level of `Combined` and return all leaf decisions.
    pub fn flatten(&self) -> Vec<&HookDecision> {
        match self {
            HookDecision::Combined(inner) => inner.iter().collect(),
            other => vec![other],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_serde_roundtrip() {
        let d = HookDecision::Allow;
        let j = serde_json::to_string(&d).unwrap();
        assert_eq!(j, "{\"type\":\"allow\"}");
        let back: HookDecision = serde_json::from_str(&j).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn deny_carries_reason() {
        let d = HookDecision::Deny {
            reason: "no".into(),
        };
        let back: HookDecision = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn ask_carries_reason() {
        let d = HookDecision::Ask {
            reason: "this is risky".into(),
        };
        let j = serde_json::to_string(&d).unwrap();
        assert!(j.contains(r#""type":"ask""#), "got: {j}");
        let back: HookDecision = serde_json::from_str(&j).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn combined_flattens() {
        let d = HookDecision::Combined(vec![
            HookDecision::Allow,
            HookDecision::Deny { reason: "x".into() },
        ]);
        let leaves = d.flatten();
        assert_eq!(leaves.len(), 2);
    }
}
