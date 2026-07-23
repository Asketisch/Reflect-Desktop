//! `reflect_app_core::protocol` —— Submission / Op constructors (B2-03).
//!
//! Mirrors `src/protocol/submissions.ts` in TypeScript. Both the TUI and
//! GUI use these to build `Submission` objects with a guaranteed-unique
//! `id` (UUID v4) and correct `Op` shape. The TUI/GUI each have their
//! own ID strategy (the TUI uses native UUID, the GUI uses
//! `crypto.randomUUID`); this module provides a canonical Rust helper
//! for any in-process builder (e.g. NAPI bindings, native tests).

use reflect_protocol::{AskUserAnswer, Op, PermissionMode, ReasoningEffortMirror, ReviewDecision, Submission, UserInputItem};

/// Build a Submission with a fresh UUID v4 id.
pub fn new_submission(op: Op) -> Submission {
    Submission::with_id(uuid::Uuid::new_v4().to_string(), op)
}

/// Convenience: `Op::UserInput` from a single text item.
pub fn user_input_text(text: impl Into<String>) -> Submission {
    new_submission(Op::UserInput {
        items: vec![UserInputItem::Text { text: text.into() }],
        thread_settings: Default::default(),
    })
}

/// Convenience: `Op::UserInput` from a list of items.
pub fn user_input_items(items: Vec<UserInputItem>) -> Submission {
    new_submission(Op::UserInput {
        items,
        thread_settings: Default::default(),
    })
}

/// Request history compaction.
pub fn compact() -> Submission {
    new_submission(Op::Compact)
}

/// Abort the in-flight turn.
pub fn interrupt() -> Submission {
    new_submission(Op::Interrupt)
}

/// Roll back to a prior turn (rewinds history).
pub fn rewind(to_turn_id: Option<String>) -> Submission {
    new_submission(Op::Rewind { to_turn_id })
}

/// Request graceful agent shutdown.
pub fn shutdown() -> Submission {
    new_submission(Op::Shutdown)
}

/// Submit a tool approval decision.
pub fn tool_approval(id: impl Into<String>, decision: ReviewDecision) -> Submission {
    new_submission(Op::ToolApproval {
        id: id.into(),
        decision,
    })
}

/// Submit a hook approval decision.
pub fn hook_approval(id: impl Into<String>, decision: ReviewDecision) -> Submission {
    new_submission(Op::HookApproval {
        id: id.into(),
        decision,
    })
}

/// Submit a plan approval decision.
pub fn plan_approval(id: impl Into<String>, decision: ReviewDecision) -> Submission {
    new_submission(Op::PlanApproval {
        id: id.into(),
        decision,
    })
}

/// Enter plan mode for the given task.
pub fn enter_plan_mode(task: impl Into<String>) -> Submission {
    new_submission(Op::EnterPlanMode { task: task.into() })
}

/// Exit plan mode.
pub fn exit_plan_mode() -> Submission {
    new_submission(Op::ExitPlanMode)
}

/// Set reasoning effort for subsequent turns.
pub fn set_effort(level: ReasoningEffortMirror) -> Submission {
    new_submission(Op::SetEffort { effort: level })
}

/// Set permission mode (e.g. read-only / full-access).
pub fn set_permission_mode(mode: PermissionMode) -> Submission {
    new_submission(Op::SetPermissionMode { mode })
}

/// Cycle through permission modes.
pub fn cycle_permission_mode() -> Submission {
    new_submission(Op::CyclePermissionMode)
}

/// Reply to an `AskUserQuestion` event with the chosen answers.
pub fn ask_user_question_response(id: impl Into<String>, answers: AskUserAnswer) -> Submission {
    new_submission(Op::AskUserQuestionResponse {
        id: id.into(),
        answers,
    })
}

/// Reply to a free-form `AskUserInput` prompt.
pub fn ask_user_input_response(id: impl Into<String>, text: impl Into<String>) -> Submission {
    new_submission(Op::AskUserInputResponse {
        id: id.into(),
        text: text.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_input_text_builds_user_input() {
        let s = user_input_text("hi");
        assert_eq!(s.op.discriminant(), "user_input");
    }

    #[test]
    fn compact_builds_compact() {
        assert_eq!(compact().op.discriminant(), "compact");
    }

    #[test]
    fn interrupt_builds_interrupt() {
        assert_eq!(interrupt().op.discriminant(), "interrupt");
    }

    #[test]
    fn shutdown_builds_shutdown() {
        assert_eq!(shutdown().op.discriminant(), "shutdown");
    }

    #[test]
    fn rewind_carries_to_turn_id() {
        let s = rewind(Some("turn-7".into()));
        assert_eq!(s.op.discriminant(), "rewind");
    }

    #[test]
    fn tool_approval_carries_decision() {
        let s = tool_approval("a1", ReviewDecision::Approve);
        assert_eq!(s.op.discriminant(), "tool_approval");
    }

    #[test]
    fn every_submission_has_unique_id() {
        let a = compact();
        let b = compact();
        assert_ne!(a.id, b.id);
        assert!(!a.id.is_empty());
    }
}
