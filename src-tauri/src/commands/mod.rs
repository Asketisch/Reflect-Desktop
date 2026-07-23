//! 协议桥 —— Tauri command 表。
//!
//! 每个 command 对应 `reflect_protocol::Op` 的一个变体(或 session/config 诊断)。
//! 12 个 Op 命令(compact / rewind / approval / plan / effort / permission / ask_user)
//! 不再是空 `Ok(())`,而是构造 `Op` 经 `MinimalAgent::submit_op` 真正驱动 AgentThread。

use serde::{Deserialize, Serialize};
use tauri::State;

use reflect_config::{default_config_path, load_from_str};
use reflect_protocol::{
    AskUserAnswer, Op, PermissionMode, ReasoningEffortMirror, ReviewDecision, Submission, ThreadId,
};
use reflect_rollout::{index as rollout_index, reader as rollout_reader};

use crate::state::{AgentStatus, MinimalAgent};

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

impl From<reflect_config::ConfigError> for CommandError {
    fn from(e: reflect_config::ConfigError) -> Self {
        Self { msg: e.to_string() }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

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

// ====== 12 个 Op 命令:构造 Op 经 submit_op 真正驱动 AgentThread ======

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
    decision: ReviewDecision,
) -> CommandResult<String> {
    Ok(agent.submit_op(Op::PlanApproval { id, decision }).await?)
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
    let answers: AskUserAnswer = serde_json::from_value(answers)
        .map_err(|e| CommandError { msg: format!("invalid answers: {e}") })?;
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
    Ok(agent.submit_op(Op::AskUserInputResponse { id, text }).await?)
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

// ====== 诊断 / config / tools 命令 ======

/// 返回 agent 状态快照(ready / has_model / model / workspace / degraded_reason)。
/// 前端用来显示状态徽标 + 引导用户去 Settings 配 API key。
#[tauri::command]
pub async fn reflect_agent_status(agent: State<'_, MinimalAgent>) -> CommandResult<AgentStatus> {
    Ok(agent.agent_status())
}

/// 返回当前 `~/.reflect/config.toml` 的 TOML 字符串。Settings 页加载用。
#[tauri::command]
pub async fn reflect_get_config(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    let cfg = agent.cfg();
    let toml = toml::to_string_pretty(&*cfg.read())
        .map_err(|e| CommandError { msg: format!("serialize config: {e}") })?;
    Ok(toml)
}

/// 写回 `~/.reflect/config.toml`。写盘前用 `load_from_str` 校验合法性,
/// 防止坏 TOML 损坏配置;校验通过才覆盖,并热更新共享 cfg。
///
/// 注意:**不**在这里直接重建 ModelRegistry / 重连 MCP —— 那些留给
/// `ConfigWatcher` 热重载流程(阶段 4 接入)。
#[tauri::command]
pub async fn reflect_save_config(
    agent: State<'_, MinimalAgent>,
    toml: String,
) -> CommandResult<()> {
    // 1. 校验:能否解析回 ReflectConfig。
    let new_cfg = load_from_str(&toml)?;
    // 2. 写盘。
    let path = default_config_path()
        .ok_or_else(|| CommandError { msg: "no HOME dir for config".into() })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &toml)?;
    // 3. 热更新共享 cfg(下一个 turn 读最新值)。
    *agent.cfg().write() = new_cfg;
    tracing::info!(
        "[reflect-gui] config saved to {} (hot-reloaded in-memory)",
        path.display()
    );
    Ok(())
}

/// 列出当前 ToolRegistry 中所有工具(name + description)。
/// 前端 Settings / Skills 页展示可用工具列表用。
#[tauri::command]
pub async fn reflect_list_tools(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<ToolInfo>> {
    let tools = agent.tools();
    let mut out: Vec<ToolInfo> = tools
        .list()
        .into_iter()
        .filter_map(|name| {
            let t = tools.get(&name)?;
            Some(ToolInfo {
                name,
                description: t.description().to_string(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 单个工具的 name + description。
#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

// ====== session 列表(M1.3) ======

use std::path::PathBuf;
use reflect_protocol::{RolloutRecord, SessionInfo};

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

/// 删除 session:实装真删除(rollout 目录 rm)。
/// 目录按 thread id 隔离,只删该 session 的 rollout 文件。
#[tauri::command]
pub async fn reflect_delete_session(id: ThreadId) -> CommandResult<()> {
    let base = sessions_base()
        .ok_or_else(|| CommandError { msg: "no home dir".into() })?;
    let dir = base.join(id.to_string());
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(CommandError::from)?;
        tracing::info!("[reflect-gui] deleted session {}", id);
    } else {
        tracing::debug!("[reflect-gui] delete_session: {} not found (no-op)", id);
    }
    Ok(())
}

#[tauri::command]
pub async fn reflect_replay_session(id: ThreadId) -> CommandResult<Vec<RolloutRecord>> {
    let base = sessions_base()
        .ok_or_else(|| CommandError { msg: "no home dir".into() })?;
    rollout_reader::replay(&base, id)
        .await
        .map_err(CommandError::from)
}

// ====== Workspace (B1-07 / B9-06) ======

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: String,
    pub label: String,
    pub last_used: u64,
    pub session_count: usize,
}

/// List known workspaces (from `~/.reflect/workspaces.json`).
#[tauri::command]
pub async fn reflect_list_workspaces() -> CommandResult<Vec<WorkspaceInfo>> {
    // Phase 1: read from `~/.reflect/workspaces.json` if present, else
    // return the current workspace only. The file is a small JSON
    // `[{ path, label, last_used, session_count }]` array.
    let path = dirs::home_dir()
        .map(|h| h.join(".reflect/workspaces.json"))
        .ok_or_else(|| CommandError { msg: "no HOME dir".into() })?;
    if !path.exists() {
        let cur = dirs::home_dir()
            .map(|h| h.join(".").display().to_string())
            .unwrap_or_else(|| ".".to_string());
        return Ok(vec![WorkspaceInfo {
            path: cur,
            label: "default".to_string(),
            last_used: 0,
            session_count: 0,
        }]);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| CommandError { msg: format!("read workspaces.json: {e}") })?;
    let parsed: Vec<WorkspaceInfo> = serde_json::from_str(&raw)
        .map_err(|e| CommandError { msg: format!("parse workspaces.json: {e}") })?;
    Ok(parsed)
}

#[tauri::command]
pub async fn reflect_set_workspace(
    agent: State<'_, MinimalAgent>,
    path: String,
) -> CommandResult<()> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(CommandError {
            msg: format!("workspace path does not exist: {path}"),
        });
    }
    if !p.is_dir() {
        return Err(CommandError {
            msg: format!("workspace path is not a directory: {path}"),
        });
    }
    agent.set_workspace(p);
    Ok(())
}

#[tauri::command]
pub async fn reflect_current_workspace(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<String> {
    Ok(agent.workspace().display().to_string())
}

// ====== Skills (B1-07 / B11-06) ======

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: String,
    pub tools: Vec<String>,
    pub triggers: Vec<String>,
}

#[tauri::command]
pub async fn reflect_list_skills() -> CommandResult<Vec<SkillInfo>> {
    // Phase 1: scan `~/.reflect/skills/**/SKILL.md` and `<cwd>/.reflect/skills/**/SKILL.md`.
    let mut out = Vec::new();
    let search_dirs: Vec<PathBuf> = [
        dirs::home_dir().map(|h| h.join(".reflect/skills")),
        std::env::current_dir().ok().map(|c| c.join(".reflect/skills")),
    ]
    .into_iter()
    .flatten()
    .collect();
    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        collect_skills_in(&dir, &mut out)?;
    }
    Ok(out)
}

fn collect_skills_in(dir: &PathBuf, out: &mut Vec<SkillInfo>) -> CommandResult<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_skills_in(&path, out)?;
        } else if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(info) = parse_skill_frontmatter(&content, &path) {
                    out.push(info);
                }
            }
        }
    }
    Ok(())
}

fn parse_skill_frontmatter(content: &str, path: &PathBuf) -> Option<SkillInfo> {
    // Minimal YAML-frontmatter parser: extract the first `---\n...\n---` block.
    let stripped = content.strip_prefix("---")?;
    let rest = stripped.trim_start_matches('\n');
    let end = rest.find("\n---")?;
    let yaml = &rest[..end];
    let body = rest[end + 4..].trim();

    let name = extract_yaml_field(yaml, "name").unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    });
    let description = extract_yaml_field(yaml, "description").unwrap_or_default();
    let tools = extract_yaml_list(yaml, "tools");
    let triggers = extract_yaml_list(yamml_safe(yaml), "triggers");

    let _ = body; // body unused in summary; surfaced in future `/skills/<name>` detail.

    Some(SkillInfo {
        name,
        description,
        path: path.display().to_string(),
        tools,
        triggers,
    })
}

fn yamml_safe(s: &str) -> &str {
    s
}

fn extract_yaml_field(yaml: &str, key: &str) -> Option<String> {
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn extract_yaml_list(yaml: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_list = false;
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            // inline [a, b, c]
            if rest.trim_start().starts_with('[') {
                let inside = rest.trim_start().trim_start_matches('[').trim_end_matches(']');
                for item in inside.split(',') {
                    let s = item.trim().trim_matches('"').trim_matches('\'');
                    if !s.is_empty() {
                        out.push(s.to_string());
                    }
                }
                in_list = false;
            } else if rest.trim().is_empty() {
                in_list = true;
            } else {
                return out; // single value, not a list
            }
            continue;
        }
        if in_list {
            if let Some(item) = trimmed.strip_prefix("- ") {
                let s = item.trim().trim_matches('"').trim_matches('\'');
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            } else if !trimmed.is_empty() {
                in_list = false;
            }
        }
    }
    out
}

// ====== Memory (B1-07 / B11-01) ======

#[derive(Debug, Serialize)]
pub struct MemoryEntry {
    pub scope: String, // "project" | "user" | "session"
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub async fn reflect_list_memory(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<MemoryEntry>> {
    agent.list_memory().map_err(CommandError::from)
}

#[tauri::command]
pub async fn reflect_add_memory(
    agent: State<'_, MinimalAgent>,
    scope: String,
    key: String,
    value: String,
) -> CommandResult<()> {
    agent.add_memory(scope, key, value).map_err(CommandError::from)
}

#[tauri::command]
pub async fn reflect_remove_memory(
    agent: State<'_, MinimalAgent>,
    scope: String,
    key: String,
) -> CommandResult<()> {
    agent.remove_memory(scope, key).map_err(CommandError::from)
}

// ====== Hooks (B1-07 / B11-02) ======

#[derive(Debug, Serialize)]
pub struct HookInfo {
    pub name: String,
    pub kind: String, // "read_before_edit" | "plan_mode_gate" | "custom"
    pub enabled: bool,
    pub config_summary: String,
}

#[tauri::command]
pub async fn reflect_list_hooks(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<HookInfo>> {
    agent.list_hooks().map_err(CommandError::from)
}

#[tauri::command]
pub async fn reflect_toggle_hook(
    agent: State<'_, MinimalAgent>,
    name: String,
    enabled: bool,
) -> CommandResult<()> {
    agent.toggle_hook(name, enabled).map_err(CommandError::from)
}

// ====== 辅助:字符串 → 强类型 ======

/// `"low"|"medium"|"high"` → `ReasoningEffortMirror`。
fn parse_effort(s: &str) -> CommandResult<ReasoningEffortMirror> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "low" => ReasoningEffortMirror::Low,
        "medium" => ReasoningEffortMirror::Medium,
        "high" => ReasoningEffortMirror::High,
        other => {
            return Err(CommandError {
                msg: format!("invalid effort '{other}'; expected low|medium|high"),
            })
        }
    })
}

/// `"auto"|"prompt"|"deny"|"plan"` → `PermissionMode`。
fn parse_permission_mode(s: &str) -> CommandResult<PermissionMode> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "auto" => PermissionMode::Auto,
        "prompt" => PermissionMode::Prompt,
        "deny" => PermissionMode::Deny,
        "plan" => PermissionMode::Plan,
        other => {
            return Err(CommandError {
                msg: format!("invalid permission mode '{other}'; expected auto|prompt|deny|plan"),
            })
        }
    })
}

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
