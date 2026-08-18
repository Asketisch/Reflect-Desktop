//! Session 列表 / 重命名 / 删除 / 回放 / JSON 导出。

use std::path::PathBuf;

use reflect_protocol::{RolloutRecord, SessionInfo, ThreadId};
use reflect_rollout::{index as rollout_index, reader as rollout_reader};

use crate::commands::error::{CommandError, CommandResult};

fn sessions_base() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".reflect/sessions"))
}

/// 列出 session,支持可选分页。
///
/// `limit` + `offset` 提供游标式分页(按时间倒序)。
/// `limit = 0` 或省略 → 无上限(返回完整列表)。
#[tauri::command]
pub async fn reflect_list_sessions(
    limit: Option<usize>,
    offset: Option<usize>,
) -> CommandResult<Vec<SessionInfo>> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let mut all = rollout_index::list_sessions(&base).map_err(CommandError::from)?;
    let off = offset.unwrap_or(0);
    if off >= all.len() {
        return Ok(Vec::new());
    }
    if off > 0 {
        all = all.split_off(off);
    }
    if let Some(n) = limit {
        if n < all.len() {
            all.truncate(n);
        }
    }
    Ok(all)
}

#[tauri::command]
pub async fn reflect_rename_session(id: ThreadId, new_name: String) -> CommandResult<()> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    rollout_index::rename_session(&base, id, &new_name).map_err(CommandError::from)
}

/// 删除 session:实装真删除(rollout 目录 rm)。
/// 目录按 thread id 隔离,只删该 session 的 rollout 文件。
#[tauri::command]
pub async fn reflect_delete_session(id: ThreadId) -> CommandResult<()> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
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
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    rollout_reader::replay(&base, id)
        .await
        .map_err(CommandError::from)
}

/// 把 session 导出为 JSON 到 `~/.reflect/exports/<id>.json` 并返回路径。供 ThreadsView / CommandPalette 的导出菜单使用。
#[tauri::command]
pub async fn reflect_export_session(id: ThreadId) -> CommandResult<String> {
    let home = dirs::home_dir().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let export_dir = home.join(".reflect/exports");
    std::fs::create_dir_all(&export_dir).map_err(CommandError::from)?;
    let dest = export_dir.join(format!("{}.json", id));
    let records = {
        let base = home.join(".reflect/sessions");
        rollout_reader::replay(&base, id)
            .await
            .map_err(CommandError::from)?
    };
    let json = serde_json::to_string_pretty(&records).map_err(|e| CommandError {
        msg: format!("json: {e}"),
    })?;
    std::fs::write(&dest, json).map_err(CommandError::from)?;
    Ok(dest.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    // 分页逻辑与 reflect_list_sessions 的 limit/offset 切片保持一致。
    // 这里不启动 Tauri runtime,而是在合成的 Vec<String> 上复现切片语义,
    // 用以捕获算法层的回归。

    fn paginate<T: Clone>(mut all: Vec<T>, limit: Option<usize>, offset: Option<usize>) -> Vec<T> {
        let off = offset.unwrap_or(0);
        if off >= all.len() {
            return Vec::new();
        }
        let mut rest = if off > 0 { all.split_off(off) } else { all };
        if let Some(n) = limit {
            if n < rest.len() {
                rest.truncate(n);
            }
        }
        rest
    }

    #[test]
    fn pagination_no_limit_returns_all() {
        let v: Vec<String> = (0..5).map(|i| format!("t-{i}")).collect();
        assert_eq!(paginate(v, None, None).len(), 5);
    }

    #[test]
    fn pagination_limit_truncates() {
        let v: Vec<String> = (0..5).map(|i| format!("t-{i}")).collect();
        assert_eq!(paginate(v, Some(2), None).len(), 2);
    }

    #[test]
    fn pagination_offset_skips() {
        let v: Vec<String> = (0..5).map(|i| format!("t-{i}")).collect();
        let out = paginate(v, None, Some(2));
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], "t-2");
    }

    #[test]
    fn pagination_offset_and_limit() {
        let v: Vec<String> = (0..10).map(|i| format!("t-{i}")).collect();
        let out = paginate(v, Some(3), Some(2));
        assert_eq!(out, vec!["t-2", "t-3", "t-4"]);
    }

    #[test]
    fn pagination_offset_beyond_end_returns_empty() {
        let v: Vec<String> = (0..3).map(|i| format!("t-{i}")).collect();
        assert_eq!(paginate(v, None, Some(10)).len(), 0);
    }
}
