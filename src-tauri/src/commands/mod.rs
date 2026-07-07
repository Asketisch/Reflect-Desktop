//! M1.2 协议桥 —— 14 个 Tauri command, 每个对应 reflect_protocol::Op 的一个变体。

use serde::Serialize;
use tauri::State;

use reflect_protocol::{ReviewDecision, Submission};

use crate::state::MinimalAgent;

/// wrapper for any command errors
#[derive(Debug, Serialize)]
pub struct CommandError {
    msg: String,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<anyhow::Error> for CommandError {
    fn from(e: anyhow::Error) -> Self {
        Self { msg: e.to_string() }
    }
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        Self { msg: s }
    }
}

impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        Self { msg: s.to_string() }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self { msg: e.to_string() }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

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

// ====== 以下 12 个 command 在 M1.2 走 stub: 接受 Submission 构造,转交 MinimalAgent ======

#[tauri::command]
pub async fn reflect_compact() -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_rewind(_to_turn_id: Option<String>) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_shutdown() -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_tool_approval(_id: String, _decision: ReviewDecision) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_hook_approval(_id: String, _decision: ReviewDecision) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_enter_plan_mode(_task: String) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_exit_plan_mode() -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_plan_approval(_id: String, _decision: ReviewDecision) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_set_effort(_level: String) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_ask_user_question_response(
    _id: String,
    _answers: serde_json::Value,
) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_ask_user_input_response(
    _id: String,
    _text: String,
) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_set_permission_mode(_mode: String) -> CommandResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn reflect_cycle_permission_mode() -> CommandResult<()> {
    Ok(())
}

/// 列出所有 14 个 command 用于 `tauri::generate_handler!`。
pub fn all_commands() -> Box<dyn std::any::Any> {
    Box::new((
        reflect_submit,
        reflect_interrupt,
        reflect_compact,
        reflect_rewind,
        reflect_shutdown,
        reflect_tool_approval,
        reflect_hook_approval,
        reflect_enter_plan_mode,
        reflect_exit_plan_mode,
        reflect_plan_approval,
        reflect_set_effort,
        reflect_ask_user_question_response,
        reflect_ask_user_input_response,
        reflect_set_permission_mode,
        reflect_cycle_permission_mode,
        reflect_list_sessions,
        reflect_rename_session,
        reflect_delete_session,
        reflect_replay_session,
    ))
}

// ====== M1.3 session 列表 ======

use std::path::PathBuf;
use reflect_protocol::{RolloutRecord, SessionInfo, ThreadId};
use reflect_rollout::{index as rollout_index, reader as rollout_reader};

fn sessions_base() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".reflect/sessions"))
}

#[tauri::command]
pub async fn reflect_list_sessions() -> CommandResult<Vec<SessionInfo>> {
    let base = sessions_base()
        .ok_or_else(|| CommandError { msg: "no home dir".into() })?;
    rollout_index::list_sessions(&base).map_err(CommandError::from)
}

#[tauri::command]
pub async fn reflect_rename_session(id: ThreadId, new_name: String) -> CommandResult<()> {
    let base = sessions_base()
        .ok_or_else(|| CommandError { msg: "no home dir".into() })?;
    rollout_index::rename_session(&base, id, &new_name).map_err(CommandError::from)
}

#[tauri::command]
pub async fn reflect_delete_session(_id: ThreadId) -> CommandResult<()> {
    // M1.3 占位:实际 archive vs delete 决策在 M2.4 落地。 目前先拒以避免误删。
    Err(CommandError {
        msg: "delete not implemented in M1; use archive in M2.4".into(),
    })
}

#[tauri::command]
pub async fn reflect_replay_session(id: ThreadId) -> CommandResult<Vec<RolloutRecord>> {
    let base = sessions_base()
        .ok_or_else(|| CommandError { msg: "no home dir".into() })?;
    rollout_reader::replay(&base, id)
        .await
        .map_err(CommandError::from)
}
