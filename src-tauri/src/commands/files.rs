//! B9-01: 文件树 (`reflect_list_dir`) + 文本读 (`reflect_read_file`)。
//!
//! 共享的过滤 / 排序辅助函数给 `search.rs` 复用。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// Directory entry returned to the frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    /// File/dir name (not full path).
    pub name: String,
    /// Absolute path.
    pub path: String,
    /// "file" | "dir" | "symlink".
    pub kind: String,
    /// File size in bytes (0 for dirs).
    pub size: u64,
    /// Unix mtime in seconds since epoch.
    pub mtime: i64,
    /// Depth from the requested root (0 = root contents).
    pub depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirListing {
    pub root: String,
    pub entries: Vec<DirEntry>,
    pub truncated: bool,
}

/// List a directory. Skips dotfiles/hidden + common heavy dirs (node_modules, .git, target).
/// `max_depth` defaults to 4; caps returned entries at 2_000 to keep payloads small.
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

/// Read a text file. Rejects files larger than `FILE_READ_MAX` and detects
/// binary by looking for NUL bytes in the first 8 KiB.
#[tauri::command]
pub async fn reflect_read_file(
    agent: State<'_, MinimalAgent>,
    path: String,
) -> CommandResult<FileReadResult> {
    let resolved = resolve_under_workspace(agent.workspace(), &path)?;
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

/// Ensure `path` resolves under the configured workspace, defending against
/// `..` escapes and absolute paths outside the workspace.
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
