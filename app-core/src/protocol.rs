//! `reflect_app_core::protocol` —— Submission / Op 构造器（B2-03）。
//!
//! 参考 TypeScript 中的 `src/protocol/submissions.ts`。TUI 和
//! GUI 都使用这些函数构造 `Submission` 对象，确保拥有唯一的
//! `id`（UUID v4）和正确的 `Op` 结构。TUI/GUI 各自拥有
//! 独立的 ID 策略（TUI 使用原生 UUID，GUI 使用
//! `crypto.randomUUID`）；本模块为进程内构造器（例如 NAPI
//! binding、原生测试）提供权威的 Rust 辅助函数。

use reflect_protocol::{
    AskUserAnswer, Op, PermissionMode, PlanApprovalChoice, ReasoningEffortMirror, ReviewDecision,
    Submission, UserInputItem,
};

/// 使用新的 UUID v4 id 构造 Submission。
pub fn new_submission(op: Op) -> Submission {
    Submission::with_id(uuid::Uuid::new_v4().to_string(), op)
}

/// 便捷函数：从单个文本项构造 `Op::UserInput`。
pub fn user_input_text(text: impl Into<String>) -> Submission {
    new_submission(Op::UserInput {
        items: vec![UserInputItem::Text { text: text.into() }],
        thread_settings: Default::default(),
    })
}

/// 便捷函数：从项列表构造 `Op::UserInput`。
pub fn user_input_items(items: Vec<UserInputItem>) -> Submission {
    new_submission(Op::UserInput {
        items,
        thread_settings: Default::default(),
    })
}

/// 请求压缩历史记录。
pub fn compact() -> Submission {
    new_submission(Op::Compact)
}

/// 中止正在进行的 turn。
pub fn interrupt() -> Submission {
    new_submission(Op::Interrupt { child_id: None })
}

/// 回滚到之前的 turn（回退历史记录）。
pub fn rewind(to_turn_id: Option<String>) -> Submission {
    new_submission(Op::Rewind { to_turn_id })
}

/// 请求 agent 正常关闭。
pub fn shutdown() -> Submission {
    new_submission(Op::Shutdown)
}

/// 提交工具批准决定。
pub fn tool_approval(id: impl Into<String>, decision: ReviewDecision) -> Submission {
    new_submission(Op::ToolApproval {
        id: id.into(),
        decision,
    })
}

/// 提交 hook 批准决定。
pub fn hook_approval(id: impl Into<String>, decision: ReviewDecision) -> Submission {
    new_submission(Op::HookApproval {
        id: id.into(),
        decision,
    })
}

/// 提交 plan 批准决定。
///
/// 注:协议层 `Op::PlanApproval` 的字段从 `decision: ReviewDecision` 升级为
/// `choice: PlanApprovalChoice`(plan 模式专用三选一:AutoMode/ManualApprove/Revise)。
/// 调用方需传入 `PlanApprovalChoice`。
pub fn plan_approval(id: impl Into<String>, choice: PlanApprovalChoice) -> Submission {
    new_submission(Op::PlanApproval {
        id: id.into(),
        choice,
    })
}

/// 为指定任务进入 plan 模式。
pub fn enter_plan_mode(task: impl Into<String>) -> Submission {
    new_submission(Op::EnterPlanMode { task: task.into() })
}

/// 退出 plan 模式。
pub fn exit_plan_mode() -> Submission {
    new_submission(Op::ExitPlanMode)
}

/// 设置后续 turn 的推理强度。
pub fn set_effort(level: ReasoningEffortMirror) -> Submission {
    new_submission(Op::SetEffort { effort: level })
}

/// 设置 permission 模式（例如只读 / 完全访问）。
pub fn set_permission_mode(mode: PermissionMode) -> Submission {
    new_submission(Op::SetPermissionMode { mode })
}

/// 循环切换 permission 模式。
pub fn cycle_permission_mode() -> Submission {
    new_submission(Op::CyclePermissionMode)
}

/// 使用所选答案回复 `AskUserQuestion` event。
pub fn ask_user_question_response(id: impl Into<String>, answers: AskUserAnswer) -> Submission {
    new_submission(Op::AskUserQuestionResponse {
        id: id.into(),
        answers,
    })
}

/// 回复自由格式的 `AskUserInput` prompt。
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
