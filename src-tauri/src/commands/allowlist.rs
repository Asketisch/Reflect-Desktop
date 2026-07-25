//! B13: Approval allowlist —— per-prefix "Always allow" 持久化。
//!
//! 用户在 approval toast 上勾选 "Always allow <prefix>" 后,该 prefix 持久化到
//! `~/.reflect/approval_allowlist.json`,下一个匹配请求直接 approve。
//!
//! 流程:
//!   1. reflect_load_allowlist() — 启动 + 每次 toast 出现之前拉一次。
//!   2. reflect_save_allowlist([...]) — 写盘(去重 + 排序)。
//!   3. reflect_check_allowlist(prefix) — 前端给定 prefix,返回 boolean。

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::commands::error::{CommandError, CommandResult};

fn allowlist_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".reflect/approval_allowlist.json"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Allowlist {
    pub prefixes: Vec<String>,
}

fn read_allowlist() -> Allowlist {
    let Some(path) = allowlist_path() else {
        return Allowlist {
            prefixes: Vec::new(),
        };
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(Allowlist {
            prefixes: Vec::new(),
        }),
        Err(_) => Allowlist {
            prefixes: Vec::new(),
        },
    }
}

fn write_allowlist(list: &Allowlist) -> Result<(), CommandError> {
    let path = allowlist_path().ok_or_else(|| CommandError {
        msg: "no HOME for allowlist".into(),
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(CommandError::from)?;
    }
    let json = serde_json::to_string_pretty(list).map_err(|e| CommandError {
        msg: format!("serialize allowlist: {e}"),
    })?;
    std::fs::write(&path, json).map_err(CommandError::from)?;
    Ok(())
}

#[tauri::command]
pub async fn reflect_load_allowlist() -> CommandResult<Allowlist> {
    Ok(read_allowlist())
}

#[tauri::command]
pub async fn reflect_save_allowlist(list: Allowlist) -> CommandResult<()> {
    // 去重 + 排序 + 修剪空白 + 限制 1024 条。
    let mut set: BTreeSet<String> = BTreeSet::new();
    for p in list.prefixes {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_string());
        }
    }
    let mut deduped: Vec<String> = set.into_iter().collect();
    if deduped.len() > 1024 {
        deduped.truncate(1024);
    }
    let cleaned = Allowlist { prefixes: deduped };
    write_allowlist(&cleaned)?;
    Ok(())
}

#[tauri::command]
pub async fn reflect_check_allowlist(prefix: String) -> CommandResult<bool> {
    let list = read_allowlist();
    let target = prefix.trim();
    if target.is_empty() {
        return Ok(false);
    }
    Ok(list.prefixes.iter().any(|p| target.starts_with(p.as_str())))
}
