//! `AgentState` — the 4-node StateGraph state (M2).
//!
//! Carries per-turn state (messages, iteration counter, tool calls, etc.).
//! M1 doesn't use this; `submission_loop` keeps state implicitly. M2 will
//! move state into `AgentState` and pass it through the node chain.

#![allow(dead_code)]

use reflect_llm::{ChatMessage, SystemBlocks, ToolSpec};
use reflect_protocol::{ContentBlock, TokenUsage};
use reflect_recovery::RecoveryEntry;
use std::collections::HashMap;

/// Full conversation message history (system + user + assistant + tool).
#[derive(Debug, Default)]
pub struct MessageHistory {
    pub messages: Vec<ChatMessage>,
}

/// Tool specs available for the current turn.
#[derive(Debug, Default)]
pub struct ToolSpecs(pub Vec<ToolSpec>);

/// Per-turn mutable state.
#[derive(Debug, Default)]
pub struct AgentState {
    /// Full conversation history. M4: this is the canonical store;
    /// `pre_loop` populates it from `NodeContext.messages` and then
    /// `model_call` reads from it.
    pub messages: MessageHistory,
    /// Current iteration (model_call count since session start).
    pub iteration: u32,
    /// Number of times a Stop hook has vetoed completion this turn.
    pub stop_hook_attempts: u32,
    /// Number of search calls (used by `search_budget` hook in M3).
    pub search_calls: u32,
    /// True if compaction has been triggered this turn.
    pub compact_triggered: bool,
    /// File diffs accumulated this turn (used for commit messages in M5).
    pub file_diffs: HashMap<std::path::PathBuf, String>,
    /// Cumulative token usage this session.
    pub total_usage: TokenUsage,
    /// Most recent content blocks (text + tool calls) emitted by the LLM.
    pub latest_content: Vec<ContentBlock>,
    /// `true` once `StateGraph::run` finishes without a fatal error.
    /// Used by `submission_loop` to decide whether to emit `TurnComplete`.
    pub completed_normally: bool,
    /// v1.2 P1-12:`true` once `model_call` early-returned because the
    /// session-level token budget was exhausted. Distinct from
    /// `completed_normally` (which marks a natural `CheckStop` end) so
    /// `submission_loop` can emit a `TurnComplete` with
    /// `TurnStatus::TokenBudgetExceeded` rather than dropping the turn
    /// silently. Reset each turn via `AgentState::default`.
    pub budget_exceeded: bool,
    /// M4: tools visible to the LLM this turn, filtered by
    /// `always_on ∪ active_skills`. Set by `pre_loop`; read by
    /// `model_call`. Empty when M4 deps are not configured.
    pub effective_tools: Vec<ToolSpec>,
    /// M4: system prompt blocks (core + append), built by `pre_loop`.
    /// Read by `model_call` to populate `ChatRequest::system`.
    pub system_blocks: SystemBlocks,
    /// M4: ephemeral reminder text (rendered as a `User` message with
    /// `<system-reminder>` tags). Set by `pre_loop`.
    pub ephemeral_text: String,
    /// M5: latest LLM-generated summary, if the compactor emitted one this
    /// session. Fed to `summarize_recent` on the next compaction pass so the
    /// LLM can produce an incremental update instead of re-summarizing the
    /// whole conversation from scratch.
    pub compaction_summary: Option<String>,
    /// v1.1.0 Phase 6 P0:本轮 pre_loop 收集到的"上下文恢复"元消息
    /// (active files / subagent registry / session memory notes)。
    /// `model_call` 在 ephemeral 推送之后再渲染成 `<system-reminder>`
    /// User 块,确保 LLM 看到的关键事实跨 turn / 跨 compact 一致。
    /// 每轮从 `M4Deps` 的共享源(`NoteStore` / `SubagentRegistry` /
    /// `ActiveFileRecovery`)重新计算,所以本字段不需跨 turn 持久化。
    pub recovery_meta: Vec<RecoveryEntry>,
}

/// Outcome of a `StateGraph::run` invocation.
#[derive(Debug)]
pub enum TurnOutcome {
    /// Turn finished normally; `TurnComplete` should be emitted.
    Success,
    /// Turn was aborted (cancel / interrupt / max-iterations).
    Aborted,
    /// Turn ended with an LLM or tool error; `Error` event already emitted.
    Error(String),
}

impl AgentState {
    pub fn has_tool_calls(&self) -> bool {
        self.latest_content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
    }
}
