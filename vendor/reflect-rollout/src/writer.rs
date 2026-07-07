//! JSONL rollout writer with size-based rotation.
//!
//! One [`JsonlRolloutWriter`] instance is bound to a single [`ThreadId`]
//! and writes its records to `<base>/YYYY/MM/DD/<thread_id>.jsonl`. When
//! the file crosses [`crate::types::ROTATE_AFTER_BYTES`] it rotates:
//!
//! 1. Close the current file.
//! 2. Move `<id>.jsonl → <id>.1.jsonl`,
//!    `<id>.1.jsonl → <id>.2.jsonl`,
//!    `<id>.2.jsonl → <id>.3.jsonl`.
//! 3. Delete `<id>.3.jsonl` (so at most [`crate::types::MAX_ROTATED_FILES`]
//!    rotated copies survive).
//! 4. Re-open a fresh `<id>.jsonl` and **re-emit the `SessionMeta` line**.
//!
//! # Why re-emit `SessionMeta` on rotation
//!
//! The session index (`crate::index::parse_first_session_meta`) requires every
//! `<id>[.N].jsonl` file's first line to be a `SessionMeta`. `SessionMeta` is
//! otherwise emitted exactly once per thread (in `submission_loop`); without
//! re-emission, after `MAX_ROTATED_FILES + 1` rotations the sole
//! `SessionMeta`-bearing file gets deleted and the session vanishes from
//! `/session`. To preserve the "first line = SessionMeta" invariant across
//! rotation, the writer remembers the meta on first sighting and writes it as
//! the first line of every freshly rotated file.
//!
//! Concurrency: a `parking_lot::Mutex` serialises every write so the writer
//! is `Send + Sync` and safe to share behind an `Arc`.

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::path::session_path_at;
use crate::redact::serialize_redacted;
use crate::types::{MAX_ROTATED_FILES, ROTATE_AFTER_BYTES};
use chrono::{DateTime, Utc};
use reflect_protocol::{RolloutRecord, RolloutRecorder, SessionInfo, ThreadId, TurnId};

/// Cached `SessionMeta` snapshot for re-emission on rotation. `session_id`
/// lives on [`JsonlRolloutWriter`]; only `model` + `started_at` are cached
/// here so `rotate()` can synthesise a `SessionMeta` as the first line of the
/// freshly opened file (see module doc on why this invariant matters).
#[derive(Clone)]
struct CachedSessionMeta {
    model: String,
    started_at: DateTime<Utc>,
}

/// Internal mutable state guarded by the outer `Mutex`.
struct Inner {
    current_path: Option<PathBuf>,
    /// Always `Some` when `current_path` is `Some`; the buffer flushes on
    /// every `record()` call so a process crash loses at most one record.
    writer: Option<BufWriter<File>>,
    current_size: u64,
    /// Set the first time a `SessionMeta` record flows through `record()`.
    /// Used by `rotate()` to re-emit the meta as the first line of the new
    /// file so the index's "first line = SessionMeta" contract holds. `None`
    /// before any `SessionMeta` has been seen (rotation is a no-op then).
    session_meta: Option<CachedSessionMeta>,
}

/// Persistent JSONL writer for one thread.
pub struct JsonlRolloutWriter {
    base_dir: PathBuf,
    session_id: ThreadId,
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for JsonlRolloutWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonlRolloutWriter")
            .field("base_dir", &self.base_dir)
            .field("session_id", &self.session_id)
            .finish()
    }
}

impl JsonlRolloutWriter {
    /// Build a writer rooted at `base_dir`. The first `record()` call lazily
    /// creates the per-date directory.
    pub fn new(base_dir: impl Into<PathBuf>, session_id: ThreadId) -> Self {
        Self {
            base_dir: base_dir.into(),
            session_id,
            inner: Mutex::new(Inner {
                current_path: None,
                writer: None,
                current_size: 0,
                session_meta: None,
            }),
        }
    }

    /// Base directory the writer writes under.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Thread id the writer is bound to.
    pub fn session_id(&self) -> ThreadId {
        self.session_id
    }

    fn ensure_open(&self, g: &mut Inner) -> std::io::Result<()> {
        if g.current_path.is_some() {
            return Ok(());
        }
        let path = session_path_at(&self.base_dir, self.session_id, chrono::Utc::now());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        g.writer = Some(BufWriter::new(file));
        g.current_path = Some(path);
        g.current_size = size;
        Ok(())
    }

    fn rotate(g: &mut Inner, session_id: ThreadId) -> std::io::Result<()> {
        let Some(current) = g.current_path.take() else {
            return Ok(());
        };
        // Flush + drop the writer so the rename is on a closed file.
        if let Some(mut w) = g.writer.take() {
            w.flush()?;
        }

        let Some(stem) = current
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
        else {
            return Ok(());
        };
        let Some(parent) = current.parent() else {
            return Ok(());
        };

        // Shift from highest index down so each .N takes the previous .N-1.
        for n in (1..=MAX_ROTATED_FILES).rev() {
            let src = if n == 1 {
                current.clone()
            } else {
                parent.join(format!(
                    "{}.{}.jsonl",
                    stem.trim_end_matches(".jsonl"),
                    n - 1
                ))
            };
            let dst = parent.join(format!("{}.{}.jsonl", stem.trim_end_matches(".jsonl"), n));
            if src.exists() {
                if dst.exists() {
                    fs::remove_file(&dst)?;
                }
                fs::rename(&src, &dst)?;
            }
        }

        // Re-open a fresh file at the original path.
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&current)?;
        g.writer = Some(BufWriter::new(file));
        g.current_path = Some(current);
        g.current_size = 0;

        // Re-emit `SessionMeta` as the first line so the freshly opened file
        // satisfies the index's "first line = SessionMeta" contract. Without
        // this, every rotated file would start with a `Message` and the
        // session would fall out of `/session` after enough rotations.
        if let Some(meta) = g.session_meta.clone() {
            let line = serialize_redacted(&RolloutRecord::SessionMeta {
                session_id,
                model: meta.model,
                started_at: meta.started_at,
            })?;
            if let Some(w) = g.writer.as_mut() {
                writeln!(w, "{line}")?;
                w.flush()?;
            }
            // Account for the meta line we just wrote (string bytes + newline).
            g.current_size = g.current_size.saturating_add(line.len() as u64 + 1);
        }
        Ok(())
    }
}

#[async_trait]
impl RolloutRecorder for JsonlRolloutWriter {
    async fn record(&self, r: RolloutRecord) -> anyhow::Result<()> {
        let line = serialize_redacted(&r)?;
        let line_bytes = line.len() as u64 + 1; // +1 for newline

        let mut g = self.inner.lock();
        // Cache the SessionMeta on first sighting so `rotate()` can re-emit it
        // as the first line of every rotated file. Only the first occurrence
        // is kept (the engine emits exactly one SessionMeta per thread).
        if g.session_meta.is_none()
            && let RolloutRecord::SessionMeta { model, started_at, .. } = &r
        {
            g.session_meta = Some(CachedSessionMeta {
                model: model.clone(),
                started_at: *started_at,
            });
        }

        Self::ensure_open(self, &mut g)?;
        if let Some(w) = g.writer.as_mut() {
            writeln!(w, "{}", line)?;
            w.flush()?;
        }
        g.current_size = g.current_size.saturating_add(line_bytes);

        if g.current_size >= ROTATE_AFTER_BYTES {
            Self::rotate(&mut g, self.session_id)?;
        }
        Ok(())
    }

    async fn replay(&self, session_id: ThreadId) -> anyhow::Result<Vec<RolloutRecord>> {
        let path = if session_id != self.session_id {
            session_path_at(&self.base_dir, session_id, chrono::Utc::now())
        } else {
            let g = self.inner.lock();
            let Some(path) = g.current_path.clone() else {
                // Nothing was ever written — return empty.
                drop(g);
                return Ok(Vec::new());
            };
            drop(g);
            path
        };
        crate::reader::replay_path(&path).await
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<SessionInfo>> {
        crate::index::list_sessions(&self.base_dir).map_err(anyhow::Error::from)
    }

    /// 破坏性回退:`to_turn_id`(含)及其后的记录全部丢弃,只保留之前的。
    /// `None` = 丢弃最后一个 turn。
    ///
    /// # 流程(全在 `inner` mutex 内,同步 IO)
    /// 1. 读当前活跃文件全文(`current_path`),逐行解析定位截断行。
    /// 2. 未找到目标 turn → no-op,返回 0。
    /// 3. **备份**:复制全文到 `<id>.rewind-<unix_ts>.bak`(同目录,可恢复)。
    /// 4. 原子 rewrite:写 `.tmp`(SessionMeta + 截断行之前)→ `fs::rename` 覆盖。
    /// 5. 重开 `BufWriter`(append 模式),更新 `current_size`,保留 `session_meta` 缓存。
    /// 6. 返回被丢弃的 `Message` 记录数。
    ///
    /// 仅作用于当前活跃文件;已 rotate 的 `.N` 文件不处理(罕见边界,/fork 是替代)。
    async fn truncate_after(&self, to_turn_id: Option<&TurnId>) -> anyhow::Result<usize> {
        let mut g = self.inner.lock();
        Self::ensure_open(self, &mut g)?;
        Self::truncate_inner(&mut g, to_turn_id)
    }
}

impl JsonlRolloutWriter {
    /// `truncate_after` 的同步核心,持有 `inner` 锁的调用方使用。
    /// 解析当前活跃文件,定位 `to_turn_id`(或 None = 最后一个 turn)所在行,
    /// 备份 + 原子 rewrite 到该行之前(不含)。返回丢弃的 `Message` 记录数。
    fn truncate_inner(g: &mut Inner, to_turn_id: Option<&TurnId>) -> anyhow::Result<usize> {
        let path = g
            .current_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("truncate_after: no active rollout file"))?;

        // 全文读 + 逐行解析,记录每行的 (字节长度, turn_id 若有, 是否 Message)。
        let content = std::fs::read_to_string(&path)?;
        if content.is_empty() {
            return Ok(0);
        }

        // 每个 entry:(line_start_byte, line_str, turn_id, is_message)。
        let mut lines: Vec<(usize, &str, Option<TurnId>, bool)> = Vec::new();
        let mut offset = 0usize;
        for line in content.split_inclusive('\n') {
            let trimmed = line.trim_end_matches('\n');
            let (turn_id, is_message) = parse_line_meta(trimmed);
            lines.push((offset, trimmed, turn_id, is_message));
            offset += line.len();
        }

        // 定位截断行索引(该行及其后全部丢弃)。
        let cut_idx = match locate_cut(&lines, to_turn_id) {
            Some(i) => i,
            None => return Ok(0), // 未找到目标 turn,no-op。
        };

        // 备份:整文件复制到 <id>.rewind-<ts>.bak(可恢复)。
        write_backup(&path, &content)?;

        // 保留 cut_idx 之前的行 + 结尾换行。
        let mut kept = String::new();
        let mut dropped_messages = 0usize;
        for (i, (_, line, _, is_message)) in lines.iter().enumerate() {
            if i < cut_idx {
                kept.push_str(line);
                kept.push('\n');
            } else if *is_message {
                dropped_messages += 1;
            }
        }

        // 原子 rewrite:写 .tmp → rename 覆盖(匹配 reflect-task/reflect-plugin 约定)。
        let tmp = path.with_extension("jsonl.tmp");
        std::fs::write(&tmp, &kept)?;
        std::fs::rename(&tmp, &path)?;

        // 重开 BufWriter(append),让后续 record() 继续追加;保留 session_meta 缓存。
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        if let Some(mut w) = g.writer.take() {
            let _ = w.flush();
        }
        g.writer = Some(BufWriter::new(file));
        g.current_size = kept.len() as u64;

        Ok(dropped_messages)
    }
}

/// 解析一行 JSONL,返回 (该行关联的 turn_id 若有, 是否为 Message 记录)。
/// turn_id 来自 Message/Compaction/Checkpoint/Rewind 变体。
fn parse_line_meta(line: &str) -> (Option<TurnId>, bool) {
    // 用 serde_json::Value 解包,避免对全枚举 from_str 的版本/变体耦合。
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
        return (None, false);
    };
    let ty = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let is_message = ty == "message";
    let turn_id = v.get("turn_id").and_then(|t| t.as_str()).and_then(|s| {
        // 解析失败(malformed)→ None,与 reader 的宽容策略一致。
        TurnId::parse_str(s).ok()
    });
    (turn_id, is_message)
}

/// 在已解析的行列表里定位截断行索引。
/// `Some(tid)` → 第一行 turn_id == tid 的索引。
/// `None` → 最后一个有 turn_id 的 turn 的「首行」索引(丢弃整个最后 turn)。
fn locate_cut(
    lines: &[(usize, &str, Option<TurnId>, bool)],
    to_turn_id: Option<&TurnId>,
) -> Option<usize> {
    match to_turn_id {
        Some(target) => lines
            .iter()
            .position(|(_, _, tid, _)| *tid == Some(*target)),
        None => {
            // 找最后一个 turn_id 的值,再回到该 turn 在文件中的第一次出现。
            let last_tid = lines.iter().rev().find_map(|(_, _, tid, _)| *tid)?;
            lines
                .iter()
                .position(|(_, _, tid, _)| *tid == Some(last_tid))
        }
    }
}

/// 把截断前的全文复制到 `<path>.rewind-<unix_ts>.bak`(同目录),可恢复。
/// 失败不致命:warn 后继续(用户已通过原子 rewrite 得到一致性保证)。
fn write_backup(path: &Path, content: &str) -> anyhow::Result<()> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut bak = path.to_path_buf();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    bak.set_file_name(format!("{stem}.rewind-{ts}.bak"));
    // 极端情况下同秒重复 rewind:加 -2/-3 直到不冲突。
    let mut n = 2;
    while bak.exists() {
        bak.set_file_name(format!("{stem}.rewind-{ts}-{n}.bak"));
        n += 1;
    }
    std::fs::write(&bak, content)?;
    tracing::debug!("rollout rewind backup written: {}", bak.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_protocol::{MessageRole, TurnId};
    use tempfile::tempdir;

    fn big_record(n: usize) -> RolloutRecord {
        RolloutRecord::message(
            TurnId::new(),
            MessageRole::Assistant,
            serde_json::json!({"i": n, "blob": "x".repeat(2048)}),
        )
    }

    #[tokio::test]
    async fn append_writes_single_line() {
        let dir = tempdir().unwrap();
        let sid = ThreadId::new();
        let w = JsonlRolloutWriter::new(dir.path(), sid);
        w.record(RolloutRecord::session_meta(sid, "openai/gpt-4o"))
            .await
            .unwrap();
        w.record(big_record(1)).await.unwrap();

        let path = session_path_at(dir.path(), sid, chrono::Utc::now());
        let body = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 2, "got lines: {lines:?}");
        // First line is SessionMeta
        let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["type"], "session_meta");
    }

    #[tokio::test]
    async fn rotates_after_threshold() {
        let dir = tempdir().unwrap();
        let sid = ThreadId::new();
        let w = JsonlRolloutWriter::new(dir.path(), sid);
        // Each record ~ 2 KiB; 200 records => ~400 KiB => forces >1 rotation.
        for i in 0..200 {
            w.record(big_record(i)).await.unwrap();
        }
        // The rotated file exists at .1.jsonl
        let rotated = dir
            .path()
            .join(".")
            .join("..")
            .canonicalize()
            .unwrap()
            .join("2026");
        // Easier: glob the date dir for any .1.jsonl
        let mut found_rotated = false;
        for entry in walkdir(dir.path()) {
            if entry.ends_with(".1.jsonl") {
                found_rotated = true;
                break;
            }
        }
        assert!(found_rotated, "expected a rotated .1.jsonl file");
        // Suppress unused.
        let _ = rotated;
    }

    #[tokio::test]
    async fn caps_rotated_files_at_max() {
        let dir = tempdir().unwrap();
        let sid = ThreadId::new();
        let w = JsonlRolloutWriter::new(dir.path(), sid);
        // Force 5 rotations: 5 * ROTATE_AFTER_BYTES worth of data.
        for i in 0..(5 * 200) {
            w.record(big_record(i)).await.unwrap();
        }
        let mut indices = Vec::new();
        for entry in walkdir(dir.path()) {
            if let Some(name) = std::path::Path::new(&entry)
                .file_name()
                .and_then(|s| s.to_str())
                && let Some(rest) = name.strip_prefix(&format!("{sid}."))
                && let Some(num) = rest.strip_suffix(".jsonl")
            {
                indices.push(num.to_string());
            }
        }
        let max_idx: usize = indices
            .iter()
            .filter_map(|s| s.parse().ok())
            .max()
            .unwrap_or(0);
        assert!(
            max_idx <= MAX_ROTATED_FILES,
            "max rotated index {max_idx} > MAX_ROTATED_FILES {MAX_ROTATED_FILES}"
        );
    }

    /// Bug fix: after rotation the active file and every rotated `.N` copy must
    /// start with a `SessionMeta` (same `session_id`). Before the fix, rotation
    /// deleted the sole `SessionMeta`-bearing file and re-opened a meta-less
    /// active file, so the session disappeared from `/session`.
    #[tokio::test]
    async fn rotation_keeps_session_meta_in_each_file() {
        let dir = tempdir().unwrap();
        let sid = ThreadId::new();
        let w = JsonlRolloutWriter::new(dir.path(), sid);
        // Seed the meta (cached), then force several rotations.
        w.record(RolloutRecord::session_meta(sid, "openai/gpt-4o"))
            .await
            .unwrap();
        for i in 0..(5 * 200) {
            w.record(big_record(i)).await.unwrap();
        }

        let mut checked = 0;
        for entry in walkdir(dir.path()) {
            let path = std::path::Path::new(&entry);
            if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }
            let body = std::fs::read_to_string(path).unwrap();
            let first = body.lines().next().expect("non-empty file");
            let v: serde_json::Value = serde_json::from_str(first).unwrap();
            assert_eq!(
                v["type"], "session_meta",
                "{} first line is not SessionMeta",
                path.display()
            );
            assert_eq!(
                v["session_id"].as_str().unwrap(),
                sid.to_string(),
                "{} SessionMeta session_id mismatch",
                path.display()
            );
            checked += 1;
        }
        assert!(checked >= 2, "expected ≥2 jsonl files, checked {checked}");
    }

    #[tokio::test]
    async fn redaction_applied_on_write() {
        let dir = tempdir().unwrap();
        let sid = ThreadId::new();
        let w = JsonlRolloutWriter::new(dir.path(), sid);
        w.record(RolloutRecord::message(
            TurnId::new(),
            MessageRole::User,
            serde_json::json!("x".repeat(20_000)),
        ))
        .await
        .unwrap();
        let path = session_path_at(dir.path(), sid, chrono::Utc::now());
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("[redacted]"), "missing marker in body");
        // Body should be well under raw 20KB.
        assert!(body.len() < 18_000, "body too large: {}", body.len());
    }

    // Tiny helper: walk one level of date dirs collecting all paths as strings.
    fn walkdir(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        fn walk(p: &Path, out: &mut Vec<String>) {
            if let Ok(rd) = std::fs::read_dir(p) {
                for e in rd.flatten() {
                    let path = e.path();
                    if path.is_dir() {
                        walk(&path, out);
                    } else {
                        out.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
        walk(root, &mut out);
        out
    }

    // ── truncate_after 测试(批次二十二)──────────────────────────────────

    /// 写一个含多 turn 的会话:SessionMeta + 3 个 user/assistant 对。
    /// 返回各 Message 的 turn_id 供测试定位截断点。
    async fn seed_three_turns(
        dir: &Path,
    ) -> (
        JsonlRolloutWriter,
        ThreadId,
        Vec<TurnId>, // 每个 Message 的 turn_id,共 6 条
    ) {
        let sid = ThreadId::new();
        let w = JsonlRolloutWriter::new(dir.to_path_buf(), sid);
        w.record(RolloutRecord::session_meta(sid, "openai/gpt-4o"))
            .await
            .unwrap();
        let mut tids = Vec::new();
        for i in 0..6 {
            let tid = TurnId::new();
            let role = if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            };
            w.record(RolloutRecord::message(
                tid,
                role,
                serde_json::json!(format!("msg-{i}")),
            ))
            .await
            .unwrap();
            tids.push(tid);
        }
        (w, sid, tids)
    }

    #[tokio::test]
    async fn truncate_after_specific_turn_drops_turn_and_after() {
        let dir = tempdir().unwrap();
        let (w, sid, tids) = seed_three_turns(dir.path()).await;
        // 截断到 tids[2](第 3 条 Message)→ 应保留 SessionMeta + tids[0,1],
        // 丢弃 tids[2..6](4 条 Message)。
        let dropped = w.truncate_after(Some(&tids[2])).await.unwrap();
        assert_eq!(dropped, 4, "应丢弃 tids[2..6] 共 4 条 Message");

        let records = w.replay(sid).await.unwrap();
        // SessionMeta + 2 Message = 3 records。
        assert_eq!(records.len(), 3, "got: {records:?}");
        // 第一行仍是 SessionMeta。
        assert!(matches!(records[0], RolloutRecord::SessionMeta { .. }));
        // 剩余 Message 的 turn_id == tids[0], tids[1]。
        if let RolloutRecord::Message { turn_id, .. } = &records[1] {
            assert_eq!(*turn_id, tids[0]);
        } else {
            panic!("records[1] 应是 Message");
        }
        if let RolloutRecord::Message { turn_id, .. } = &records[2] {
            assert_eq!(*turn_id, tids[1]);
        } else {
            panic!("records[2] 应是 Message");
        }
    }

    #[tokio::test]
    async fn truncate_after_none_drops_last_turn() {
        let dir = tempdir().unwrap();
        let (w, sid, _tids) = seed_three_turns(dir.path()).await;
        // None = 丢弃最后一个 turn(tids[4] + tids[5] 属同一 turn? 这里每条
        // Message 有独立 turn_id,故 None 丢弃最后一个 turn_id 的整 turn = tids[5])。
        let dropped = w.truncate_after(None).await.unwrap();
        assert_eq!(dropped, 1, "None 应丢弃最后一条 Message");

        let records = w.replay(sid).await.unwrap();
        // SessionMeta + 5 Message。
        assert_eq!(records.len(), 6, "got: {records:?}");
    }

    #[tokio::test]
    async fn truncate_after_unknown_turn_is_noop() {
        let dir = tempdir().unwrap();
        let (w, sid, _tids) = seed_three_turns(dir.path()).await;
        let before = w.replay(sid).await.unwrap();
        let dropped = w
            .truncate_after(Some(&TurnId::new())) // 不存在的 tid
            .await
            .unwrap();
        assert_eq!(dropped, 0, "未知 tid 应是 no-op");
        let after = w.replay(sid).await.unwrap();
        assert_eq!(before.len(), after.len());
        // 不应产生 .bak(未改动)。
        assert!(
            walkdir(dir.path())
                .iter()
                .all(|p| !p.contains(".bak")),
            "no-op 不应写备份"
        );
    }

    #[tokio::test]
    async fn truncate_after_keeps_session_meta_first_line() {
        let dir = tempdir().unwrap();
        let (w, sid, tids) = seed_three_turns(dir.path()).await;
        // 截断到第一个 Message(tids[0])→ 只剩 SessionMeta。
        let dropped = w.truncate_after(Some(&tids[0])).await.unwrap();
        assert_eq!(dropped, 6, "丢弃全部 6 条 Message");

        let records = w.replay(sid).await.unwrap();
        assert_eq!(records.len(), 1, "只剩 SessionMeta");
        assert!(matches!(records[0], RolloutRecord::SessionMeta { .. }));
    }

    #[tokio::test]
    async fn truncate_after_creates_backup_file() {
        let dir = tempdir().unwrap();
        let (w, sid, tids) = seed_three_turns(dir.path()).await;
        // 先记下截断前的全文。
        let path = session_path_at(dir.path(), sid, chrono::Utc::now());
        let pre_content = std::fs::read_to_string(&path).unwrap();

        let _ = w.truncate_after(Some(&tids[3])).await.unwrap();

        // 应存在一个 .bak 文件,内容 == 截断前全文。
        let mut bak_content: Option<String> = None;
        for p in walkdir(dir.path()) {
            if p.contains(".rewind-") && p.ends_with(".bak") {
                bak_content = Some(std::fs::read_to_string(&p).unwrap());
                break;
            }
        }
        let bak = bak_content.expect("应存在 .bak 备份");
        assert_eq!(bak, pre_content, "备份应 == 截断前全文");
    }

    #[tokio::test]
    async fn truncate_after_then_record_appends_correctly() {
        let dir = tempdir().unwrap();
        let (w, sid, tids) = seed_three_turns(dir.path()).await;
        // 截断到中间,再追加一条新 Message,确认文件可继续写 + SessionMeta 仍在第一行。
        let _ = w.truncate_after(Some(&tids[2])).await.unwrap(); // 剩 SessionMeta + 2 Message
        let new_tid = TurnId::new();
        w.record(RolloutRecord::message(
            new_tid,
            MessageRole::User,
            serde_json::json!("post-rewind"),
        ))
        .await
        .unwrap();

        let records = w.replay(sid).await.unwrap();
        // SessionMeta + 2(保留) + 1(新)= 4。
        assert_eq!(records.len(), 4, "got: {records:?}");
        assert!(matches!(records[0], RolloutRecord::SessionMeta { .. }));
        // 最后一条是新 turn_id。
        if let RolloutRecord::Message { turn_id, .. } = records.last().unwrap() {
            assert_eq!(*turn_id, new_tid);
        } else {
            panic!("最后一条应是新 Message");
        }
    }
}
