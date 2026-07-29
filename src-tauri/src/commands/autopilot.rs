//! Autopilot commands.
//!
//! Phase 3 item 10: automatic task scheduling.
//! Thin wrapper around `reflect_app_core::autopilot::AutopilotManager`,
//! exposing its config + run history to the frontend.
//!
//! ## 命令清单
//!
//! - `reflect_get_autopilot_config` — 读取当前配置
//! - `reflect_update_autopilot_config(config)` — 写入新配置
//! - `reflect_autopilot_history` — 拉取最近 run 历史

use reflect_app_core::autopilot::{AutopilotConfig, AutopilotRun};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// Get current autopilot configuration.
#[tauri::command]
pub async fn reflect_get_autopilot_config(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<AutopilotConfig> {
    Ok(agent.autopilot_manager().load_config())
}

/// Update autopilot configuration.
#[tauri::command]
pub async fn reflect_update_autopilot_config(
    agent: State<'_, MinimalAgent>,
    config: AutopilotConfig,
) -> CommandResult<()> {
    agent
        .autopilot_manager()
        .save_config(&config)
        .map_err(|e| CommandError { msg: e.to_string() })
}

/// Get autopilot run history.
#[tauri::command]
pub async fn reflect_autopilot_history(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<Vec<AutopilotRun>> {
    Ok(agent.autopilot_manager().load_history())
}