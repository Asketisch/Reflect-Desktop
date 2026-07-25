//! Memory 增删查命令。

use serde::Serialize;
use tauri::State;

use crate::commands::error::CommandResult;
use crate::state::MinimalAgent;

#[derive(Debug, Serialize)]
pub struct MemoryEntry {
    pub scope: String, // "project" | "user" | "session"
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub async fn reflect_list_memory(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<Vec<MemoryEntry>> {
    agent
        .list_memory()
        .map_err(crate::commands::error::CommandError::from)
}

#[tauri::command]
pub async fn reflect_add_memory(
    agent: State<'_, MinimalAgent>,
    scope: String,
    key: String,
    value: String,
) -> CommandResult<()> {
    agent
        .add_memory(scope, key, value)
        .map_err(crate::commands::error::CommandError::from)
}

#[tauri::command]
pub async fn reflect_remove_memory(
    agent: State<'_, MinimalAgent>,
    scope: String,
    key: String,
) -> CommandResult<()> {
    agent
        .remove_memory(scope, key)
        .map_err(crate::commands::error::CommandError::from)
}
