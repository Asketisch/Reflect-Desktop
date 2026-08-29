//! 文件树 (`reflect_list_dir`) + 文本读 (`reflect_read_file`)。
//!
//! 共享的过滤 / 排序辅助函数给 `search.rs` 复用。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// 返回给前端的目录条目。
#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    /// 文件 / 目录名(非完整路径)。
    pub name: String,
    /// 绝对路径。
    pub path: String,
    /// 取值 `"file" | "dir" | "symlink"`。
    pub kind: String,
    /// 文件字节数(目录时为 0)。
    pub size: u64,
    /// Unix mtime,自 epoch 起秒数。
    pub mtime: i64,
    /// 距所请求根目录的层级(0 表示根目录的直接内容)。
    pub depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirListing {
    pub root: String,
    pub entries: Vec<DirEntry>,
    pub truncated: bool,
}

/// 列出目录内容。跳过隐藏文件 / 常见重型目录(`node_modules`、`.git`、`target` 等)。
/// `max_depth` 默认 4,返回条目上限 2_000,避免 payload 过大。
#[tauri::command]
pub async fn reflect_list_dir(
    agent: State<'_, MinimalAgent>,
    path: Option<String>,
    max_depth: Option<usize>,
) -> CommandResult<DirListing> {
    let requested = match path.as_deref() {
        Some("") | None => agent.workspace().to_path_buf(),
        Some(p) => PathBuf::from(p),
    };
    let root = if requested.is_absolute() {
        requested
    } else {
        agent.workspace().join(&requested)
    };
    if !root.exists() {
        return Err(CommandError {
            msg: format!("path does not exist: {}", root.display()),
        });
    }
    if !root.is_dir() {
        return Err(CommandError {
            msg: format!("not a directory: {}", root.display()),
        });
    }
    let max_d = max_depth.unwrap_or(4);
    let mut entries: Vec<DirEntry> = Vec::new();
    let mut truncated = false;
    const CAP: usize = 2_000;
    walk_dir(&root, &root, 0, max_d, &mut entries, &mut truncated, CAP);
    Ok(DirListing {
        root: root.display().to_string(),
        entries,
        truncated,
    })
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<DirEntry>,
    truncated: &mut bool,
    cap: usize,
) {
    if out.len() >= cap {
        *truncated = true;
        return;
    }
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    let mut sorted: Vec<_> = read.filter_map(|e| e.ok()).collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(depth_first_sort_key(&e)));
    for entry in sorted {
        if out.len() >= cap {
            *truncated = true;
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&name) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let path = entry.path();
        let kind = if meta.is_dir() {
            "dir"
        } else if meta.file_type().is_symlink() {
            "symlink"
        } else {
            "file"
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(DirEntry {
            name,
            path: path.display().to_string(),
            kind: kind.to_string(),
            size: if meta.is_dir() { 0 } else { meta.len() },
            mtime,
            depth,
        });
        if kind == "dir" && depth + 1 < max_depth {
            walk_dir(root, &path, depth + 1, max_depth, out, truncated, cap);
        }
    }
}

pub(crate) fn depth_first_sort_key(entry: &std::fs::DirEntry) -> (u8, String) {
    let name = entry.file_name().to_string_lossy().to_string();
    let is_dir = entry
        .file_type()
        .map(|t| if t.is_dir() { 0 } else { 1 })
        .unwrap_or(1);
    (is_dir, name.to_ascii_lowercase())
}

pub(crate) fn should_skip(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    matches!(
        name,
        "node_modules" | "target" | "dist" | "build" | "__pycache__" | "venv" | ".next" | ".cache"
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileReadResult {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub binary: bool,
    pub truncated: bool,
}

const FILE_READ_MAX: u64 = 1_048_576; // 1 MiB

/// 读取文本文件。超过 `FILE_READ_MAX` 的文件会被拒绝,通过前 8 KiB 内是否
/// 含 NUL 字节判断是否为二进制。
#[tauri::command]
pub async fn reflect_read_file(
    agent: State<'_, MinimalAgent>,
    path: String,
) -> CommandResult<FileReadResult> {
    let resolved = resolve_under_workspace(&agent.workspace(), &path)?;
    let meta = std::fs::metadata(&resolved).map_err(CommandError::from)?;
    if !meta.is_file() {
        return Err(CommandError {
            msg: format!("not a file: {}", resolved.display()),
        });
    }
    let truncated = meta.len() > FILE_READ_MAX;
    let read_n = if truncated { FILE_READ_MAX } else { meta.len() } as usize;

    let mut bytes = vec![0u8; read_n];
    use std::io::Read;
    let mut f = std::fs::File::open(&resolved).map_err(CommandError::from)?;
    f.read_exact(&mut bytes).map_err(CommandError::from)?;

    let probe_n = bytes.len().min(8192);
    let binary = bytes[..probe_n].contains(&0);
    let content = if binary {
        String::new()
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    Ok(FileReadResult {
        path: resolved.display().to_string(),
        content,
        size: meta.len(),
        binary,
        truncated,
    })
}

/// 写回文本文件(编辑器 Reject 恢复原文 / 手动保存)。工作区沙盒内。
///
/// 供编辑器把恢复后的内容写回磁盘:`Reject` 会用 inverse-patch 重建原文,
/// 经此命令落盘。文件不存在时创建(含父目录);按 UTF-8 覆盖写。
#[tauri::command]
pub async fn reflect_write_file(
    agent: State<'_, MinimalAgent>,
    path: String,
    content: String,
) -> CommandResult<String> {
    let resolved = resolve_under_workspace(&agent.workspace(), &path)?;
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(CommandError::from)?;
    }
    std::fs::write(&resolved, content.as_bytes()).map_err(CommandError::from)?;
    tracing::info!(path = %resolved.display(), bytes = content.len(), "file written");
    Ok(resolved.display().to_string())
}

/// 确保 `path` 解析后位于已配置的工作区之下,
/// 防止 `..` 逃逸以及指向工作区外的绝对路径。
pub(crate) fn resolve_under_workspace(workspace: &Path, path: &str) -> CommandResult<PathBuf> {
    let p = Path::new(path);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace.join(p)
    };
    let canon_workspace =
        std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    let canon_joined = std::fs::canonicalize(&joined).unwrap_or(joined);
    if !canon_joined.starts_with(&canon_workspace) {
        return Err(CommandError {
            msg: format!(
                "path escapes workspace: {} (workspace={})",
                canon_joined.display(),
                canon_workspace.display()
            ),
        });
    }
    Ok(canon_joined)
}
