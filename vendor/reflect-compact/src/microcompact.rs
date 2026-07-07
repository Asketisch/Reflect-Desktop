//! Microcompact: local heuristic, no LLM.
//!
//! Pin `System` + first `User`; for each "old" message (anything not in
//! the keep_recent window, not pinned, and not a preserve tool), replace
//! the content with a placeholder. Strip thinking blocks (Anthropic
//! server already has them). Returns `(messages, was_compacted)`.
//!
//! Mirrors reflect `graph.py:microcompact_messages` (lines 330-439).

use reflect_llm::{AssistantContent, ChatMessage, ToolResult};

/// Tools whose results are NEVER replaced (their content is essential
/// for the next model call). Matches reflect `_PRESERVE_TOOL_NAMES`,
/// 加上 v1.1.0 task 系统:`TaskUpdate` 返回的 `updatedFields` /
/// `statusChange` 是后续 turn 决策的关键信号,`TaskCreate` 返回的 task id
/// 是后续 TaskGet/TaskUpdate 的入口 —— 二者都不能被 microcompact 抹掉。
pub const PRESERVE_TOOL_NAMES: &[&str] = &[
    "write",
    "replace",
    "edit",
    "todo",
    "TaskCreate",
    "TaskUpdate",
];

/// Tools whose results ARE replaced with a placeholder.
pub const TRUNCATABLE_TOOL_NAMES: &[&str] = &[
    "read",
    "grep",
    "glob",
    "bash",
    "load_skill",
    "web_fetch",
    "web_search",
];

/// Default fraction of `trigger_tokens` below which microcompact is a
/// no-op. Matches reflect `microcompact_trigger_ratio = 0.7`.
pub const MICROCOMPACT_TRIGGER_RATIO: f32 = 0.7;

/// Default `keep_recent` window for microcompact. M5 v0: aligned with claw
/// (`4`) for tight per-turn context; the prior M4 default was 30 (reflect).
pub const KEEP_RECENT_DEFAULT: usize = 4;

/// Placeholder inserted in place of truncatable tool results.
pub const TRUNCATE_PLACEHOLDER: &str = "[工具结果已清除]";

/// Placeholder inserted for `load_skill` results.
pub const SKILL_PLACEHOLDER: &str = "[技能内容已清除。可调用 load_skill(\"name\") 重新加载]";

/// Outcome of a microcompact call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactReport {
    /// Token estimate before compacting.
    pub before_tokens: u32,
    /// Token estimate after compacting.
    pub after_tokens: u32,
    /// Number of messages whose body was replaced.
    pub removed_count: usize,
    /// True if the strategy actually modified the message list.
    pub was_compacted: bool,
}

/// Tunable parameters for microcompact.
#[derive(Debug, Clone)]
pub struct MicrocompactConfig {
    /// Tokens below this threshold trigger compaction.
    pub trigger_tokens: u32,
    /// Number of recent messages to keep verbatim.
    pub keep_recent: usize,
    /// Skip compaction if total tokens < trigger * ratio.
    pub trigger_ratio: f32,
}

impl Default for MicrocompactConfig {
    fn default() -> Self {
        Self {
            trigger_tokens: crate::strategy::DEFAULT_TRIGGER_TOKENS,
            keep_recent: KEEP_RECENT_DEFAULT,
            trigger_ratio: MICROCOMPACT_TRIGGER_RATIO,
        }
    }
}

/// Run microcompact. Returns the new message list and a report.
///
/// If `crate::estimate_messages(&messages) * ratio < trigger_tokens`
/// the input is returned unchanged (`was_compacted = false`).
pub fn microcompact(
    messages: Vec<ChatMessage>,
    cfg: &MicrocompactConfig,
) -> (Vec<ChatMessage>, CompactReport) {
    let before = crate::tokens::estimate_messages(&messages);
    let threshold = ((cfg.trigger_tokens as f32) * cfg.trigger_ratio) as u32;
    if before < threshold {
        return (
            messages,
            CompactReport {
                before_tokens: before,
                after_tokens: before,
                removed_count: 0,
                was_compacted: false,
            },
        );
    }

    let n = messages.len();
    if n <= cfg.keep_recent {
        return (
            messages,
            CompactReport {
                before_tokens: before,
                after_tokens: before,
                removed_count: 0,
                was_compacted: false,
            },
        );
    }

    // Identify pinned indices: every System + the first User.
    let first_user = messages
        .iter()
        .position(|m| matches!(m, ChatMessage::User(_)))
        .unwrap_or(usize::MAX);
    let mut pinned: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (i, m) in messages.iter().enumerate() {
        if matches!(m, ChatMessage::System(_)) || i == first_user {
            pinned.insert(i);
        }
    }
    // Anything in the keep_recent tail is also exempt.
    let keep_start = n.saturating_sub(cfg.keep_recent);
    for i in keep_start..n {
        pinned.insert(i);
    }
    // M5: expand the keep window to preserve every tool_use ↔ tool_result
    // pair so we never emit a half-pair to the provider.
    let expanded = crate::tool_pair::expand_keep_window_to_preserve_pairs(&messages, keep_start);
    for i in expanded..keep_start {
        pinned.insert(i);
    }

    let mut removed_count = 0;
    let mut new_messages = Vec::with_capacity(n);
    let call_id_map = crate::tool_pair::build_call_id_to_tool_name_map(&messages);
    for (i, m) in messages.into_iter().enumerate() {
        if pinned.contains(&i) {
            new_messages.push(m);
            continue;
        }
        let (new_m, was_replaced) = replace_compactable(m, &call_id_map);
        if was_replaced {
            removed_count += 1;
        }
        new_messages.push(new_m);
    }

    let after = crate::tokens::estimate_messages(&new_messages);
    (
        new_messages,
        CompactReport {
            before_tokens: before,
            after_tokens: after,
            removed_count,
            was_compacted: removed_count > 0,
        },
    )
}

/// If the message is a `Tool` whose tool name is truncatable, replace
/// its content. Returns the (possibly replaced) message and a `bool`
/// indicating whether the body was actually changed.
fn replace_compactable(
    msg: ChatMessage,
    call_id_map: &std::collections::HashMap<String, String>,
) -> (ChatMessage, bool) {
    match msg {
        ChatMessage::Tool(t) => {
            // Look up the tool name from the Assistant tool_calls that
            // produced this result, then check PRESERVE_TOOL_NAMES.
            let tool_name = call_id_map
                .get(&t.call_id)
                .map(String::as_str)
                .unwrap_or("");
            if PRESERVE_TOOL_NAMES.contains(&tool_name) {
                (ChatMessage::Tool(t), false)
            } else {
                (
                    ChatMessage::Tool(ToolResult {
                        call_id: t.call_id,
                        content: TRUNCATE_PLACEHOLDER.into(),
                        is_error: t.is_error,
                    }),
                    true,
                )
            }
        }
        ChatMessage::Assistant(a) => {
            // Strip thinking blocks; keep text + tool_calls.
            let stripped = AssistantContent {
                text: a.text,
                tool_calls: a.tool_calls,
                thinking: None,
            };
            (ChatMessage::Assistant(stripped), a.thinking.is_some())
        }
        other => (other, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_llm::ToolCallRequest;
    use reflect_llm::{AssistantContent, ContentBlock, ToolResult, UserContent};

    fn make_tool_result(call_id: &str, content: &str) -> ChatMessage {
        ChatMessage::Tool(ToolResult {
            call_id: call_id.into(),
            content: content.into(),
            is_error: false,
        })
    }

    fn make_assistant_with_tool_call(call_id: &str, tool_name: &str) -> ChatMessage {
        ChatMessage::Assistant(AssistantContent {
            text: Some("ok".into()),
            tool_calls: vec![ToolCallRequest {
                id: call_id.into(),
                name: tool_name.into(),
                arguments: serde_json::Value::Null,
            }],
            thinking: None,
        })
    }

    fn make_assistant_with_thinking() -> ChatMessage {
        ChatMessage::Assistant(AssistantContent {
            text: Some("ok".into()),
            tool_calls: vec![],
            thinking: Some("internal".into()),
        })
    }

    #[test]
    fn below_threshold_noop() {
        let msgs = vec![ChatMessage::System("hi".into())];
        let (_, r) = microcompact(msgs.clone(), &MicrocompactConfig::default());
        assert!(!r.was_compacted);
        assert_eq!(r.before_tokens, r.after_tokens);
    }

    #[test]
    fn pinned_system_kept_verbatim() {
        // Layout:
        //   index 0: System                → pinned (System)
        //   index 1: Assistant (tool_call write)→ builds call_id→name map
        //   index 2: Tool c1_write (write)     → preserved via map
        //   index 3: Tool c2 (truncatable)     → in keep_recent tail (last 1)
        //
        // After compaction: System unchanged, c1 preserved, c2 in
        // keep_recent tail (so unchanged). No replacement happens.
        let msgs = vec![
            ChatMessage::System("core system prompt".into()),
            make_assistant_with_tool_call("c1_write", "write"),
            make_tool_result("c1_write", "important file content"),
            make_tool_result("c2", "bash output here"),
        ];
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 1,
            ..Default::default()
        };
        let (out, r) = microcompact(msgs, &cfg);
        // No replacement → not compacted.
        assert!(!r.was_compacted);
        assert_eq!(r.removed_count, 0);
        match &out[0] {
            ChatMessage::System(s) => assert_eq!(s, "core system prompt"),
            _ => panic!("expected System at 0"),
        }
        match &out[2] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "important file content"),
            _ => panic!("expected Tool at 2"),
        }
        match &out[3] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "bash output here"),
            _ => panic!("expected Tool at 3"),
        }
    }

    #[test]
    fn truncatable_tool_replaced_when_not_pinned() {
        // 6 messages, keep_recent=2 → tail (indices 4,5) is exempt.
        // Index 2 (write, preserved via map) is not replaced. Index 3 (c2,
        // truncatable, not in keep_recent) IS replaced.
        let msgs = vec![
            ChatMessage::System("sys".into()),
            make_assistant_with_tool_call("c1_write", "write"),
            make_tool_result("c1_write", "important"),
            make_tool_result("c2", "long bash output data here"),
            make_tool_result("c3", "more data"),
            make_tool_result("c4", "tail data"),
        ];
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 2,
            ..Default::default()
        };
        let (out, r) = microcompact(msgs, &cfg);
        assert!(r.was_compacted);
        assert_eq!(r.removed_count, 1);
        // System at 0.
        assert!(matches!(out[0], ChatMessage::System(_)));
        // Preserved write result at 2.
        match &out[2] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "important"),
            _ => panic!("expected Tool at 2"),
        }
        // c2 at 3 → replaced with placeholder.
        match &out[3] {
            ChatMessage::Tool(t) => assert_eq!(t.content, TRUNCATE_PLACEHOLDER),
            _ => panic!("expected Tool at 3"),
        }
        // c3, c4 in keep_recent tail → unchanged.
        match &out[4] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "more data"),
            _ => panic!("expected Tool at 4"),
        }
        match &out[5] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "tail data"),
            _ => panic!("expected Tool at 5"),
        }
    }

    #[test]
    fn truncatable_tool_replaced_keep_recent_1() {
        // 5 messages, keep_recent=1 → only index 4 is in keep_recent.
        // Index 1 (tool) is truncatable → replaced.
        let msgs = vec![
            ChatMessage::System("sys".into()),
            make_tool_result("c1", "data here"),
            make_tool_result("c2", "more data"),
            make_tool_result("c3", "even more"),
            make_tool_result("c4", "tail"),
        ];
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 1,
            ..Default::default()
        };
        let (out, r) = microcompact(msgs, &cfg);
        assert!(r.was_compacted);
        // c1 at index 1 should be truncated.
        match &out[1] {
            ChatMessage::Tool(t) => assert_eq!(t.content, TRUNCATE_PLACEHOLDER),
            _ => panic!("expected Tool at 1"),
        }
        // c4 at index 4 is in keep_recent tail → unchanged.
        match &out[4] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "tail"),
            _ => panic!("expected Tool at 4"),
        }
    }

    #[test]
    fn first_user_pinned() {
        let msgs = vec![
            ChatMessage::User(UserContent {
                blocks: vec![ContentBlock::text("first user message")],
            }),
            make_tool_result("c1", "data"),
            make_tool_result("c2", "more"),
        ];
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 1,
            ..Default::default()
        };
        let (out, r) = microcompact(msgs, &cfg);
        assert!(r.was_compacted);
        // Index 0 (first User) pinned.
        match &out[0] {
            ChatMessage::User(_) => {}
            _ => panic!("expected User at 0"),
        }
        // Index 1 in keep_recent tail (n-1..n) — also kept.
        match &out[2] {
            ChatMessage::Tool(t) => assert_eq!(t.content, "more"),
            _ => panic!("expected Tool at 2"),
        }
    }

    #[test]
    fn keep_recent_window_protects_tail() {
        let mut msgs = vec![ChatMessage::System("sys".into())];
        for i in 0..20 {
            msgs.push(make_tool_result(&format!("c{i}"), &format!("data {i}")));
        }
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 5,
            ..Default::default()
        };
        let (out, r) = microcompact(msgs, &cfg);
        assert!(r.was_compacted);
        // The last 5 tool results should be unchanged.
        for (i, slot) in out.iter().enumerate().take(21).skip(16) {
            match slot {
                ChatMessage::Tool(t) => assert!(t.content.starts_with("data ")),
                _ => panic!("expected Tool at {i}"),
            }
        }
    }

    #[test]
    fn thinking_blocks_stripped() {
        let msgs = vec![
            ChatMessage::System("sys".into()),
            make_assistant_with_thinking(),
            make_tool_result("c1", "x"),
        ];
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 1,
            ..Default::default()
        };
        let (out, r) = microcompact(msgs, &cfg);
        assert!(r.was_compacted);
        match &out[1] {
            ChatMessage::Assistant(a) => {
                assert!(a.thinking.is_none(), "thinking should be stripped");
                assert_eq!(a.text.as_deref(), Some("ok"));
            }
            _ => panic!("expected Assistant at 1"),
        }
    }

    #[test]
    fn no_compact_when_messages_shorter_than_keep_recent() {
        let msgs = vec![
            ChatMessage::System("sys".into()),
            make_tool_result("c1", "x"),
        ];
        let cfg = MicrocompactConfig {
            trigger_tokens: 1,
            keep_recent: 100,
            ..Default::default()
        };
        let (_, r) = microcompact(msgs, &cfg);
        assert!(!r.was_compacted);
    }
}
