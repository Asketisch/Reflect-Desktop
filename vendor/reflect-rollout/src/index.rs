//! `list_sessions` walks the date directory tree under a base directory and
//! returns one [`SessionInfo`] per JSONL file found.
//!
//! Each file's first line is expected to be a `SessionMeta` record; if not
//! (e.g. the writer crashed before flushing), the file is logged and skipped.
//!
//! v0.2.4 新增 [`list_sessions_with_discussion`]:只列出 JSONL 文件中包含
//! 指定 `discussion_id` 的 `RolloutRecord::DiscussionTranscript` 记录的 session。

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use reflect_protocol::{MessageRole, RolloutRecord, SessionInfo, ThreadId, TurnId};

/// Scan `<base>/**/*.jsonl` and return one [`SessionInfo`] per **session**
/// (deduplicated by `session_id`). Cheap: only reads the first line of each
/// file.
///
/// ## Why dedup
///
/// Since the writer re-emits `SessionMeta` on rotation (see
/// `writer.rs` module doc), each rotated copy `<id>.N.jsonl` also starts with a
/// `SessionMeta`. Without dedup the same session would appear once per file.
/// We keep **one entry per `session_id`**, preferring the active file
/// (`<id>.jsonl` with no `.N` suffix — its body is the most recent) and using
/// its `message_count`. Ties (no active file, only rotated copies) keep the
/// highest-index rotated copy.
pub fn list_sessions(base: &Path) -> std::io::Result<Vec<SessionInfo>> {
    let mut raw: Vec<(SessionInfo, std::path::PathBuf)> = Vec::new();
    if !base.exists() {
        return Ok(Vec::new());
    }
    walk(base, &mut raw)?;
    // Group by session_id, preferring the active (non-rotated) file.
    let mut by_id: HashMap<ThreadId, (SessionInfo, usize)> = HashMap::new();
    for (info, path) in raw {
        // Priority: active file (suffix_rank 0) > .N copy (rank N). Higher rank
        // wins for rotated copies so we fall back to the most recent rotation.
        let rank = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|s: &str| {
                let stem = s.strip_suffix(".jsonl")?;
                stem.rsplit_once('.')
                    .and_then(|(_, n): (&str, &str)| n.parse::<usize>().ok())
            })
            .unwrap_or(0);
        let active_rank = usize::MAX;
        let effective_rank = if rank == 0 { active_rank } else { rank };
        by_id
            .entry(info.session_id)
            .and_modify(|(prev, prev_rank)| {
                if effective_rank > *prev_rank {
                    *prev = info.clone();
                    *prev_rank = effective_rank;
                }
            })
            .or_insert((info, effective_rank));
    }
    let mut out: Vec<SessionInfo> = by_id.into_values().map(|(info, _)| info).collect();
    // Newest first by started_at.
    out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(out)
}

/// v0.4: 把 `-c` / `-r N` 旗标解析成具体 `ThreadId`。
///
/// - `continue_last == true`:返回 `list_sessions` 最新一条;若列表为空 → Err。
/// - `resume_by = Some(n)`:返回第 `n` 条(1-indexed);越界 → Err 含具体计数。
/// - 两者都为 `None` / 互斥 → Err(交给 caller 决定是否要报错)。
///
/// 列表**在调用时立即冻结**:返回的 `ThreadId` 在调用瞬间确定,不随后续新
/// session 写入而漂移。这样调用方可以安全地把 `ThreadId` 灌给 `bootstrap_resume`,
/// 不会因为启动过程中 session 列表变化而错位。
pub fn resolve_session_index(
    base: &Path,
    continue_last: bool,
    resume_by: Option<usize>,
) -> anyhow::Result<ThreadId> {
    // 互斥校验(虽然 clap 已经 enforce,这里再 defense-in-depth)。
    if continue_last && resume_by.is_some() {
        return Err(anyhow::anyhow!(
            "continue_last and resume_by are mutually exclusive"
        ));
    }
    if !continue_last && resume_by.is_none() {
        return Err(anyhow::anyhow!(
            "resolve_session_index called without -c / -r"
        ));
    }
    let sessions = list_sessions(base)?;
    if sessions.is_empty() {
        return Err(anyhow::anyhow!(
            "no sessions found under {}; nothing to resume",
            base.display()
        ));
    }
    if continue_last {
        return Ok(sessions[0].session_id);
    }
    // resume_by:1-indexed。
    let n = resume_by.expect("validated above");
    if n == 0 {
        return Err(anyhow::anyhow!(
            "resume index must be >= 1 (use -c to pick newest)"
        ));
    }
    if n > sessions.len() {
        return Err(anyhow::anyhow!(
            "no session at index {n}; only {} sessions available",
            sessions.len()
        ));
    }
    Ok(sessions[n - 1].session_id)
}

/// v0.2.4: 列出包含指定 `discussion_id` 的 `DiscussionTranscript` 记录的 session。
///
/// 实现:遍历 `<base>/**/*.jsonl` 文件,对每个文件扫描每一行 JSONL record,
/// 匹配 `RolloutRecord::DiscussionTranscript { discussion_id, .. } == given_id`
/// 即把该文件的 SessionInfo 加入结果。文件可能很大,但 DiscussionTranscript
/// 通常出现在尾部,所以折中策略:读整个文件(简单的实现,日后优化读尾 16 KiB
/// 即可)。
///
/// `discussion_id` 是 UUID 字符串(匹配时 `Uuid::parse_str` 后比较)。
pub fn list_sessions_with_discussion(
    base: &Path,
    discussion_id: &str,
) -> std::io::Result<Vec<SessionInfo>> {
    let mut out = Vec::new();
    if !base.exists() {
        return Ok(out);
    }
    let target = match uuid::Uuid::parse_str(discussion_id) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(error = %e, "invalid discussion_id uuid");
            return Ok(out);
        }
    };
    walk_with_discussion(base, &target, &mut out)?;
    out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<(SessionInfo, std::path::PathBuf)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl")
            && let Some(info) = parse_first_session_meta(&path)
        {
            out.push((info, path));
        }
    }
    Ok(())
}

fn walk_with_discussion(
    dir: &Path,
    target: &uuid::Uuid,
    out: &mut Vec<SessionInfo>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_with_discussion(&path, target, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl")
            && session_contains_discussion(&path, target)
            && let Some(info) = parse_first_session_meta(&path)
        {
            out.push(info);
        }
    }
    Ok(())
}

fn parse_first_session_meta(path: &Path) -> Option<SessionInfo> {
    let body = std::fs::read_to_string(path).ok()?;
    let first = body.lines().next()?;
    let non_blank_lines = body.lines().filter(|l| !l.trim().is_empty()).count();

    // Happy path: the first line is itself a SessionMeta. `message_count`
    // subtracts 1 for the meta line.
    if let Ok(record) = serde_json::from_str::<RolloutRecord>(first)
        && let RolloutRecord::SessionMeta {
            session_id,
            model,
            started_at,
        } = record
    {
        return Some(SessionInfo {
            session_id,
            model,
            started_at,
            message_count: non_blank_lines.saturating_sub(1),
        });
    }

    // Recovery path: this file's first line is NOT a SessionMeta. This happens
    // for older rollout files produced before the writer re-emitted SessionMeta
    // on rotation (or for the rotated `.N` copies). Scan the sibling rotated
    // files (`<stem>.1.jsonl` / `.2.jsonl` / `.3.jsonl`) — one of them carries
    // the original SessionMeta on its first line. Without this, long sessions
    // that rotated past the `SessionMeta`-bearing file vanish from `/session`.
    // Here there is no meta line in *this* file, so every non-blank line is a
    // message — no `-1` subtraction.
    if let Some(meta) = session_meta_from_rotated_sibling(path) {
        return Some(SessionInfo {
            session_id: meta.session_id,
            model: meta.model,
            started_at: meta.started_at,
            message_count: non_blank_lines,
        });
    }

    tracing::warn!("rollout: {} has no SessionMeta first line", path.display());
    None
}

/// Scan the sibling rotated files of `path` (`.1.jsonl` up to
/// `.MAX_ROTATED_FILES.jsonl`) and return the first one's SessionMeta. Each
/// rotated file's first line is checked; the first match wins. Returns `None`
/// if no sibling has a SessionMeta first line (or `path` has no stem).
///
/// The caller already has this file's body; we read at most one line per
/// sibling, so this stays cheap (≤ 3 line reads).
fn session_meta_from_rotated_sibling(path: &Path) -> Option<SessionMetaFields> {
    use crate::types::MAX_ROTATED_FILES;
    let stem = path.file_name()?.to_str()?;
    let stem = stem.strip_suffix(".jsonl")?;
    let parent = path.parent()?;
    for n in 1..=MAX_ROTATED_FILES {
        let sibling = parent.join(format!("{stem}.{n}.jsonl"));
        let Ok(mut f) = std::fs::File::open(&sibling) else {
            continue;
        };
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(&mut f);
        let mut first_line = String::new();
        if reader.read_line(&mut first_line).ok()? == 0 {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<RolloutRecord>(first_line.trim())
            && let RolloutRecord::SessionMeta {
                session_id,
                model,
                started_at,
            } = record
        {
            return Some(SessionMetaFields {
                session_id,
                model,
                started_at,
            });
        }
    }
    None
}

/// Flat fields extracted by [`session_meta_from_rotated_sibling`].
struct SessionMetaFields {
    session_id: ThreadId,
    model: String,
    started_at: chrono::DateTime<chrono::Utc>,
}

fn session_contains_discussion(path: &Path, target: &uuid::Uuid) -> bool {
    let body = match std::fs::read_to_string(path) {
        Ok(b) => b,
        Err(_) => return false,
    };
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(RolloutRecord::DiscussionTranscript { discussion_id, .. }) =
            serde_json::from_str::<RolloutRecord>(line)
            && &discussion_id == target
        {
            return true;
        }
    }
    false
}

// ── S5b:rename / fork 真实化 API ────────────────────────────────────────
//
// 存储约定:session 人可读 name 写到 `<base>/_names/<id>.name`(`_names`
// 是下划线前缀的隐藏子目录,`walk` 只 pick `.jsonl`,自然忽略)。Fork
// record 直接 append 到 parent session 的 JSONL 末尾(对齐 `replay`
// 的 append-only 假设)。

/// 递归扫描 `base` 找到 `id` 对应的 JSONL 文件路径。文件第一行必须是
/// `SessionMeta { session_id: id, .. }`,否则跳过(防御错位文件)。
///
/// `Some(path)` 找到 / `None` 找不到(包括 base 不存在)。**不**缓存
/// 结果 —— TUI 启动后 session 文件可能新增,每次调用都 walk 一次。
pub fn find_session_path(base: &Path, id: ThreadId) -> Option<std::path::PathBuf> {
    find_session_path_inner(base, id)
}

fn find_session_path_inner(dir: &Path, id: ThreadId) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_session_path_inner(&path, id) {
                return Some(found);
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl")
            && let Some(info) = parse_first_session_meta(&path)
            && info.session_id == id
        {
            return Some(path);
        }
    }
    None
}

/// 把 `id` 对应 session 改名为 `new_name`。存储到 `<base>/_names/<id>.name`
/// 的 plain text 文件(UTF-8)。重复调用即覆盖(支持"再改一次")。
///
/// `new_name.trim().is_empty()` → `Err(InvalidInput)`,避免写出空文件
/// 让后续 read 误判为"未命名"。
pub fn rename_session(base: &Path, id: ThreadId, new_name: &str) -> std::io::Result<()> {
    if new_name.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "rename: new name cannot be empty",
        ));
    }
    let names_dir = base.join("_names");
    std::fs::create_dir_all(&names_dir)?;
    let path = names_dir.join(format!("{id}.name"));
    std::fs::write(path, new_name.as_bytes())
}

/// 读取 `id` 的已命名 session 名。`None` = 未命名(无 .name 文件);
/// `Some(name)` = 已命名。trim 去除尾随换行(`std::fs::write` 不会加
/// `\n`,但 `read_to_string` 也不会;trim 是 defense-in-depth)。
pub fn read_session_name(base: &Path, id: ThreadId) -> std::io::Result<Option<String>> {
    let path = base.join("_names").join(format!("{id}.name"));
    match std::fs::read_to_string(&path) {
        Ok(s) => Ok(Some(s.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// 写 `RolloutRecord::Fork` 到 `parent_id` 对应 session 的 JSONL 末尾,
/// 返回新 session 的 ThreadId。
///
/// 行为细节:
/// - parent 文件**找不到** → `Err(NotFound)` 含 UUID,方便 TUI Pill 报错。
/// - `branch_name` 直接进 record 字段(`Vec<String>` 的 participants 留 v2.x)。
/// - 用 `OpenOptions::create(true).append(true)` —— JSONL 是 append-only,
///   重复 fork 不会覆盖。
/// - 失败时不回滚已写入 bytes(JSONL 单行写入要么全成要么 NotFound,无
///   中间态)。
pub fn write_fork_record(
    base: &Path,
    parent_id: ThreadId,
    branch_name: &str,
) -> std::io::Result<ThreadId> {
    use std::io::Write;

    let parent_path = find_session_path(base, parent_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "parent session {parent_id} not found under {}",
                base.display()
            ),
        )
    })?;
    let new_id = ThreadId::new();
    let record = RolloutRecord::Fork {
        parent_session_id: parent_id,
        branch_name: branch_name.to_string(),
    };
    let line = serde_json::to_string(&record).map_err(std::io::Error::other)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&parent_path)?;
    writeln!(f, "{line}")?;
    Ok(new_id)
}

/// v1.x fork-at-node:把父会话「截止 fork 点」的对话历史复制进一个**新建**
/// 的子会话 JSONL,并在父子两端标记 `Fork` 关系。返回子 `ThreadId`。
///
/// 与 [`write_fork_record`] 的区别:`write_fork_record` 只往父 JSONL append
/// 一条 marker、返回一个**没有文件**的新 id(v1.x 子会话为空);
/// `fork_with_history` 真正创建子 JSONL 并把历史写进去,使得
/// `reflect --resume <child_id>` 能在子会话里看到 fork 点之前的完整对话。
///
/// 设计要点:
/// - **子文件首行必须是 `SessionMeta { session_id: child_id, .. }`**,
///   否则 `find_session_path` 找不到子文件(`parse_first_session_meta`
///   按首行 session_id 匹配)。用 child 的新 id,不能用 parent 的。
/// - `up_to_messages` 由调用方负责重建(TUI 把内存 `history` 里的 user
///   文本和父 JSONL 里的 assistant 文本配对成有序列表)。本函数只负责
///   按顺序落盘。
/// - `latest_summary`(可选):父会话最新一条 `Compaction` 摘要,作为子会话
///   的首条 `Compaction` 继承下来,`bootstrap_resume` 会把它作为 synthetic
///   System message 注入。
/// - **父 JSONL 也 append 一条 `Fork` marker**(与 `write_fork_record`
///   一致),保持血缘追踪。
/// - 失败时不回滚已写入 bytes(JSONL 单行写入要么全成要么 NotFound)。
pub fn fork_with_history(
    base: &Path,
    parent_id: ThreadId,
    branch_name: &str,
    up_to_messages: &[(MessageRole, String)],
    latest_summary: Option<&str>,
    parent_model: &str,
) -> std::io::Result<ThreadId> {
    use std::io::Write;

    // 1) 校验父会话存在(否则子会话无父可 fork)。
    let parent_path = find_session_path(base, parent_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "parent session {parent_id} not found under {}",
                base.display()
            ),
        )
    })?;

    // 2) 分配子 id + 计算子文件路径(按当前 UTC 日期分桶)。
    let child_id = ThreadId::new();
    let child_path = crate::path::session_path_at(base, child_id, Utc::now());
    if let Some(parent_dir) = child_path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    // 序列化辅助:把 record 转成单行 JSON(不带末尾换行,由 writeln! 补)。
    let to_line = |r: &RolloutRecord| serde_json::to_string(r).map_err(std::io::Error::other);

    // 3) 写子文件:truncate 新建,首行 SessionMeta(child_id)。
    let mut child = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&child_path)?;

    writeln!(
        child,
        "{}",
        to_line(&RolloutRecord::SessionMeta {
            session_id: child_id,
            model: parent_model.to_string(),
            started_at: Utc::now(),
        })?
    )?;

    // 3a) 继承父会话最新 compaction 摘要(若有)。
    if let Some(summary) = latest_summary.filter(|s| !s.is_empty()) {
        writeln!(
            child,
            "{}",
            to_line(&RolloutRecord::Compaction {
                turn_id: TurnId::new(),
                strategy: "fork_inherited".to_string(),
                removed_count: 0,
                summary: summary.to_string(),
            })?
        )?;
    }

    // 3b) 按顺序写 user / assistant 文本消息。每条用新 turn_id(与原会话
    //     解耦;turn_id 只是 per-turn 标记,resume 时不按 turn_id 过滤)。
    for (role, content) in up_to_messages {
        writeln!(
            child,
            "{}",
            to_line(&RolloutRecord::Message {
                turn_id: TurnId::new(),
                role: *role,
                content: serde_json::Value::String(content.clone()),
            })?
        )?;
    }

    // 3c) 子文件末尾:Fork 标记(指向 parent)。
    writeln!(
        child,
        "{}",
        to_line(&RolloutRecord::Fork {
            parent_session_id: parent_id,
            branch_name: branch_name.to_string(),
        })?
    )?;
    child.flush()?;

    // 4) 父文件 append 一条 Fork marker(血缘追踪,镜像 write_fork_record)。
    let fork_line = to_line(&RolloutRecord::Fork {
        parent_session_id: parent_id,
        branch_name: branch_name.to_string(),
    })?;
    let mut parent_f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&parent_path)?;
    writeln!(parent_f, "{fork_line}")?;

    Ok(child_id)
}

/// v1.2 P0-3:把一条 `Checkpoint` 记录 append 到 `session_id` 的 JSONL。
///
/// 镜像 [`write_fork_record`] 的 append-only 写法:`find_session_path`
/// 定位文件 → `serde_json` 序列化 → append 一行。失败时返回 `io::Error`
/// (NotFound 表示 session 不存在)。
///
/// `sha` 是 `git_auto_commit` 后的 HEAD sha;`label` 可选。返回写入的
/// `Checkpoint` 记录(供 `CheckpointTool` 返回给 LLM)。
pub fn write_checkpoint_record(
    base: &Path,
    session_id: ThreadId,
    turn_id: TurnId,
    sha: &str,
    label: Option<&str>,
) -> std::io::Result<RolloutRecord> {
    use std::io::Write;
    let path = find_session_path(base, session_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session {session_id} not found under {}", base.display()),
        )
    })?;
    let record = RolloutRecord::Checkpoint {
        turn_id,
        sha: sha.to_string(),
        label: label.map(|s| s.to_string()),
        created_at: Utc::now(),
    };
    let line = serde_json::to_string(&record).map_err(std::io::Error::other)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "{line}")?;
    Ok(record)
}

/// v1.2 P0-3:把一条 `Rewind` 记录 append 到 `session_id` 的 JSONL。
///
/// `target_sha` 是回退到的 checkpoint sha;`from_sha` 是回退前的 HEAD
/// sha(便于审计 / 再次前进)。返回写入的 `Rewind` 记录。
pub fn write_rewind_record(
    base: &Path,
    session_id: ThreadId,
    turn_id: TurnId,
    target_sha: &str,
    from_sha: &str,
) -> std::io::Result<RolloutRecord> {
    use std::io::Write;
    let path = find_session_path(base, session_id).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("session {session_id} not found under {}", base.display()),
        )
    })?;
    let record = RolloutRecord::Rewind {
        turn_id,
        target_sha: target_sha.to_string(),
        from_sha: from_sha.to_string(),
        at: Utc::now(),
    };
    let line = serde_json::to_string(&record).map_err(std::io::Error::other)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "{line}")?;
    Ok(record)
}

/// v1.2 P0-3:读 `session_id` 的 JSONL,返回所有 `Checkpoint` 记录
/// (按 created_at 升序)。`CheckpointTool` 的 `list` action 用。
pub fn list_checkpoints(base: &Path, session_id: ThreadId) -> std::io::Result<Vec<RolloutRecord>> {
    let Some(path) = find_session_path(base, session_id) else {
        return Ok(Vec::new());
    };
    let body = std::fs::read_to_string(&path)?;
    let mut out = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<RolloutRecord>(line)
            && matches!(rec, RolloutRecord::Checkpoint { .. })
        {
            out.push(rec);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use reflect_protocol::{MessageRole, ThreadId, TurnId};
    use tempfile::tempdir;

    fn write_session(dir: &Path, day: &str, sid: ThreadId) -> std::io::Result<()> {
        let day_dir = dir.join(day);
        std::fs::create_dir_all(&day_dir)?;
        let path = day_dir.join(format!("{sid}.jsonl"));
        let mut s = String::new();
        s.push_str(
            &serde_json::to_string(&RolloutRecord::session_meta(sid, "openai/gpt-4o")).unwrap(),
        );
        s.push('\n');
        for i in 0..3 {
            s.push_str(
                &serde_json::to_string(&RolloutRecord::message(
                    TurnId::new(),
                    MessageRole::User,
                    serde_json::json!(i),
                ))
                .unwrap(),
            );
            s.push('\n');
        }
        std::fs::write(path, s)
    }

    #[test]
    fn lists_session_dirs() {
        let dir = tempdir().unwrap();
        let sid_a = ThreadId::new();
        let sid_b = ThreadId::new();
        write_session(dir.path(), "2026/06/17", sid_a).unwrap();
        write_session(dir.path(), "2026/06/18", sid_b).unwrap();

        let sessions = list_sessions(dir.path()).unwrap();
        assert_eq!(sessions.len(), 2);
        let ids: Vec<_> = sessions.iter().map(|s| s.session_id).collect();
        assert!(ids.contains(&sid_a));
        assert!(ids.contains(&sid_b));
        // Each session has 3 message lines + 1 session_meta => 4 total, 3 messages.
        for s in &sessions {
            assert_eq!(s.message_count, 3);
        }
    }

    #[test]
    fn missing_base_returns_empty() {
        let dir = tempdir().unwrap();
        let sessions = list_sessions(&dir.path().join("does/not/exist")).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn skips_files_without_session_meta() {
        let dir = tempdir().unwrap();
        let day_dir = dir.path().join("2026/06/18");
        std::fs::create_dir_all(&day_dir).unwrap();
        let bogus = day_dir.join("bogus.jsonl");
        std::fs::write(bogus, "not json\n").unwrap();
        let sessions = list_sessions(dir.path()).unwrap();
        assert!(sessions.is_empty(), "should skip bogus file");
    }

    /// Recovery path: when an active `<id>.jsonl`'s first line is NOT a
    /// `SessionMeta` (the pre-fix orphan state for rotated sessions), the
    /// index falls back to a sibling rotated file (`.1.jsonl`) to recover the
    /// meta so the session still shows up in `/session`.
    #[test]
    fn recovers_meta_from_rotated_sibling() {
        let dir = tempdir().unwrap();
        let day_dir = dir.path().join("2026/06/18");
        std::fs::create_dir_all(&day_dir).unwrap();
        let sid = ThreadId::new();
        let started = Utc.with_ymd_and_hms(2026, 6, 18, 12, 0, 0).unwrap();

        // `.1.jsonl` carries the original SessionMeta on its first line.
        let rotated = day_dir.join(format!("{sid}.1.jsonl"));
        let meta_line = serde_json::to_string(&RolloutRecord::SessionMeta {
            session_id: sid,
            model: "openai/gpt-4o".into(),
            started_at: started,
        })
        .unwrap();
        std::fs::write(
            &rotated,
            format!(
                "{meta_line}\n{}\n",
                serde_json::to_string(&RolloutRecord::message(
                    TurnId::new(),
                    MessageRole::User,
                    serde_json::json!("old"),
                ))
                .unwrap()
            ),
        )
        .unwrap();

        // Active file: starts with a Message (orphan state), 2 message lines.
        let active = day_dir.join(format!("{sid}.jsonl"));
        let msg = serde_json::to_string(&RolloutRecord::message(
            TurnId::new(),
            MessageRole::User,
            serde_json::json!("recent"),
        ))
        .unwrap();
        std::fs::write(&active, format!("{msg}\n{msg}\n")).unwrap();

        let sessions = list_sessions(dir.path()).unwrap();
        assert_eq!(sessions.len(), 1, "should recover the orphaned session");
        assert_eq!(sessions[0].session_id, sid);
        assert_eq!(sessions[0].model, "openai/gpt-4o");
        // message_count is from the *active* file (2 lines, both messages).
        assert_eq!(sessions[0].message_count, 2);
    }

    /// v0.2.4: `list_sessions_with_discussion` 只列出包含指定 `discussion_id`
    /// 的 `DiscussionTranscript` record 的 session。
    #[test]
    fn list_sessions_with_discussion_filters_by_id() {
        use uuid::Uuid;
        let dir = tempdir().unwrap();
        let day_dir = dir.path().join("2026/06/18");
        std::fs::create_dir_all(&day_dir).unwrap();

        let target_disc = Uuid::new_v4();
        let other_disc = Uuid::new_v4();

        // Session 1: 包含 target discussion_id 的 transcript
        let sid_a = ThreadId::new();
        let path_a = day_dir.join(format!("{sid_a}.jsonl"));
        let body_a = format!(
            "{}\n{}\n",
            serde_json::to_string(&RolloutRecord::SessionMeta {
                session_id: sid_a,
                model: "openai/gpt-4o".into(),
                started_at: Utc.with_ymd_and_hms(2026, 6, 18, 12, 0, 0).unwrap(),
            })
            .unwrap(),
            serde_json::to_string(&RolloutRecord::DiscussionTranscript {
                discussion_id: target_disc,
                mode: "sequential".into(),
                participants: vec!["a".into(), "b".into()],
                agent_id: None,
                transcript: serde_json::json!([{"id":0,"from":"a","kind":"utterance","content":"hi"}]),
            })
            .unwrap(),
        );
        std::fs::write(&path_a, body_a).unwrap();

        // Session 2: 不包含 target,只有 other discussion_id
        let sid_b = ThreadId::new();
        let path_b = day_dir.join(format!("{sid_b}.jsonl"));
        let body_b = format!(
            "{}\n{}\n",
            serde_json::to_string(&RolloutRecord::SessionMeta {
                session_id: sid_b,
                model: "anthropic/claude".into(),
                started_at: Utc.with_ymd_and_hms(2026, 6, 18, 13, 0, 0).unwrap(),
            })
            .unwrap(),
            serde_json::to_string(&RolloutRecord::DiscussionTranscript {
                discussion_id: other_disc,
                mode: "concurrent".into(),
                participants: vec!["x".into()],
                agent_id: None,
                transcript: serde_json::json!([]),
            })
            .unwrap(),
        );
        std::fs::write(&path_b, body_b).unwrap();

        // target_disc 应只命中 sid_a
        let filtered = list_sessions_with_discussion(dir.path(), &target_disc.to_string()).unwrap();
        assert_eq!(filtered.len(), 1, "should match only sid_a");
        assert_eq!(filtered[0].session_id, sid_a);

        // other_disc 应只命中 sid_b
        let filtered = list_sessions_with_discussion(dir.path(), &other_disc.to_string()).unwrap();
        assert_eq!(filtered.len(), 1, "should match only sid_b");
        assert_eq!(filtered[0].session_id, sid_b);

        // 不存在的 uuid → 空
        let unknown = Uuid::new_v4();
        let filtered = list_sessions_with_discussion(dir.path(), &unknown.to_string()).unwrap();
        assert!(filtered.is_empty());

        // 无效的 uuid 字符串 → 静默空(trace warn)
        let filtered = list_sessions_with_discussion(dir.path(), "not-a-uuid").unwrap();
        assert!(filtered.is_empty());
    }

    #[test]
    fn newest_first_ordering() {
        let dir = tempdir().unwrap();
        let older = Utc.with_ymd_and_hms(2026, 6, 17, 12, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2026, 6, 18, 12, 0, 0).unwrap();
        let sid_old = ThreadId::new();
        let sid_new = ThreadId::new();

        // Write directly with controlled started_at via session_path_at.
        let path_old = crate::path::session_path_at(dir.path(), sid_old, older);
        let path_new = crate::path::session_path_at(dir.path(), sid_new, newer);
        std::fs::create_dir_all(path_old.parent().unwrap()).unwrap();
        std::fs::create_dir_all(path_new.parent().unwrap()).unwrap();
        std::fs::write(
            &path_old,
            serde_json::to_string(&RolloutRecord::SessionMeta {
                session_id: sid_old,
                model: "m".into(),
                started_at: older,
            })
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            &path_new,
            serde_json::to_string(&RolloutRecord::SessionMeta {
                session_id: sid_new,
                model: "m".into(),
                started_at: newer,
            })
            .unwrap(),
        )
        .unwrap();

        let sessions = list_sessions(dir.path()).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, sid_new, "newest first");
        assert_eq!(sessions[1].session_id, sid_old);
    }

    // ── v0.4: resolve_session_index ──

    /// 准备一个含 N 条 session 的 tmpdir,返回 (dir, ids_newest_to_oldest)。
    fn prepare_n_sessions(n: usize) -> (tempfile::TempDir, Vec<ThreadId>) {
        let dir = tempdir().unwrap();
        let mut ids = Vec::new();
        for i in 0..n {
            let sid = ThreadId::new();
            let started = Utc.with_ymd_and_hms(2026, 6, 18, 12, 0, i as u32).unwrap();
            let path = crate::path::session_path_at(dir.path(), sid, started);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                serde_json::to_string(&RolloutRecord::SessionMeta {
                    session_id: sid,
                    model: "m".into(),
                    started_at: started,
                })
                .unwrap(),
            )
            .unwrap();
            ids.push(sid);
        }
        // newest first(对应 `list_sessions` 排序)
        ids.reverse();
        (dir, ids)
    }

    /// `-c` 选最新一条(list_sessions 已排序 newest first → [0])。
    #[test]
    fn resolve_session_index_continue_last_picks_newest() {
        let (dir, ids) = prepare_n_sessions(3);
        let picked = resolve_session_index(dir.path(), true, None).unwrap();
        assert_eq!(picked, ids[0]);
    }

    /// `-r 1` 等价 `-c`(都是最新)。
    #[test]
    fn resolve_session_index_resume_by_one_picks_newest() {
        let (dir, ids) = prepare_n_sessions(3);
        let picked = resolve_session_index(dir.path(), false, Some(1)).unwrap();
        assert_eq!(picked, ids[0]);
    }

    /// `-r 2` 选次新。
    #[test]
    fn resolve_session_index_resume_by_two_picks_second_newest() {
        let (dir, ids) = prepare_n_sessions(3);
        let picked = resolve_session_index(dir.path(), false, Some(2)).unwrap();
        assert_eq!(picked, ids[1]);
    }

    /// `-r 99` 越界 → Err(包含 "only N sessions")。
    #[test]
    fn resolve_session_index_resume_by_out_of_range_errors() {
        let (dir, _ids) = prepare_n_sessions(2);
        let r = resolve_session_index(dir.path(), false, Some(99));
        assert!(r.is_err());
        let msg = format!("{:#}", r.unwrap_err());
        assert!(msg.contains("only 2 sessions"), "got: {msg}");
    }

    /// `-r 0` → Err(must be >= 1)。
    #[test]
    fn resolve_session_index_resume_by_zero_rejected() {
        let (dir, _ids) = prepare_n_sessions(2);
        let r = resolve_session_index(dir.path(), false, Some(0));
        assert!(r.is_err());
    }

    /// 列表为空 → Err。
    #[test]
    fn resolve_session_index_empty_dir_errors() {
        let dir = tempdir().unwrap();
        let r = resolve_session_index(dir.path(), true, None);
        assert!(r.is_err());
        assert!(format!("{:#}", r.unwrap_err()).contains("no sessions found"));
    }

    /// 同时传 `-c` 和 `-r` → Err(互斥)。
    #[test]
    fn resolve_session_index_continue_last_and_resume_by_mutually_exclusive() {
        let (dir, _ids) = prepare_n_sessions(2);
        let r = resolve_session_index(dir.path(), true, Some(1));
        assert!(r.is_err());
    }

    /// 两者都为 None → Err(交给 caller 决定 fallback)。
    #[test]
    fn resolve_session_index_neither_flag_errors() {
        let (dir, _ids) = prepare_n_sessions(2);
        let r = resolve_session_index(dir.path(), false, None);
        assert!(r.is_err());
    }

    // ── S5b:find_session_path / rename_session / read_session_name / write_fork_record ─

    /// `find_session_path` 找到 prepare_n_sessions 写下的 JSONL,SessionMeta 匹配。
    #[test]
    fn find_session_path_resolves_to_existing_jsonl() {
        let (dir, ids) = prepare_n_sessions(3);
        let p = find_session_path(dir.path(), ids[0]).expect("should find");
        assert!(
            p.ends_with(format!("{}.jsonl", ids[0]).as_str()),
            "got: {p:?}"
        );
        assert!(p.is_file());
    }

    /// `find_session_path` 找不到随机 UUID → None(不 panic)。
    #[test]
    fn find_session_path_returns_none_for_unknown_id() {
        let (dir, _ids) = prepare_n_sessions(2);
        let bogus = ThreadId::new();
        assert!(find_session_path(dir.path(), bogus).is_none());
    }

    /// `rename_session` + `read_session_name` round-trip:写后能读回,
    /// 重复写(再 rename)覆盖前值。
    #[test]
    fn rename_session_round_trip() {
        let (dir, ids) = prepare_n_sessions(1);
        let id = ids[0];
        // 初始未命名 → read 返回 None。
        assert_eq!(read_session_name(dir.path(), id).unwrap(), None);
        // 第一次 rename。
        rename_session(dir.path(), id, "my-debug").unwrap();
        assert_eq!(
            read_session_name(dir.path(), id).unwrap().as_deref(),
            Some("my-debug")
        );
        // 第二次 rename 覆盖。
        rename_session(dir.path(), id, "production").unwrap();
        assert_eq!(
            read_session_name(dir.path(), id).unwrap().as_deref(),
            Some("production")
        );
    }

    /// `rename_session` 拒空名字 —— 避免写出空文件让 read 误判。
    #[test]
    fn rename_session_rejects_empty_name() {
        let (dir, ids) = prepare_n_sessions(1);
        assert!(rename_session(dir.path(), ids[0], "").is_err());
        assert!(rename_session(dir.path(), ids[0], "   \t  ").is_err());
        // 拒空后 _names/ 目录不应被创建(或创建了但没 .name 文件)。
        let names_dir = dir.path().join("_names");
        let entries: Vec<_> = std::fs::read_dir(&names_dir)
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        for e in &entries {
            assert!(e.path().extension().and_then(|s| s.to_str()) == Some("name"));
        }
    }

    /// `write_fork_record` 写一条 Fork record 到 parent JSONL 末尾,
    /// 返回新 ThreadId 且新 id 与 parent 不同。
    #[test]
    fn write_fork_record_appends_to_parent() {
        // 手工写一个**带换行**的 SessionMeta 文件;`prepare_n_sessions`
        // 不带换行,会让 fork 写入后 SessionMeta 与 fork 黏成一行,
        // `parse_first_session_meta` 失灵。
        let dir = tempdir().unwrap();
        let parent = ThreadId::new();
        let started = Utc.with_ymd_and_hms(2026, 6, 18, 12, 0, 0).unwrap();
        let path = crate::path::session_path_at(dir.path(), parent, started);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut body = String::new();
        body.push_str(
            &serde_json::to_string(&RolloutRecord::SessionMeta {
                session_id: parent,
                model: "m".into(),
                started_at: started,
            })
            .unwrap(),
        );
        body.push('\n');
        std::fs::write(&path, body).unwrap();

        let new_id = write_fork_record(dir.path(), parent, "explorer").unwrap();
        assert_ne!(new_id, parent);

        // 读回文件,扫所有 line 找 Fork record。
        let body = std::fs::read_to_string(&path).unwrap();
        let mut found_fork = false;
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(RolloutRecord::Fork {
                parent_session_id,
                branch_name,
            }) = serde_json::from_str::<RolloutRecord>(line)
            {
                assert_eq!(parent_session_id, parent);
                assert_eq!(branch_name, "explorer");
                found_fork = true;
            }
        }
        assert!(found_fork, "no Fork record found in body: {body}");
    }

    /// `write_fork_record` parent 不存在 → Err(NotFound) 含 UUID 字符串,
    /// 让 TUI Pill 友好显示。
    #[test]
    fn write_fork_record_errors_on_missing_parent() {
        let (dir, _ids) = prepare_n_sessions(1);
        let bogus = ThreadId::new();
        let r = write_fork_record(dir.path(), bogus, "any");
        assert!(r.is_err());
        let err = r.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        let msg = format!("{err}");
        assert!(msg.contains(&bogus.to_string()), "err msg: {msg}");
    }
}
