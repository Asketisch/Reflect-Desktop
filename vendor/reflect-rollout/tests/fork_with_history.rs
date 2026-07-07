//! Integration tests for `fork_with_history` —— fork-at-node 的核心存储原语。
//!
//! 验证:fork 真正创建子 JSONL、复制指定消息、双向标记 Fork 关系,
//! 且 `replay` 能无损回放子会话历史(`bootstrap_resume` 据此重建上下文)。

use std::io::Write;

use chrono::Utc;
use reflect_protocol::{MessageRole, RolloutRecord, ThreadId, TurnId};
use reflect_rollout::index::fork_with_history;
use reflect_rollout::path::{default_base, session_path_at};
use tempfile::tempdir;

/// 同步读取一个 JSONL 文件并解析成 `Vec<RolloutRecord>`。
///
/// 不走 async `reader::replay_path`(它依赖 tokio reactor,plain `cargo test`
/// 没有 runtime)。JSONL 是 append-only 单行一记录,直接按行 `serde_json` 解析
/// 与 `replay_path` 语义一致(空行跳过,畸形行 panic —— 测试里文件是受控的)。
fn replay_file(path: &std::path::Path) -> Vec<RolloutRecord> {
    let content = std::fs::read_to_string(path).unwrap();
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<RolloutRecord>(l).unwrap())
        .collect()
}

/// 构造一个父会话 JSONL:SessionMeta + N 条 assistant Message(+ 可选 Compaction)。
/// 返回 (parent_id, parent_path)。模拟 `submission_loop` 实际写入的形态
/// (user 消息不在 JSONL 里 —— fork 时由调用方从内存 history 补)。
fn write_parent(
    base: &std::path::Path,
    assistant_texts: &[&str],
    summary: Option<&str>,
    model: &str,
) -> (ThreadId, std::path::PathBuf) {
    let sid = ThreadId::new();
    let path = session_path_at(base, sid, Utc::now());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(
        f,
        "{}",
        serde_json::to_string(&RolloutRecord::SessionMeta {
            session_id: sid,
            model: model.to_string(),
            started_at: Utc::now(),
        })
        .unwrap()
    )
    .unwrap();
    if let Some(s) = summary {
        writeln!(
            f,
            "{}",
            serde_json::to_string(&RolloutRecord::Compaction {
                turn_id: TurnId::new(),
                strategy: "smart_prune".to_string(),
                removed_count: 3,
                summary: s.to_string(),
            })
            .unwrap()
        )
        .unwrap();
    }
    for t in assistant_texts {
        writeln!(
            f,
            "{}",
            serde_json::to_string(&RolloutRecord::message(
                TurnId::new(),
                MessageRole::Assistant,
                serde_json::Value::String((*t).to_string()),
            ))
            .unwrap()
        )
        .unwrap();
    }
    (sid, path)
}

#[test]
fn fork_with_history_creates_child_jsonl_with_copied_messages() {
    let dir = tempdir().unwrap();
    let base = dir.path();
    let (parent, _parent_path) = write_parent(
        base,
        &["answer-1", "answer-2"],
        Some("prior context summary"),
        "openai/gpt-4o",
    );

    // 调用方(TUI)重建好的有序消息:2 个 (user, assistant) 对。
    let up_to = vec![
        (MessageRole::User, "question-1".to_string()),
        (MessageRole::Assistant, "answer-1".to_string()),
        (MessageRole::User, "question-2".to_string()),
        (MessageRole::Assistant, "answer-2".to_string()),
    ];
    let child = fork_with_history(
        base,
        parent,
        "fix-branch",
        &up_to,
        Some("prior context summary"),
        "openai/gpt-4o",
    )
    .unwrap();

    // 子文件存在 + 可被 find_session_path 定位(首行 SessionMeta 用 child_id)。
    let child_path = reflect_rollout::index::find_session_path(base, child);
    assert!(child_path.is_some(), "child JSONL should be discoverable");

    let records = replay_file(child_path.as_ref().unwrap());
    // 期望:SessionMeta + Compaction(inherited) + 2×User + 2×Assistant + Fork = 7
    assert_eq!(records.len(), 7, "got {records:?}");

    // 首行是 child 的 SessionMeta(id 必须是 child,不能误用 parent)。
    match &records[0] {
        RolloutRecord::SessionMeta { session_id, model, .. } => {
            assert_eq!(*session_id, child, "child SessionMeta must use child id");
            assert_eq!(model, "openai/gpt-4o");
        }
        other => panic!("expected SessionMeta first, got {other:?}"),
    }
    // 第二行继承的 Compaction。
    assert!(matches!(
        &records[1],
        RolloutRecord::Compaction { summary, strategy, .. }
        if summary == "prior context summary" && strategy == "fork_inherited"
    ));
    // 末行 Fork 指向 parent。
    match records.last() {
        Some(RolloutRecord::Fork { parent_session_id, branch_name }) => {
            assert_eq!(*parent_session_id, parent);
            assert_eq!(branch_name, "fix-branch");
        }
        other => panic!("expected Fork last, got {other:?}"),
    }
}

#[test]
fn fork_with_history_truncates_at_up_to_messages() {
    let dir = tempdir().unwrap();
    let base = dir.path();
    let (parent, _) = write_parent(base, &["a1", "a2", "a3"], None, "m");

    // 只复制前 1 对(用户在第 1 轮 fork)。
    let up_to = vec![
        (MessageRole::User, "q1".to_string()),
        (MessageRole::Assistant, "a1".to_string()),
    ];
    let child = fork_with_history(base, parent, "b", &up_to, None, "m").unwrap();
    let child_path = reflect_rollout::index::find_session_path(base, child).unwrap();
    let records = replay_file(&child_path);
    // SessionMeta + 1 User + 1 Assistant + Fork = 4。无 Compaction(summary=None)。
    assert_eq!(records.len(), 4, "truncated child should have 4 records, got {records:?}");
    let roles: Vec<_> = records.iter().filter_map(|r| match r {
        RolloutRecord::Message { role, content, .. } => {
            Some((*role, content.as_str().unwrap().to_string()))
        }
        _ => None,
    }).collect();
    assert_eq!(roles, vec![
        (MessageRole::User, "q1".to_string()),
        (MessageRole::Assistant, "a1".to_string()),
    ]);
}

#[test]
fn fork_with_history_missing_parent_returns_not_found() {
    let dir = tempdir().unwrap();
    let base = dir.path();
    let ghost = ThreadId::new();
    let err = fork_with_history(
        base,
        ghost,
        "b",
        &[(MessageRole::User, "hi".to_string())],
        None,
        "m",
    )
    .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn fork_with_history_appends_fork_marker_to_parent() {
    let dir = tempdir().unwrap();
    let base = dir.path();
    let (parent, parent_path) = write_parent(base, &["a1"], None, "m");
    // fork 前父文件:SessionMeta + 1 Assistant = 2 行。
    assert_eq!(replay_file(&parent_path).len(), 2);

    let _child = fork_with_history(
        base,
        parent,
        "b",
        &[(MessageRole::User, "q1".to_string()),
          (MessageRole::Assistant, "a1".to_string())],
        None,
        "m",
    )
    .unwrap();

    // fork 后父文件末尾多一条 Fork marker。
    let parent_records = replay_file(&parent_path);
    assert_eq!(parent_records.len(), 3, "parent should gain a Fork marker");
    assert!(matches!(parent_records.last(), Some(RolloutRecord::Fork { parent_session_id, .. }) if *parent_session_id == parent));
}

#[test]
fn fork_with_history_without_compaction_or_messages() {
    // 边界:无 compaction + 空消息列表 → 子文件只有 SessionMeta + Fork。
    let dir = tempdir().unwrap();
    let base = dir.path();
    let (parent, _) = write_parent(base, &[], None, "m");
    let child = fork_with_history(base, parent, "b", &[], None, "m").unwrap();
    let child_path = reflect_rollout::index::find_session_path(base, child).unwrap();
    let records = replay_file(&child_path);
    assert_eq!(records.len(), 2, "minimal child = SessionMeta + Fork, got {records:?}");
    assert!(matches!(records[0], RolloutRecord::SessionMeta { .. }));
    assert!(matches!(records[1], RolloutRecord::Fork { .. }));
}

/// 防回归:`default_base()` 仍指向 `$HOME/.reflect/sessions`,fork 路径不碰它。
#[test]
fn default_base_unchanged() {
    let p = default_base();
    assert!(p.ends_with(".reflect/sessions"), "got {p:?}");
}
