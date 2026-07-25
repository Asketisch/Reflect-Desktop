//! Workspace 命令。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

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
        .ok_or_else(|| CommandError {
            msg: "no HOME dir".into(),
        })?;
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
    let raw = std::fs::read_to_string(&path).map_err(|e| CommandError {
        msg: format!("read workspaces.json: {e}"),
    })?;
    let parsed: Vec<WorkspaceInfo> = serde_json::from_str(&raw).map_err(|e| CommandError {
        msg: format!("parse workspaces.json: {e}"),
    })?;
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
pub async fn reflect_current_workspace(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    Ok(agent.workspace().display().to_string())
}
