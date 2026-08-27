//! Session 列表 / 重命名 / 删除 / 归档 / 回放 / JSON 导出。
//!
//! 应用层适配（不改 submodule 语义）:
//! - `reader::replay` 只查「今天/昨天/前天」三日窗口,历史会话会拿到空
//!   结果 → `replay_session` 在全树扫描该 session 的所有文件(跨日期目录
//!   + 轮转副本)后按时间序回放兜底;
//! - `index::list_sessions` 派生的 `title` 对 block 数组 `content`
//!   (user 消息的新格式)会退化成整段 JSON 字符串,且从不合并
//!   `_names/<id>.name` 自定义名 → `refine_session_titles` 统一精化;
//! - **session 归属以文件内 `session_meta.session_id` 为准**(与索引一致)。
//!   真实数据存在文件名 ≠ 内嵌 meta id 的文件(如 resume 沿用旧文件名
//!   写入新线程),仅按文件名定位会 miss → 文件定位统一先读首行 meta,
//!   无 meta 时才回退文件名 stem。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use reflect_protocol::{derive_title, MessageRole, RolloutRecord, SessionInfo, ThreadId};
use reflect_rollout::{index as rollout_index, reader as rollout_reader, types::MAX_ROTATED_FILES};

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;
use tauri::State;

/// 复用 `reflect_rollout::path::default_base()` = `$HOME/.reflect/sessions`。
///
/// v1.x:原 `dirs::home_dir().join(".reflect/sessions")` 与 submodule 的
/// `default_base()` 路径语义等价但拼装独立,改为直接走 core 助手避免
/// record 端(`reflect_rollout::JsonlRolloutWriter`)与 read 端(sessions_base)
/// 因环境差异而分叉。两边现在都走同一函数。
fn sessions_base() -> Option<PathBuf> {
    Some(reflect_rollout::path::default_base())
}

/// 预分配一个新 session id(前端路由用)。
///
/// 返回前端 navigate 到的 `/chat/<id>`。session 的 workspace 归属由
/// 首条 `Submission.workspace` 触发后端 `submission_loop` 写入
/// `SessionMeta`(见 PROTOCOL_BRIDGE §2 `workspace` 字段),不在这里
/// 落盘 —— 避免产生只有 SessionMeta 头、无消息的空 JSONL。
///
/// v1.x:`ChatView` 在挂载时调 `reflect_bind_session(id)`,把后端
/// AgentThread 真正绑到该 id。`reflect_create_session` 本身仍是纯 ID
/// 分配,绑定延迟到 ChatView 挂载。
#[tauri::command]
pub async fn reflect_create_session() -> CommandResult<String> {
    Ok(ThreadId::new().to_string())
}

/// v1.x:把后端 AgentThread 绑到指定 session id(回放历史 → preload → 重建)。
///
/// ChatView 激活每个会话都调一次:
/// - 路由新会话(id 刚由 `reflect_create_session` 分配)→ replay 空 → 空历史绑定,
///   首条消息开始落盘。
/// - 路由到历史会话 → replay 整段 JSONL → `records_to_preload` → 注入 preload,
///   LLM 恢复上下文;后续消息续写到同一文件。
///
/// 未知 id(磁盘上无对应 JSONL,例如刚创建未发消息的新会话)→ **不报错**,
/// 走空历史分支,以兼容 New Chat 的 create → navigate → bind 序列。
#[tauri::command]
pub async fn reflect_bind_session(
    agent: State<'_, MinimalAgent>,
    id: ThreadId,
) -> CommandResult<()> {
    if !agent.model_registry_ready() {
        return Err(CommandError {
            msg: "agent not installed yet".into(),
        });
    }
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    // replay_session 返回空 Vec 时不报错 —— 这是合法状态(刚创建的 id)。
    let records = replay_session(&base, &id)
        .await
        .map_err(|e| CommandError {
            msg: format!("replay_session({id}) failed: {e:#}"),
        })?;
    let preload = reflect_core::resume::records_to_preload(&records);
    crate::state::rebind::rebind_session(&agent, id, preload)
        .map_err(|e| CommandError { msg: format!("rebind_session failed: {e:#}") })
}

/// 列出 session,支持可选 workspace 过滤 + 分页。
///
/// - `workspace = None` → 全量(含未归属旧 session);
/// - `workspace = Some(ws)` → 只返回 `SessionMeta.workspace == ws`
///   的 session;未归属(`workspace = None`)的旧 session 不出现,
///   避免把历史会话误归到任何项目视图。
/// - `limit` + `offset` 提供游标式分页(按时间倒序)。
///   `limit = 0` 或省略 → 无上限(返回完整列表)。
#[tauri::command]
pub async fn reflect_list_sessions(
    workspace: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> CommandResult<Vec<SessionInfo>> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let mut all = match workspace.as_deref() {
        Some(ws) if !ws.is_empty() => rollout_index::list_sessions_in_workspace(&base, ws),
        _ => rollout_index::list_sessions(&base),
    }
    .map_err(CommandError::from)?;
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
    // 分页后再精化,只处理返回页(标题精化本身是一次全局 walk,
    // 与分页无关,但避免改写整表)。
    refine_session_titles(&base, &mut all);
    Ok(all)
}

#[tauri::command]
pub async fn reflect_rename_session(id: ThreadId, new_name: String) -> CommandResult<()> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    rollout_index::rename_session(&base, id, &new_name).map_err(CommandError::from)
}

/// 删除 session:移除该 session 的全部 rollout 文件(含文件名错位副本、
/// 轮转副本)、自定义名,以及旧布局的 `<base>/<id>/` 目录。
#[tauri::command]
pub async fn reflect_delete_session(id: ThreadId) -> CommandResult<()> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let removed = delete_session_paths(&base, &id)?;
    if removed > 0 {
        tracing::info!("[reflect-gui] deleted session {} ({} paths)", id, removed);
    } else {
        tracing::debug!("[reflect-gui] delete_session: {} not found (no-op)", id);
    }
    Ok(())
}

// ── 归档 ────────────────────────────────────────────────────────────────
//
// rollout 索引/回放对 `<base>` 做全树 `**/*.jsonl` 扫描,归档文件留在树内
// (如 `_archived/` 子目录)仍会被 `list_sessions` / replay 命中。因此归档
// 根目录放在 sessions 树之外:`~/.reflect/sessions-archive/`,以「整树
// 搬移 + 相对路径不变」为不变量 —— unarchive 沿同一相对路径搬回。

fn archive_base() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".reflect/sessions-archive"))
}

/// 归档 session:把该 session 的全部文件(含轮转/错位副本)与自定义名
/// 从 `~/.reflect/sessions` 搬到 `~/.reflect/sessions-archive`。
///
/// sessions 侧路径走 [`sessions_base`](与 record/read 同源);归档树独立
/// 于 rollout base(`archive_base`),unarchive 沿同一对目录往返。
#[tauri::command]
pub async fn reflect_archive_session(id: ThreadId) -> CommandResult<()> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let archive = archive_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let moved = move_session_tree(&base, &archive, &id)?;
    if moved > 0 {
        tracing::info!("[reflect-gui] archived session {} ({} paths)", id, moved);
    } else {
        tracing::debug!("[reflect-gui] archive_session: {} not found (no-op)", id);
    }
    Ok(())
}

/// 恢复归档 session:搬回 `~/.reflect/sessions` 原相对路径。
#[tauri::command]
pub async fn reflect_unarchive_session(id: ThreadId) -> CommandResult<()> {
    let archive = archive_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let moved = move_session_tree(&archive, &base, &id)?;
    if moved > 0 {
        tracing::info!("[reflect-gui] unarchived session {} ({} paths)", id, moved);
    } else {
        tracing::debug!("[reflect-gui] unarchive_session: {} not found (no-op)", id);
    }
    Ok(())
}

/// 列出已归档 session(按 started_at 倒序,标题精化规则与活跃列表一致)。
/// `workspace` 过滤语义与 [`reflect_list_sessions`] 一致。
#[tauri::command]
pub async fn reflect_list_archived_sessions(
    workspace: Option<String>,
) -> CommandResult<Vec<SessionInfo>> {
    let base = archive_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    let mut all = match workspace.as_deref() {
        Some(ws) if !ws.is_empty() => rollout_index::list_sessions_in_workspace(&base, ws),
        _ => rollout_index::list_sessions(&base),
    }
    .map_err(CommandError::from)?;
    refine_session_titles(&base, &mut all);
    Ok(all)
}

/// `reflect_archive_session` / `reflect_unarchive_session` 的共用核心:
/// 把 `from` 树下该 session 的全部文件(`session_files` 定位,含 meta 错位
/// 副本)+ `_names/<id>.name` 搬到 `to` 树的**同相对路径**。返回搬移数。
fn move_session_tree(from: &Path, to: &Path, id: &ThreadId) -> CommandResult<usize> {
    let mut moved = 0usize;
    for path in session_files(from, id) {
        let rel = path
            .strip_prefix(from)
            .map_err(|e| CommandError { msg: e.to_string() })?;
        move_file(&path, &to.join(rel))?;
        moved += 1;
    }
    let name = from.join("_names").join(format!("{id}.name"));
    if name.exists() {
        move_file(&name, &to.join("_names").join(format!("{id}.name")))?;
        moved += 1;
    }
    Ok(moved)
}

/// rename 优先(同 `$HOME` 下同文件系统);跨设备时退化为 copy + remove。
fn move_file(src: &Path, dest: &Path) -> CommandResult<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::fs::copy(src, dest)?;
            std::fs::remove_file(src)?;
            Ok(())
        }
    }
}


/// `reflect_delete_session` 的可测试核心:删除 `base` 下该 session 的
/// 全部路径,返回删除数。
fn delete_session_paths(base: &Path, id: &ThreadId) -> CommandResult<usize> {
    let mut removed = 0usize;
    for path in session_files(base, id) {
        std::fs::remove_file(&path).map_err(CommandError::from)?;
        removed += 1;
    }
    let name = base.join("_names").join(format!("{id}.name"));
    if name.exists() {
        std::fs::remove_file(&name).map_err(CommandError::from)?;
        removed += 1;
    }
    // 旧布局:<base>/<id>/ 目录。
    let dir = base.join(id.to_string());
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(CommandError::from)?;
        removed += 1;
    }
    Ok(removed)
}

#[tauri::command]
pub async fn reflect_replay_session(id: ThreadId) -> CommandResult<Vec<RolloutRecord>> {
    let base = sessions_base().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    replay_session(&base, &id)
        .await
        .map_err(CommandError::from)
}

/// 回放一个 session 的全部记录:
///
/// 1. 快路径:`rollout_reader::replay`(三日窗口,覆盖活跃会话);
/// 2. 兜底:快路径为空时全树扫描该 session 的文件(历史日期目录 +
///    轮转副本),按时间序拼接回放。JSONL 是 append-only 且轮转是
///    rename(不复制),多文件拼接不会重复。
async fn replay_session(base: &Path, id: &ThreadId) -> anyhow::Result<Vec<RolloutRecord>> {
    let mut records = rollout_reader::replay(base, *id).await?;
    if records.is_empty() {
        for path in session_files(base, id) {
            records.extend(rollout_reader::replay_path(&path).await?);
        }
    }
    Ok(records)
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
    let records = replay_session(&home.join(".reflect/sessions"), &id)
        .await
        .map_err(CommandError::from)?;
    let json = serde_json::to_string_pretty(&records).map_err(|e| CommandError {
        msg: format!("json: {e}"),
    })?;
    std::fs::write(&dest, json).map_err(CommandError::from)?;
    Ok(dest.to_string_lossy().into_owned())
}

// ── 历史会话文件定位(replay 兜底) ──────────────────────────────────────

/// 收集 `id` 对应 session 的全部 JSONL 文件(活跃文件 + 轮转副本,
/// 可能跨多个日期目录),按时间升序返回:
///
/// - 目录层级(日期桶)按全路径字典序 = 时间序(`YYYY/MM/DD`);
/// - 同目录内 `<id>.1.jsonl` < `.2` < `.3` < 活跃 `<id>.jsonl`
///   (轮转把旧文件推向更大 N,见 `writer::rotate`)。
fn session_files(base: &Path, id: &ThreadId) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    collect_session_files(base, id, &mut out);
    out.sort();
    out
}

fn collect_session_files(dir: &Path, id: &ThreadId, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_session_files(&path, id, out);
        } else if let Some(file) = matching_session_file(&path, id) {
            out.push(file);
        }
    }
}

/// 文件名 stem 恰为 `<id>` 或 `<id>.<n>`(1..=MAX_ROTATED_FILES)时命中。
/// ThreadId 是 UUID(无 `.` 字符),prefix 匹配无歧义。
///
/// 匹配优先级与索引(`index::list_sessions`)的 session 识别规则一致:
/// 1. 首行 `session_meta.session_id == id`(真实数据存在文件名 ≠ meta id
///    的文件,如 resume 沿用旧文件名写入新线程;meta 是归属的权威来源);
/// 2. 无 meta 行的文件回退文件名 stem 匹配。
fn matching_session_file(path: &Path, id: &ThreadId) -> Option<PathBuf> {
    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return None;
    }
    let id_str = id.to_string();
    match first_line_session_id(path) {
        Some(meta) => (meta == id_str).then(|| path.to_path_buf()),
        None => filename_session_match(path, &id_str),
    }
}

/// 读 JSONL 首行 `session_meta` 的 session_id。meta 恰好发送一次且先于
/// 所有其它记录(reader/writer 协议),首行即够;非 meta 开头 → `None`。
fn first_line_session_id(path: &Path) -> Option<String> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).ok()?;
    let first = std::io::BufReader::new(file).lines().next()?.ok()?;
    match serde_json::from_str::<RolloutRecord>(&first).ok()? {
        RolloutRecord::SessionMeta { session_id, .. } => Some(session_id.to_string()),
        _ => None,
    }
}

/// 文件名 stem 恰为 `<id>` 或 `<id>.<n>`(1..=MAX_ROTATED_FILES)。
fn filename_session_match(path: &Path, id_str: &str) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    let stem = file_name.strip_suffix(".jsonl")?;
    if stem == id_str {
        return Some(path.to_path_buf());
    }
    let n = stem.strip_prefix(&format!("{id_str}."))?.parse::<usize>().ok()?;
    (1..=MAX_ROTATED_FILES).contains(&n).then(|| path.to_path_buf())
}

// ── 列表标题精化 ────────────────────────────────────────────────────────
//
// 两级优先级:
// 1. 用户自定义名(`<base>/_names/<id>.name`,由 `reflect_rename_session`
//    写入;TUI 的「自定义名优先于派生值」语义在此镜像);
// 2. 首条 User 消息重派生 —— `index::list_sessions` 的 `title` 对 block
//    数组 `content`(user 消息新格式)是整段 JSON 字符串,不可用作标题。

/// 精化 `sessions` 中每条的 `title`(原地)。custom name 永远胜出;
/// 无 custom name 时用 block-aware 首条 User 消息覆盖(结果幂等:
/// 纯字符串 content 的会话两边派生值相同)。
fn refine_session_titles(base: &Path, sessions: &mut [SessionInfo]) {
    if sessions.is_empty() {
        return;
    }
    let custom = custom_names(base);
    let derived = derived_titles(base);
    for s in sessions.iter_mut() {
        let id = s.session_id.to_string();
        if let Some(name) = custom.get(&id) {
            s.title = Some(name.clone());
        } else if let Some(title) = derived.get(&id) {
            s.title = Some(title.clone());
        }
    }
}

/// 读 `<base>/_names/*.name` → `session_id → 自定义名`。
fn custom_names(base: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(entries) = std::fs::read_dir(base.join("_names")) else {
        return out;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Some(id) = file_name.strip_suffix(".name") else {
            continue;
        };
        match std::fs::read_to_string(entry.path()) {
            Ok(text) if !text.trim().is_empty() => {
                out.insert(id.to_string(), text.trim().to_string());
            }
            _ => {}
        }
    }
    out
}

/// 全树单次 walk(条目按名排序 → 日期桶升序 = 时间序),为每个
/// session 记录其文件里首条 User 消息派生的标题。同一 session 命中
/// 首个文件即胜出 —— 排序遍历保证最旧文件(含会话真正首条消息)优先。
fn derived_titles(base: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    walk_for_titles(base, &mut out);
    out
}

fn walk_for_titles(dir: &Path, out: &mut HashMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk_for_titles(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl")
            && let Some((session_id, title)) = first_user_message_title_file(&path)
        {
            out.entry(session_id).or_insert(title);
        }
    }
}

/// 读单个 JSONL 文件,从首条 `Message { role: User }` 记录派生标题。
/// 返回 `(session_id, title)`;session_id 取自文件内 `session_meta`
/// (与索引的识别规则一致 —— 真实数据存在文件名 ≠ meta id 的文件),
/// 无 meta 行时回退文件名 stem(`<id>.jsonl` / `<id>.<n>.jsonl`)。
fn first_user_message_title_file(path: &Path) -> Option<(String, String)> {
    let body = std::fs::read_to_string(path).ok()?;
    let mut meta_id: Option<String> = None;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<RolloutRecord>(trimmed) else {
            continue;
        };
        match record {
            RolloutRecord::SessionMeta { session_id, .. } => {
                meta_id = Some(session_id.to_string());
            }
            RolloutRecord::Message {
                role: MessageRole::User,
                content,
                ..
            } => {
                let Some(text) = user_message_text(&content) else {
                    continue;
                };
                let Some(title) = derive_title(&text) else {
                    continue;
                };
                let session_id = meta_id.or_else(|| session_file_id(path))?;
                return Some((session_id, title));
            }
            _ => {}
        }
    }
    None
}

/// 文件名 stem → session id(无 meta 行时的回退):`<id>.jsonl` → `<id>`;
/// `<id>.<n>.jsonl`(轮转副本)→ `<id>`。归属优先取文件内 meta,见
/// [`first_user_message_title_file`]。
fn session_file_id(path: &Path) -> Option<String> {
    let stem = path.file_name()?.to_str()?.strip_suffix(".jsonl")?;
    if let Some((head, n)) = stem.rsplit_once('.') {
        if n.parse::<usize>().is_ok() {
            return Some(head.to_string());
        }
    }
    Some(stem.to_string())
}

/// 从 User 消息 `content` 提取展示文本:
/// - 字符串 → 原样;
/// - block 数组 → 所有 `text` 块以空格连接(跳过 image 等非文本块)。
fn user_message_text(content: &serde_json::Value) -> Option<String> {
    match content {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        serde_json::Value::Array(blocks) => {
            let texts: Vec<&str> = blocks
                .iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect();
            let joined = texts.join(" ");
            if joined.trim().is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // 分页逻辑与 reflect_list_sessions 的 limit/offset 切片保持一致。
    // 这里不启动 Tauri runtime,而是在合成的 Vec<String> 上复现切片语义,
    // 用以捕获算法层的回归。

    fn paginate<T: Clone>(mut all: Vec<T>, limit: Option<usize>, offset: Option<usize>) -> Vec<T> {
        let off = offset.unwrap_or(0);
        if off >= all.len() {
            return Vec::new();
        }
        let mut rest = if off > 0 {
            all.split_off(off)
        } else {
            all
        };
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

    // ── 历史会话文件定位 ────────────────────────────────────────────────

    fn write_file(dir: &Path, name: &str, body: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    const SESSION_META_LINE: &str =
        r#"{"type":"session_meta","session_id":"11111111-2222-3333-4444-555555555555","model":"m","started_at":"2026-07-01T00:00:00Z"}"#;

    /// 另一个 session 的 meta(干扰项夹具必须用真实不同的归属 id,
    /// 文件定位按内容 meta 优先匹配)。
    const OTHER_SESSION_META_LINE: &str =
        r#"{"type":"session_meta","session_id":"99999999-9999-9999-9999-999999999999","model":"m","started_at":"2026-07-01T00:00:00Z"}"#;

    #[test]
    fn session_files_orders_rotated_copies_chronologically() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("07").join("01");
        // 乱序写入,验证排序:同目录 .1 < .2 < .3 < 活跃;跨日期目录升序。
        write_file(&day, "11111111-2222-3333-4444-555555555555.jsonl", SESSION_META_LINE);
        write_file(
            &day,
            "11111111-2222-3333-4444-555555555555.1.jsonl",
            SESSION_META_LINE,
        );
        write_file(
            &day,
            "11111111-2222-3333-4444-555555555555.3.jsonl",
            SESSION_META_LINE,
        );
        let older = base.join("2026").join("06").join("30");
        write_file(
            &older,
            "11111111-2222-3333-4444-555555555555.jsonl",
            SESSION_META_LINE,
        );
        // 干扰项:其它 session(meta 归属不同)/ 非 jsonl。
        write_file(&day, "99999999-9999-9999-9999-999999999999.jsonl", OTHER_SESSION_META_LINE);
        write_file(&day, "11111111-2222-3333-4444-555555555555.json", "x");

        let id = ThreadId::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let files = session_files(base, &id);
        let names: Vec<String> = files
            .iter()
            .map(|p| {
                format!(
                    "{}/{}",
                    p.parent().unwrap().file_name().unwrap().to_str().unwrap(),
                    p.file_name().unwrap().to_str().unwrap()
                )
            })
            .collect();
        // 06/30 在前(更旧),07/01 内 .1 < .3 < 活跃;`999…` 不命中。
        assert_eq!(names.len(), 4);
        assert!(names[0].starts_with("30/"));
        assert!(names[1].ends_with(".1.jsonl"));
        assert!(names[2].ends_with(".3.jsonl"));
        assert!(names[3].ends_with("11111111-2222-3333-4444-555555555555.jsonl"));
    }

    #[test]
    fn session_files_matches_by_embedded_meta_id_when_filename_differs() {
        // 真实数据场景:文件名是 A,内嵌 meta id 是 B(resume 沿用旧文件名
        // 写入新线程)→ 按 B 定位必须命中该文件。
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("08").join("20");
        write_file(
            &day,
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl",
            SESSION_META_LINE,
        );

        let id = ThreadId::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let files = session_files(base, &id);
        assert_eq!(files.len(), 1);
        assert!(
            files[0]
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("aaaaaaaa")
        );

        // 反向:按文件名 id(aaaaaaaa)查,meta 归属是 11111111 → 不命中。
        let other = ThreadId::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        assert!(session_files(base, &other).is_empty());
    }

    #[test]
    fn session_files_falls_back_to_filename_without_meta() {
        // 无 meta 行的文件回退文件名 stem 匹配(活跃 + 轮转副本)。
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("07").join("01");
        write_file(&day, "11111111-2222-3333-4444-555555555555.jsonl", "garbage\n");
        write_file(
            &day,
            "11111111-2222-3333-4444-555555555555.1.jsonl",
            "garbage\n",
        );
        write_file(&day, "11111111-2222-3333-4444-555555555555.9.jsonl", "garbage\n");

        let id = ThreadId::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let files = session_files(base, &id);
        // 活跃 + .1 命中;越界 .9 不命中。
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn delete_session_removes_files_custom_name_and_legacy_dir() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        let id = ThreadId::new();
        let day = base.join("2026").join("08").join("20");
        // 文件名错位副本(meta id 即目标)+ 正常命名副本。
        write_file(
            &day,
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl",
            &format!(r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-08-20T00:00:00Z"}}"#),
        );
        write_file(
            &day,
            &format!("{id}.jsonl"),
            &format!(r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-08-20T00:00:00Z"}}"#),
        );
        std::fs::create_dir_all(base.join("_names")).unwrap();
        std::fs::write(base.join("_names").join(format!("{id}.name")), "n").unwrap();
        // 旧布局目录。
        std::fs::create_dir_all(base.join(id.to_string())).unwrap();
        // 其它 session 的文件,不能被误删。
        write_file(
            &day,
            "99999999-9999-9999-9999-999999999999.jsonl",
            OTHER_SESSION_META_LINE,
        );

        let removed = delete_session_paths(base, &id).unwrap();
        assert_eq!(removed, 4);
        assert!(!day.join("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl").exists());
        assert!(!day.join(format!("{id}.jsonl")).exists());
        assert!(!base.join("_names").join(format!("{id}.name")).exists());
        assert!(!base.join(id.to_string()).exists());
        assert!(day.join("99999999-9999-9999-9999-999999999999.jsonl").exists());
    }

    // ── 归档 / 恢复 ──────────────────────────────────────────────────────

    #[test]
    fn archive_moves_files_and_custom_name_to_archive_tree() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("sessions");
        let archive = dir.path().join("sessions-archive");
        let id = ThreadId::new();
        let day = base.join("2026").join("08").join("20");
        write_file(
            &day,
            &format!("{id}.jsonl"),
            &format!(r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-08-20T00:00:00Z"}}"#),
        );
        write_file(
            &day,
            &format!("{id}.1.jsonl"),
            &format!(r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-08-20T00:00:00Z"}}"#),
        );
        // 文件名错位副本也随行。
        write_file(
            &day,
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl",
            &format!(r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-08-20T00:00:00Z"}}"#),
        );
        std::fs::create_dir_all(base.join("_names")).unwrap();
        std::fs::write(base.join("_names").join(format!("{id}.name")), "归档名").unwrap();
        // 干扰项:其它 session 留在原地。
        write_file(
            &day,
            "99999999-9999-9999-9999-999999999999.jsonl",
            OTHER_SESSION_META_LINE,
        );

        let moved = move_session_tree(&base, &archive, &id).unwrap();
        assert_eq!(moved, 4);
        assert!(!day.join(format!("{id}.jsonl")).exists());
        assert!(!base.join("_names").join(format!("{id}.name")).exists());
        assert!(archive.join("2026/08/20").join(format!("{id}.jsonl")).exists());
        assert!(archive.join("2026/08/20").join(format!("{id}.1.jsonl")).exists());
        assert!(
            archive
                .join("2026/08/20")
                .join("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl")
                .exists()
        );
        assert_eq!(
            std::fs::read_to_string(archive.join("_names").join(format!("{id}.name"))).unwrap(),
            "归档名"
        );
        // 其它 session 不受影响。
        assert!(day.join("99999999-9999-9999-9999-999999999999.jsonl").exists());

        // 归档树中的 session 可被索引列出(标题精化含自定义名)。
        let mut listed = rollout_index::list_sessions(&archive).unwrap();
        assert_eq!(listed.len(), 1);
        refine_session_titles(&archive, &mut listed);
        assert_eq!(listed[0].title.as_deref(), Some("归档名"));
    }

    #[test]
    fn archive_then_unarchive_round_trips_to_original_relative_paths() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("sessions");
        let archive = dir.path().join("sessions-archive");
        let id = ThreadId::new();
        let day = base.join("2026").join("07").join("01");
        write_file(
            &day,
            &format!("{id}.jsonl"),
            &format!(r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-07-01T00:00:00Z"}}"#),
        );

        assert_eq!(move_session_tree(&base, &archive, &id).unwrap(), 1);
        assert_eq!(move_session_tree(&archive, &base, &id).unwrap(), 1);
        // 搬回后回到原相对路径,且归档树为空。
        assert!(base.join("2026/07/01").join(format!("{id}.jsonl")).exists());
        assert!(!archive.join("2026/07/01").join(format!("{id}.jsonl")).exists());
        assert_eq!(rollout_index::list_sessions(&archive).unwrap().len(), 0);
    }

    #[test]
    fn archive_missing_session_is_noop() {
        let dir = tempdir().unwrap();
        let moved = move_session_tree(
            &dir.path().join("sessions"),
            &dir.path().join("sessions-archive"),
            &ThreadId::new(),
        )
        .unwrap();
        assert_eq!(moved, 0);
    }

    // ── 标题精化 ────────────────────────────────────────────────────────

    const BLOCK_ARRAY_USER_MSG: &str =
        r#"{"type":"message","turn_id":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","role":"user","content":[{"text":"修复滚动条问题","type":"text"}]}"#;

    #[test]
    fn user_message_text_handles_string_and_block_array() {
        let string = serde_json::json!("直接文本");
        assert_eq!(user_message_text(&string).as_deref(), Some("直接文本"));

        let blocks = serde_json::json!([
            {"type": "text", "text": "修复"},
            {"type": "image", "data": "xx", "mime_type": "image/png"},
            {"type": "text", "text": "滚动条"}
        ]);
        assert_eq!(
            user_message_text(&blocks).as_deref(),
            Some("修复 滚动条")
        );

        let empty = serde_json::json!("   ");
        assert_eq!(user_message_text(&empty), None);
        let number = serde_json::json!(42);
        assert_eq!(user_message_text(&number), None);
    }

    #[test]
    fn session_file_id_strips_rotation_suffix() {
        let active = PathBuf::from("/tmp/s/2026/07/01/abc-1.jsonl");
        assert_eq!(
            session_file_id(&active).as_deref(),
            Some("abc-1")
        );
        let rotated = PathBuf::from("/tmp/s/2026/07/01/abc-1.2.jsonl");
        assert_eq!(session_file_id(&rotated).as_deref(), Some("abc-1"));
    }

    /// 以给定 id 生成 session_meta 行(夹具的 meta id 必须与 SessionInfo
    /// 的 session_id 一致 —— session 归属以 meta 为准)。
    fn meta_line(id: &ThreadId) -> String {
        format!(
            r#"{{"type":"session_meta","session_id":"{id}","model":"m","started_at":"2026-07-01T00:00:00Z"}}"#
        )
    }

    fn session_info(id: ThreadId, title: Option<String>) -> SessionInfo {
        SessionInfo {
            session_id: id,
            model: "m".into(),
            started_at: chrono::Utc::now(),
            message_count: 2,
            title,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_usd: None,
            workspace: None,
        }
    }

    #[test]
    fn refine_prefers_custom_name_over_derived() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("07").join("01");
        let id = ThreadId::new();
        write_file(
            &day,
            &format!("{id}.jsonl"),
            &format!("{}\n{BLOCK_ARRAY_USER_MSG}\n", meta_line(&id)),
        );
        // 自定义名。
        std::fs::create_dir_all(base.join("_names")).unwrap();
        std::fs::write(base.join("_names").join(format!("{id}.name")), "我的会话").unwrap();

        let mut info = vec![session_info(
            id,
            Some("[{\"text\":\"修复滚动条问题\"}]".into()),
        )];
        refine_session_titles(base, &mut info);
        assert_eq!(info[0].title.as_deref(), Some("我的会话"));
    }

    #[test]
    fn refine_derives_block_aware_title_when_no_custom_name() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("07").join("01");
        let id = ThreadId::new();
        write_file(
            &day,
            &format!("{id}.jsonl"),
            &format!("{}\n{BLOCK_ARRAY_USER_MSG}\n", meta_line(&id)),
        );

        let mut info = vec![session_info(id, None)];
        refine_session_titles(base, &mut info);
        assert_eq!(info[0].title.as_deref(), Some("修复滚动条问题"));
    }

    #[test]
    fn refine_derives_title_keyed_by_meta_id_when_filename_differs() {
        // 真实数据场景:文件名 A ≠ 内嵌 meta id B。索引按 B 返回
        // SessionInfo → 派生标题必须以 B 为 key 才能命中。
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("08").join("20");
        let id = ThreadId::new();
        write_file(
            &day,
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl",
            &format!("{}\n{BLOCK_ARRAY_USER_MSG}\n", meta_line(&id)),
        );

        let mut info = vec![session_info(id, None)];
        refine_session_titles(base, &mut info);
        assert_eq!(info[0].title.as_deref(), Some("修复滚动条问题"));
    }

    #[test]
    fn refine_keeps_title_for_session_without_user_message() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        let day = base.join("2026").join("07").join("01");
        let id = ThreadId::new();
        // 只有 session_meta,没有 user 消息 → 保持原 title 不动。
        write_file(&day, &format!("{id}.jsonl"), &meta_line(&id));

        let mut info = vec![session_info(id, Some("已有标题".into()))];
        refine_session_titles(base, &mut info);
        assert_eq!(info[0].title.as_deref(), Some("已有标题"));
    }
}
