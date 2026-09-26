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
    walk_dir(&root, 0, max_d, &mut entries, &mut truncated, CAP);
    Ok(DirListing {
        root: root.display().to_string(),
        entries,
        truncated,
    })
}

fn walk_dir(
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
    sorted.sort_by_key(|e| std::cmp::Reverse(depth_first_sort_key(e)));
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
            walk_dir(&path, depth + 1, max_depth, out, truncated, cap);
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
    // `..` 组件一律拒绝:目标文件尚不存在时 canonicalize 失败会退回原始
    // join 路径,而 `Path::starts_with` 是纯词法前缀比较,
    // `ws/../etc/x` 这类路径会绕过校验被写到工作区外。
    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(CommandError {
            msg: format!("path must not contain '..': {path}"),
        });
    }
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace.join(p)
    };
    let canon_workspace =
        std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.to_path_buf());
    // 目标可能尚不存在(新建文件):此时 canonicalize 最深的已存在祖先目录
    // 再拼回剩余组件,防止父级符号链接把真实路径指到工作区外。
    let canon_joined = std::fs::canonicalize(&joined).unwrap_or_else(|_| {
        for ancestor in joined.ancestors().skip(1) {
            if let Ok(canon) = std::fs::canonicalize(ancestor) {
                if let Ok(suffix) = joined.strip_prefix(ancestor) {
                    return canon.join(suffix);
                }
            }
        }
        joined.clone()
    });
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

/// `reflect_read_image_base64` 的返回:图片字节(base64)+ MIME。
#[derive(Debug, Serialize)]
pub struct ImageBase64Result {
    /// 文件绝对路径。
    pub path: String,
    /// 按扩展名推断的 MIME(`image/png` 等)。
    pub mime_type: String,
    /// 文件字节的 base64 编码(不带 data-URL 前缀)。
    pub base64: String,
    /// 原始字节数。
    pub size: u64,
}

/// 20 MiB:provider vision 接口的实际上限远小于此;再大的文件几乎必然
/// 是用户拖错东西,直接拒绝而不是让 IPC payload 失控。
const IMAGE_READ_MAX: u64 = 20 * 1_048_576;

/// 按扩展名推断图片 MIME;非白名单扩展返回 `None`。
fn image_mime_by_extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => return None,
    })
}

/// 读取图片文件并返回 base64 —— Composer 拖拽/选择器附件管线用。
///
/// 与 `reflect_read_file` 的两个刻意差异:
/// - **不做工作区沙盒**:拖拽来源可以是 Desktop / Downloads 等任意位置,
///   沙盒会让主用例(拖张截图进来)直接失败;
/// - **扩展名白名单 + 20 MiB 上限**:core 侧 `UserInputItem::LocalImage`
///   尚未接通(submission_loop 静默跳过),前端必须拿到字节以 inline
///   `Image` item 提交 —— 这里挡住非图片与大文件,防 IPC payload 失控。
#[tauri::command]
pub async fn reflect_read_image_base64(path: String) -> CommandResult<ImageBase64Result> {
    let p = PathBuf::from(&path);
    let Some(mime) = image_mime_by_extension(&p) else {
        return Err(CommandError {
            msg: format!("not a supported image file: {path} (expected png/jpg/jpeg/gif/webp/bmp)"),
        });
    };
    let meta = std::fs::metadata(&p).map_err(CommandError::from)?;
    if !meta.is_file() {
        return Err(CommandError {
            msg: format!("not a file: {}", p.display()),
        });
    }
    if meta.len() > IMAGE_READ_MAX {
        return Err(CommandError {
            msg: format!(
                "image too large: {} bytes (max {IMAGE_READ_MAX})",
                meta.len()
            ),
        });
    }
    let bytes = std::fs::read(&p).map_err(CommandError::from)?;
    Ok(ImageBase64Result {
        path: p.display().to_string(),
        mime_type: mime.to_string(),
        base64: crate::commands::media::base64_encode(&bytes),
        size: meta.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_mime_by_extension_maps_whitelist() {
        assert_eq!(
            image_mime_by_extension(Path::new("/tmp/a.PNG")),
            Some("image/png")
        );
        assert_eq!(
            image_mime_by_extension(Path::new("/tmp/b.jpeg")),
            Some("image/jpeg")
        );
        assert_eq!(
            image_mime_by_extension(Path::new("/tmp/c.webp")),
            Some("image/webp")
        );
        assert_eq!(image_mime_by_extension(Path::new("/tmp/d.txt")), None);
        assert_eq!(image_mime_by_extension(Path::new("/tmp/noext")), None);
    }

    #[test]
    fn read_image_base64_round_trip() {
        let dir = std::env::temp_dir().join("reflect-files-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("reflect_read_image_base64.png");
        std::fs::write(&path, [0x89u8, b'P', b'N', b'G']).unwrap();
        let res = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(reflect_read_image_base64(path.display().to_string()))
            .unwrap();
        assert_eq!(res.mime_type, "image/png");
        assert_eq!(res.size, 4);
        // 4 字节 → base64 为 8 字符(无 padding,ceil(4/3)*4)。
        assert_eq!(res.base64.len(), 8);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn read_image_base64_rejects_non_image() {
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(reflect_read_image_base64(
                "/tmp/reflect-not-an-image.txt".into(),
            ))
            .unwrap_err();
        assert!(err.msg.contains("not a supported image file"));
    }
}
