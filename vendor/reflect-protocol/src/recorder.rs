//! Rollout recorder contract (M5).
//!
//! `reflect-core` emits `RolloutRecord`s on every noteworthy event; a
//! `RolloutRecorder` implementation persists them. The trait lives in
//! `reflect-protocol` so `reflect-core` can take
//! `Option<Arc<dyn RolloutRecorder>>` without depending on
//! `reflect-rollout` (which would create a `core ↔ rollout` cycle).
//!
//! The concrete `JsonlRolloutWriter` lives in the `reflect-rollout` crate;
//! tests and short-lived runs can use `NullRecorder`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::item::{ThreadId, TurnId};

/// Marker for which side of the conversation a persisted message came from.
///
/// Intentionally minimal — the `content` payload carries the actual data as
/// `serde_json::Value`, and richer structures stay in `reflect-core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Lightweight session metadata used by the CLI `resume` flow and the
/// session index. `message_count` is updated lazily on each record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: ThreadId,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub message_count: usize,
}

/// One persisted event in a thread's JSONL rollout.
///
/// Serialised as a serde tagged union (`{"type": "...", ...}`) so external
/// tooling can `jq` the file without parsing the full Rust enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RolloutRecord {
    /// Emitted exactly once per thread, before any other record.
    SessionMeta {
        session_id: ThreadId,
        model: String,
        started_at: DateTime<Utc>,
    },
    /// One persisted turn of conversation. `content` is opaque JSON so the
    /// recorder can carry `ContentBlock`s, plain text, or tool payloads.
    Message {
        turn_id: TurnId,
        role: MessageRole,
        content: serde_json::Value,
    },
    /// Emitted by `pre_loop` after `Compactor::compact` runs and the strategy
    /// is not `Noop`. `summary` is the body of the synthetic `<summary>`
    /// System message that replaced the dropped slice.
    Compaction {
        turn_id: TurnId,
        strategy: String,
        removed_count: usize,
        summary: String,
    },
    /// Emitted by `SubAgentFactory::fork` to mark a branched session.
    Fork {
        parent_session_id: ThreadId,
        branch_name: String,
    },
    /// v1.2 P0-3:`CheckpointTool` 在 `git_auto_commit` 后发出,记录工作区
    /// 的 git 快照 sha。`rewind` 工具据此把工作区 `git reset --hard` 回该
    /// sha。append-only:不截断历史,只追加 marker。`checkpoint_id` 是
    /// 工具返回给 LLM 的稳定标识(此处 = sha,但保留独立字段以便未来用
    /// uuid 而 sha 仅作恢复目标)。
    Checkpoint {
        turn_id: TurnId,
        /// 工作区 git 快照的 commit sha(`git rev-parse HEAD`)。
        sha: String,
        /// 给人 / LLM 的标签(可选)。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        created_at: DateTime<Utc>,
    },
    /// v1.2 P0-3:`RewindTool` 在 `git_reset_hard` 后发出,记录回退到的
    /// 目标 checkpoint。append-only:会话历史保留,仅工作区文件回退。
    Rewind {
        turn_id: TurnId,
        /// 回退到的目标 checkpoint 的 sha。
        target_sha: String,
        /// 回退前的 HEAD sha(便于审计 / 再次前进)。
        from_sha: String,
        at: DateTime<Utc>,
    },
    /// M9: a full discussion transcript, emitted by `DiscussionOrchestrator`
    /// once the discussion completes (consensus / finish / max-rounds). The
    /// transcript payload is opaque JSON so `reflect-protocol` stays
    /// independent of `reflect-discussion` (no `core ↔ discussion` cycle).
    /// `reflect-rollout::redact` still applies its 16 KiB content cap.
    ///
    /// v0.2.4: `agent_id` is `None` for whole-discussion transcripts and
    /// `Some(agent_id)` for per-agent slices emitted by `prompt_for_closure`
    /// after each spawn. `#[serde(default, skip_serializing_if)]` keeps the
    /// wire compatible with M9 records that omit the field.
    DiscussionTranscript {
        discussion_id: Uuid,
        mode: String,
        participants: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        transcript: serde_json::Value,
    },
}

impl RolloutRecord {
    /// Convenience constructor for the `Message` variant.
    pub fn message(turn_id: TurnId, role: MessageRole, content: serde_json::Value) -> Self {
        RolloutRecord::Message {
            turn_id,
            role,
            content,
        }
    }

    /// Convenience constructor for `SessionMeta`.
    pub fn session_meta(session_id: ThreadId, model: impl Into<String>) -> Self {
        RolloutRecord::SessionMeta {
            session_id,
            model: model.into(),
            started_at: Utc::now(),
        }
    }
}

/// Pluggable persistence sink for [`RolloutRecord`]s.
///
/// `Send + Sync` so it can sit on `NodeContext`; `Debug` so it can be
/// pretty-printed in error paths. `async_trait` is used for dyn-compatibility
/// (Rust 2024 stable async fn in trait is not dyn-compatible).
#[async_trait]
pub trait RolloutRecorder: Send + Sync + std::fmt::Debug {
    /// Append one record to the underlying store. Implementations should be
    /// best-effort: a failed write logs and continues so a disk-full error
    /// does not abort an in-flight turn.
    async fn record(&self, r: RolloutRecord) -> anyhow::Result<()>;

    /// Replay every record for `session_id`. Malformed lines should be
    /// skipped with a `tracing::warn!` and not abort the whole replay.
    async fn replay(&self, session_id: ThreadId) -> anyhow::Result<Vec<RolloutRecord>>;

    /// List every persisted session, newest first. Cheap (only reads the
    /// first line of each file).
    async fn list_sessions(&self) -> anyhow::Result<Vec<SessionInfo>>;

    /// Destructive rewind: drop `to_turn_id` (inclusive) and every record
    /// after it, keeping only the records that came *before* that turn.
    /// `to_turn_id = None` drops the last turn (most recent turn boundary).
    ///
    /// Returns the number of dropped `Message` records (so the engine can
    /// surface `truncated_after` in `TurnRewoundEvent`). Returns `0` when the
    /// turn id is not present (no-op). Implementations that cannot rewind
    /// (`NullRecorder`, stubs, in-memory test doubles) return `Ok(0)`.
    ///
    /// Safety: the JSONL implementation writes a `.bak` sibling before
    /// truncating, so the dropped turns remain recoverable on disk.
    async fn truncate_after(&self, to_turn_id: Option<&TurnId>) -> anyhow::Result<usize>;
}

/// No-op recorder. Used by tests that don't care about persistence and by
/// `AgentConfig::default` so the absence of a recorder never panics.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullRecorder;

#[async_trait]
impl RolloutRecorder for NullRecorder {
    async fn record(&self, _r: RolloutRecord) -> anyhow::Result<()> {
        Ok(())
    }

    async fn replay(&self, _session_id: ThreadId) -> anyhow::Result<Vec<RolloutRecord>> {
        Ok(Vec::new())
    }

    async fn list_sessions(&self) -> anyhow::Result<Vec<SessionInfo>> {
        Ok(Vec::new())
    }

    async fn truncate_after(&self, _to_turn_id: Option<&TurnId>) -> anyhow::Result<usize> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollout_record_session_meta_serde() {
        let sid = ThreadId::new();
        let r = RolloutRecord::session_meta(sid, "openai/gpt-4o");
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"session_meta""#), "got: {j}");
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rollout_record_message_serde() {
        let tid = TurnId::new();
        let r = RolloutRecord::message(tid, MessageRole::User, serde_json::json!("hi"));
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"message""#));
        assert!(j.contains(r#""role":"user""#));
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rollout_record_compaction_serde() {
        let tid = TurnId::new();
        let r = RolloutRecord::Compaction {
            turn_id: tid,
            strategy: "microcompact".into(),
            removed_count: 12,
            summary: "<summary>...</summary>".into(),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"compaction""#));
        assert!(j.contains(r#""strategy":"microcompact""#));
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rollout_record_fork_serde() {
        let parent = ThreadId::new();
        let r = RolloutRecord::Fork {
            parent_session_id: parent,
            branch_name: "explorer-branch".into(),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"fork""#));
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rollout_record_checkpoint_serde() {
        let tid = TurnId::new();
        let r = RolloutRecord::Checkpoint {
            turn_id: tid,
            sha: "abc123".into(),
            label: Some("before-refactor".into()),
            created_at: Utc::now(),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"checkpoint""#));
        assert!(j.contains(r#""sha":"abc123""#));
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rollout_record_checkpoint_label_optional() {
        // label 缺省(None)必须能 round-trip(skip_serializing_if)。
        let j = r#"{"type":"checkpoint","turn_id":"00000000-0000-0000-0000-000000000001","sha":"def","created_at":"2026-07-02T00:00:00Z"}"#;
        let back: RolloutRecord = serde_json::from_str(j).unwrap();
        match back {
            RolloutRecord::Checkpoint { sha, label, .. } => {
                assert_eq!(sha, "def");
                assert_eq!(label, None);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn rollout_record_rewind_serde() {
        let tid = TurnId::new();
        let r = RolloutRecord::Rewind {
            turn_id: tid,
            target_sha: "abc123".into(),
            from_sha: "fff999".into(),
            at: Utc::now(),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"rewind""#));
        assert!(j.contains(r#""target_sha":"abc123""#));
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn rollout_record_discussion_transcript_serde() {
        // M9: DiscussionTranscript carries an opaque JSON payload so the
        // protocol crate stays decoupled from reflect-discussion's types.
        // v0.2.4: also carries optional `agent_id` for per-agent slices;
        // None branch must roundtrip via `skip_serializing_if`.
        let did = Uuid::new_v4();
        let payload = serde_json::json!([
            {"id": 0, "from": "a", "kind": "utterance", "content": "hi"},
            {"id": 1, "from": "b", "kind": "consensus", "content": "agreed"},
        ]);
        let r = RolloutRecord::DiscussionTranscript {
            discussion_id: did,
            mode: "concurrent".into(),
            participants: vec!["a".into(), "b".into()],
            agent_id: None,
            transcript: payload.clone(),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""type":"discussion_transcript""#), "got: {j}");
        assert!(j.contains(r#""mode":"concurrent""#), "got: {j}");
        assert!(
            !j.contains("agent_id"),
            "None agent_id must be skipped via skip_serializing_if, got: {j}"
        );
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        match back {
            RolloutRecord::DiscussionTranscript {
                discussion_id,
                mode,
                participants,
                agent_id,
                transcript,
            } => {
                assert_eq!(discussion_id, did);
                assert_eq!(mode, "concurrent");
                assert_eq!(participants, vec!["a", "b"]);
                assert_eq!(agent_id, None);
                assert_eq!(transcript, payload);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn discussion_transcript_with_agent_id_roundtrip() {
        // v0.2.4: per-agent transcript slice. `Some(agent_id)` must serialize
        // and round-trip.
        let did = Uuid::new_v4();
        let payload = serde_json::json!([
            {"id": 0, "from": "advocate", "kind": "utterance", "content": "I disagree"},
        ]);
        let r = RolloutRecord::DiscussionTranscript {
            discussion_id: did,
            mode: "sequential".into(),
            participants: vec!["advocate".into(), "skeptic".into()],
            agent_id: Some("advocate".into()),
            transcript: payload.clone(),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains(r#""agent_id":"advocate""#), "got: {j}");
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        match back {
            RolloutRecord::DiscussionTranscript {
                discussion_id,
                agent_id,
                ..
            } => {
                assert_eq!(discussion_id, did);
                assert_eq!(agent_id.as_deref(), Some("advocate"));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn discussion_transcript_backward_compat_with_m9_wire() {
        // M9 wire shape: `{"type":"discussion_transcript", ...}` without
        // `agent_id`. M9 producers don't write this field; v0.2.4 consumers
        // must decode it as `None`.
        let did = Uuid::new_v4();
        let j = format!(
            r#"{{"type":"discussion_transcript","discussion_id":"{did}","mode":"sequential","participants":["a"],"transcript":[]}}"#
        );
        let back: RolloutRecord = serde_json::from_str(&j).unwrap();
        match back {
            RolloutRecord::DiscussionTranscript {
                discussion_id,
                agent_id,
                ..
            } => {
                assert_eq!(discussion_id, did);
                assert_eq!(
                    agent_id, None,
                    "missing agent_id in M9 wire must decode as None"
                );
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn message_role_serde_uses_snake_case() {
        for (role, expected) in [
            (MessageRole::System, "\"system\""),
            (MessageRole::User, "\"user\""),
            (MessageRole::Assistant, "\"assistant\""),
            (MessageRole::Tool, "\"tool\""),
        ] {
            assert_eq!(serde_json::to_string(&role).unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn null_recorder_is_noop() {
        let r = NullRecorder;
        let sid = ThreadId::new();
        r.record(RolloutRecord::session_meta(sid, "m"))
            .await
            .unwrap();
        assert!(r.replay(sid).await.unwrap().is_empty());
        assert!(r.list_sessions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn null_recorder_truncate_after_is_zero() {
        // 批次二十二:truncate_after 在 NullRecorder 上是 no-op,返回 0。
        let r = NullRecorder;
        assert_eq!(r.truncate_after(None).await.unwrap(), 0);
        let tid = TurnId::new();
        assert_eq!(r.truncate_after(Some(&tid)).await.unwrap(), 0);
    }
}
