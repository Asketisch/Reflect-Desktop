//! 核心 agent 桥接:`reflect_submit` / `reflect_interrupt` + 14 个 Op 命令。
//!
//! 每个 Op 命令构造一个 `Op`,经 `MinimalAgent::submit_op` 真正驱动 `AgentThread`。

use tauri::State;

use reflect_protocol::{AskUserAnswer, Op, PermissionMode, ReasoningEffortMirror, Submission};

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

// ====== 核心:用户提交 + 中断 ======

/// 把 Submission 通过 MinimalAgent 发送到后端。返回 submission.id。
///
/// 前端 invoke 形态:`invoke<string>('reflect_submit', { submission })`。
#[tauri::command]
pub async fn reflect_submit(
    agent: State<'_, MinimalAgent>,
    submission: Submission,
) -> CommandResult<String> {
    let id = submission.id.clone();
    agent.submit(submission).await?;
    Ok(id)
}

#[tauri::command]
pub async fn reflect_interrupt(agent: State<'_, MinimalAgent>) -> CommandResult<()> {
    agent.interrupt();
    Ok(())
}

// ====== 14 个 Op 命令:构造 Op 经 submit_op 真正驱动 AgentThread ======

#[tauri::command]
pub async fn reflect_compact(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    Ok(agent.submit_op(Op::Compact).await?)
}

#[tauri::command]
pub async fn reflect_rewind(
    agent: State<'_, MinimalAgent>,
    to_turn_id: Option<String>,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::Rewind { to_turn_id }).await?)
}

#[tauri::command]
pub async fn reflect_shutdown(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    Ok(agent.submit_op(Op::Shutdown).await?)
}

#[tauri::command]
pub async fn reflect_tool_approval(
    agent: State<'_, MinimalAgent>,
    id: String,
    decision: ReviewDecision,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::ToolApproval { id, decision }).await?)
}

#[tauri::command]
pub async fn reflect_hook_approval(
    agent: State<'_, MinimalAgent>,
    id: String,
    decision: ReviewDecision,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::HookApproval { id, decision }).await?)
}

#[tauri::command]
pub async fn reflect_enter_plan_mode(
    agent: State<'_, MinimalAgent>,
    task: String,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::EnterPlanMode { task }).await?)
}

#[tauri::command]
pub async fn reflect_exit_plan_mode(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    Ok(agent.submit_op(Op::ExitPlanMode).await?)
}

#[tauri::command]
pub async fn reflect_plan_approval(
    agent: State<'_, MinimalAgent>,
    id: String,
    choice: PlanApprovalChoice,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::PlanApproval { id, choice }).await?)
}

/// 设置 reasoning effort。前端传字符串 `"low"|"medium"|"high"`,
/// 这里转成 `ReasoningEffortMirror`。
#[tauri::command]
pub async fn reflect_set_effort(
    agent: State<'_, MinimalAgent>,
    level: String,
) -> CommandResult<String> {
    let effort = parse_effort(&level)?;
    Ok(agent.submit_op(Op::SetEffort { effort }).await?)
}

#[tauri::command]
pub async fn reflect_ask_user_question_response(
    agent: State<'_, MinimalAgent>,
    id: String,
    answers: serde_json::Value,
) -> CommandResult<String> {
    // 前端传 JSON,这里反序列化成 AskUserAnswer(结构化)。
    let answers: AskUserAnswer = serde_json::from_value(answers).map_err(|e| CommandError {
        msg: format!("invalid answers: {e}"),
    })?;
    Ok(agent
        .submit_op(Op::AskUserQuestionResponse { id, answers })
        .await?)
}

#[tauri::command]
pub async fn reflect_ask_user_input_response(
    agent: State<'_, MinimalAgent>,
    id: String,
    text: String,
) -> CommandResult<String> {
    Ok(agent
        .submit_op(Op::AskUserInputResponse { id, text })
        .await?)
}

#[tauri::command]
pub async fn reflect_set_permission_mode(
    agent: State<'_, MinimalAgent>,
    mode: String,
) -> CommandResult<String> {
    let mode = parse_permission_mode(&mode)?;
    Ok(agent.submit_op(Op::SetPermissionMode { mode }).await?)
}

#[tauri::command]
pub async fn reflect_cycle_permission_mode(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::CyclePermissionMode).await?)
}

// ====== 目标模式(v1.2 P1,reflect-goal 编排)======

/// 进入目标模式:`Op::EnterGoalMode`。core 侧构造 `GoalController`,
/// 后续每轮 turn 结束自校验,未完成则 steering 续作,完成则退出。
///
/// 前端 invoke 形态:`invoke<string>('reflect_enter_goal_mode', { goal, verifyCommand?, tokenBudget? })`。
#[tauri::command]
pub async fn reflect_enter_goal_mode(
    agent: State<'_, MinimalAgent>,
    goal: String,
    verify_command: Option<String>,
    token_budget: Option<u64>,
) -> CommandResult<String> {
    Ok(agent
        .submit_op(Op::EnterGoalMode {
            goal,
            verify_command,
            token_budget,
        })
        .await?)
}

/// 退出目标模式:`Op::ExitGoalMode`,停止自动续作。
#[tauri::command]
pub async fn reflect_exit_goal_mode(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    Ok(agent.submit_op(Op::ExitGoalMode).await?)
}

// ====== 辅助:字符串 → 强类型 ======

/// `"low"|"medium"|"high"` → `ReasoningEffortMirror`。
pub(crate) fn parse_effort(s: &str) -> CommandResult<ReasoningEffortMirror> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "low" => ReasoningEffortMirror::Low,
        "medium" => ReasoningEffortMirror::Medium,
        "high" => ReasoningEffortMirror::High,
        other => {
            return Err(CommandError {
                msg: format!("invalid effort '{other}'; expected low|medium|high"),
            });
        }
    })
}

/// `"auto"|"prompt"|"deny"|"plan"|"accept_edits"|"bubble"|"bypass"` → `PermissionMode`。
pub(crate) fn parse_permission_mode(s: &str) -> CommandResult<PermissionMode> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "auto" => PermissionMode::Auto,
        "prompt" => PermissionMode::Prompt,
        "deny" => PermissionMode::Deny,
        "plan" => PermissionMode::Plan,
        "accept_edits" => PermissionMode::AcceptEdits,
        "bubble" => PermissionMode::Bubble,
        "bypass" => PermissionMode::Bypass,
        other => {
            return Err(CommandError {
                msg: format!("invalid permission mode '{other}'; expected auto|prompt|deny|plan|accept_edits|bubble|bypass"),
            });
        }
    })
}

use reflect_protocol::{PlanApprovalChoice, ReviewDecision};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_effort_round_trip() {
        assert_eq!(parse_effort("low").unwrap(), ReasoningEffortMirror::Low);
        assert_eq!(
            parse_effort("Medium").unwrap(),
            ReasoningEffortMirror::Medium
        );
        assert_eq!(parse_effort("HIGH").unwrap(), ReasoningEffortMirror::High);
        assert!(parse_effort("nope").is_err());
    }

    #[test]
    fn parse_permission_mode_round_trip() {
        assert_eq!(parse_permission_mode("auto").unwrap(), PermissionMode::Auto);
        assert_eq!(
            parse_permission_mode("prompt").unwrap(),
            PermissionMode::Prompt
        );
        assert_eq!(parse_permission_mode("deny").unwrap(), PermissionMode::Deny);
        assert_eq!(parse_permission_mode("plan").unwrap(), PermissionMode::Plan);
        assert_eq!(parse_permission_mode("accept_edits").unwrap(), PermissionMode::AcceptEdits);
        assert_eq!(parse_permission_mode("bubble").unwrap(), PermissionMode::Bubble);
        assert_eq!(parse_permission_mode("bypass").unwrap(), PermissionMode::Bypass);
        assert!(parse_permission_mode("wat").is_err());
    }

    #[test]
    fn review_decision_serde_round_trip() {
        // ReviewDecision 用 #[serde(rename_all = "snake_case")] —— 前端必须传
        // "approve" / "approve_for_session" / {"deny":{"reason":"..."}}。
        let approve = serde_json::from_str::<ReviewDecision>("\"approve\"").unwrap();
        assert!(matches!(approve, ReviewDecision::Approve));
        // deny 是 struct variant,payload 必须是 {"reason":"..."} 嵌套对象。
        let deny =
            serde_json::from_str::<ReviewDecision>(r#"{"deny":{"reason":"too risky"}}"#).unwrap();
        assert!(matches!(deny, ReviewDecision::Deny { .. }));
        // PascalCase 应被拒绝,防止前端误用。
        assert!(serde_json::from_str::<ReviewDecision>("\"Approve\"").is_err());
    }
}
