//! Hooks 列表 / 切换命令。

use serde::Serialize;
use tauri::State;

use crate::commands::error::CommandResult;
use crate::state::MinimalAgent;

#[derive(Debug, Serialize)]
pub struct HookInfo {
    pub name: String,
    pub kind: String, // "read_before_edit" | "plan_mode_gate" | "custom"
    pub enabled: bool,
    pub config_summary: String,
}

#[tauri::command]
pub async fn reflect_list_hooks(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<HookInfo>> {
    agent
        .list_hooks()
        .map_err(crate::commands::error::CommandError::from)
}

#[tauri::command]
pub async fn reflect_toggle_hook(
    agent: State<'_, MinimalAgent>,
    name: String,
    enabled: bool,
) -> CommandResult<()> {
    agent
        .toggle_hook(name, enabled)
        .map_err(crate::commands::error::CommandError::from)
}
