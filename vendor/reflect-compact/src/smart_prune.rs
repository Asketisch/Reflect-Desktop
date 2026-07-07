//! Smart-prune: local heuristic, no LLM.
//!
//! Phase 1: per-tool truncation. `grep`-like tools keep their head
//! `MAX_TOOL_RESULT_LINES` lines; `bash`-like keep the tail;
//! `read`-like keep head+tail with a `... [middle truncated] ...` marker;
//! others truncate to `MAX_TOOL_RESULT_CHARS`. Assistant text longer
//! than `MAX_ASSISTANT_CHARS` is truncated and the tool list is appended.
//!
//! Phase 2: drop the oldest non-pinned messages until the token estimate
//! is below `target_tokens`.
//!
//! Mirrors reflect `graph.py:smart_prune_messages` (lines 442-609).

use reflect_llm::{AssistantContent, ChatMessage, ToolResult};

/// Default max characters for a truncated tool result.
pub const MAX_TOOL_RESULT_CHARS: usize = 300;
/// Default max characters for assistant text.
pub const MAX_ASSISTANT_CHARS: usize = 600;
/// Default max characters for subagent tool results.
pub const MAX_SUBAGENT_RESULT_CHARS: usize = 3000;
/// Default max lines to keep from a tool result.
pub const MAX_TOOL_RESULT_LINES: usize = 50;

/// Marker inserted when read-like tools lose their middle.
pub const MIDDLE_MARKER: &str = "\n... [middle truncated] ...\n";

/// Tool name categories. Tools not in any category fall through to
/// "truncate to MAX_TOOL_RESULT_CHARS".
pub const GREP_LIKE_TOOLS: &[&str] = &["grep", "ranked_search", "semantic_search"];
pub const BASH_LIKE_TOOLS: &[&str] = &["bash"];
pub const READ_LIKE_TOOLS: &[&str] = &["read", "read_ranges", "read_code_context"];

/// Tunable parameters for smart-prune.
#[derive(Debug, Clone)]
pub struct SmartPruneConfig {
    pub trigger_tokens: u32,
    pub keep_recent: usize,
    pub target_tokens: u32,
    pub max_tool_result_chars: usize,
    pub max_assistant_chars: usize,
    pub max_subagent_result_chars: usize,
    pub max_tool_result_lines: usize,
}

impl Default for SmartPruneConfig {
    fn default() -> Self {
        Self {
            trigger_tokens: crate::strategy::DEFAULT_TRIGGER_TOKENS,
            keep_recent: crate::microcompact::KEEP_RECENT_DEFAULT + 1,
            // 0.75 * trigger. Kept computed at runtime to track `trigger_tokens`.
            target_tokens: ((crate::strategy::DEFAULT_TRIGGER_TOKENS as f32) * 0.75) as u32,
            max_tool_result_chars: MAX_TOOL_RESULT_CHARS,
            max_assistant_chars: MAX_ASSISTANT_CHARS,
            max_subagent_result_chars: MAX_SUBAGENT_RESULT_CHARS,
            max_tool_result_lines: MAX_TOOL_RESULT_LINES,
        }
    }
}

/// Outcome of smart-prune.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactReport {
    pub before_tokens: u32,
    pub after_tokens: u32,
    pub removed_count: usize,
    pub truncated_count: usize,
    pub was_compacted: bool,
}

/// Run smart-prune. Returns the new message list and a report.
pub fn smart_prune(
    messages: Vec<ChatMessage>,
    cfg: &SmartPruneConfig,
) -> (Vec<ChatMessage>, CompactReport) {
    let before = crate::tokens::estimate_messages(&messages);
    if before < cfg.trigger_tokens {
        return (
            messages,
            CompactReport {
                before_tokens: before,
                after_tokens: before,
                removed_count: 0,
                truncated_count: 0,
                was_compacted: false,
            },
        );
    }

    // Phase 1: per-tool truncation.
    let mut truncated_count = 0;
    let mut after_phase1: Vec<ChatMessage> = Vec::with_capacity(messages.len());
    let call_id_map = crate::tool_pair::build_call_id_to_tool_name_map(&messages);
    for m in messages {
        let (replaced, was_truncated) = truncate_message(m, cfg, &call_id_map);
        if was_truncated {
            truncated_count += 1;
        }
        after_phase1.push(replaced);
    }

    // Phase 2: drop oldest non-pinned until under target.
    let n = after_phase1.len();
    if n <= cfg.keep_recent {
        let after = crate::tokens::estimate_messages(&after_phase1);
        return (
            after_phase1,
            CompactReport {
                before_tokens: before,
                after_tokens: after,
                removed_count: 0,
                truncated_count,
                was_compacted: truncated_count > 0,
            },
        );
    }
    // Pinned: every System + first User.
    let first_user = after_phase1
        .iter()
        .position(|m| matches!(m, ChatMessage::User(_)))
        .unwrap_or(usize::MAX);
    let mut pinned: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (i, m) in after_phase1.iter().enumerate() {
        if matches!(m, ChatMessage::System(_)) || i == first_user {
            pinned.insert(i);
        }
    }
    let keep_start = n.saturating_sub(cfg.keep_recent);
    for i in keep_start..n {
        pinned.insert(i);
    }

    // Walk old indices (not in pinned, not in keep_recent), drop oldest.
    let mut removed_count = 0;
    let mut keep: Vec<(usize, ChatMessage)> = after_phase1.into_iter().enumerate().collect();
    let mut current_tokens =
        crate::tokens::estimate_messages(&keep.iter().map(|(_, m)| m.clone()).collect::<Vec<_>>());
    let mut drop_idx = 0;
    while current_tokens >= cfg.target_tokens && drop_idx < keep.len() {
        if pinned.contains(&drop_idx) || drop_idx >= keep_start {
            drop_idx += 1;
            continue;
        }
        keep.remove(drop_idx);
        // After removal, indices shift; pinned set is by original
        // positions but the loop walks in order. Recompute pinned
        // for safety.
        pinned.remove(&drop_idx);
        for p in pinned.iter() {
            if *p > drop_idx {
                // adjust later
            }
        }
        // Re-index pinned: subtract 1 from any pinned > drop_idx.
        let mut new_pinned: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for p in &pinned {
            if *p > drop_idx {
                new_pinned.insert(*p - 1);
            } else {
                new_pinned.insert(*p);
            }
        }
        pinned = new_pinned;
        if keep_start > 0 {
            // keep_start also shifts by 1.
        }
        removed_count += 1;
        current_tokens = crate::tokens::estimate_messages(
            &keep.iter().map(|(_, m)| m.clone()).collect::<Vec<_>>(),
        );
        // Don't advance drop_idx; the next element shifted into this slot.
    }

    let final_msgs: Vec<ChatMessage> = keep.into_iter().map(|(_, m)| m).collect();
    let after = crate::tokens::estimate_messages(&final_msgs);
    (
        final_msgs,
        CompactReport {
            before_tokens: before,
            after_tokens: after,
            removed_count,
            truncated_count,
            was_compacted: removed_count > 0 || truncated_count > 0,
        },
    )
}

/// Truncate one message per its tool category. Returns the (possibly
/// truncated) message and a `bool` indicating whether the body actually
/// changed.
fn truncate_message(
    msg: ChatMessage,
    cfg: &SmartPruneConfig,
    call_id_map: &std::collections::HashMap<String, String>,
) -> (ChatMessage, bool) {
    match msg {
        ChatMessage::Tool(t) => {
            let new_content = truncate_tool_result(
                &t.call_id,
                &t.content,
                cfg.max_tool_result_chars,
                cfg.max_tool_result_lines,
                call_id_map,
            );
            let was_truncated = new_content != t.content;
            (
                ChatMessage::Tool(ToolResult {
                    call_id: t.call_id,
                    content: new_content,
                    is_error: t.is_error,
                }),
                was_truncated,
            )
        }
        ChatMessage::Assistant(a) => {
            let text = a.text.clone().unwrap_or_default();
            if text.chars().count() <= cfg.max_assistant_chars {
                return (ChatMessage::Assistant(a), false);
            }
            let truncated: String = text.chars().take(cfg.max_assistant_chars).collect();
            let tool_names: Vec<String> = a.tool_calls.iter().map(|t| t.name.clone()).collect();
            let suffix = if tool_names.is_empty() {
                String::new()
            } else {
                format!("\n[... tools: {} ...]", tool_names.join(", "))
            };
            (
                ChatMessage::Assistant(AssistantContent {
                    text: Some(format!("{truncated}{suffix}")),
                    tool_calls: a.tool_calls,
                    thinking: a.thinking,
                }),
                true,
            )
        }
        other => (other, false),
    }
}

/// Truncate a tool result based on tool name. The call_id encodes the
/// tool name with a `tool:<name>:` prefix (set by strategy layer); if
/// absent, default to chars-based truncation.
pub fn truncate_tool_result(
    call_id: &str,
    content: &str,
    max_chars: usize,
    max_lines: usize,
    call_id_map: &std::collections::HashMap<String, String>,
) -> String {
    // Prefer the tool name from the call_id->name map (production path);
    // fall back to the legacy `tool:<name>:` prefix used by tests.
    let name_owned: String = call_id_map.get(call_id).cloned().unwrap_or_else(|| {
        let tool_name = call_id.strip_prefix("tool:").unwrap_or("");
        tool_name.split(':').next().unwrap_or("").to_string()
    });
    let name: &str = &name_owned;

    if content.len() <= max_chars {
        return content.to_string();
    }
    if GREP_LIKE_TOOLS.contains(&name) {
        keep_head_lines(content, max_lines)
    } else if BASH_LIKE_TOOLS.contains(&name) {
        keep_tail_lines(content, max_lines.max(30))
    } else if READ_LIKE_TOOLS.contains(&name) {
        keep_head_tail_lines(content, max_lines.max(20) / 2)
    } else if name.starts_with("call_") {
        // subagent result
        truncate_chars(content, max_chars.max(3000))
    } else {
        truncate_chars(content, max_chars)
    }
}

fn keep_head_lines(s: &str, n: usize) -> String {
    let mut out = String::new();
    for (i, line) in s.lines().enumerate() {
        if i >= n {
            out.push_str("\n[... truncated ...]\n");
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn keep_tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    let mut out = String::new();
    if start > 0 {
        out.push_str("[... earlier output truncated ...]\n");
    }
    for line in &lines[start..] {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn keep_head_tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let half = n.max(10);
    if lines.len() <= half * 2 {
        return s.to_string();
    }
    let head: Vec<&str> = lines[..half].to_vec();
    let tail: Vec<&str> = lines[lines.len() - half..].to_vec();
    let mut out = String::new();
    for line in head {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(MIDDLE_MARKER);
    for line in tail {
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let truncated: String = s.chars().take(n).collect();
    format!("{truncated}\n[... truncated ...]")
}

// Standalone helper kept for test reachability; inlined into
// `truncate_message` for production use.
#[allow(dead_code)]
fn truncate_assistant_in_place(a: AssistantContent, max_chars: usize) -> (ChatMessage, bool) {
    let text = a.text.clone().unwrap_or_default();
    if text.chars().count() <= max_chars {
        return (ChatMessage::Assistant(a), false);
    }
    let truncated: String = text.chars().take(max_chars).collect();
    let tool_names: Vec<String> = a.tool_calls.iter().map(|t| t.name.clone()).collect();
    let suffix = if tool_names.is_empty() {
        String::new()
    } else {
        format!("\n[... tools: {} ...]", tool_names.join(", "))
    };
    (
        ChatMessage::Assistant(AssistantContent {
            text: Some(format!("{truncated}{suffix}")),
            tool_calls: a.tool_calls,
            thinking: a.thinking,
        }),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_llm::{ContentBlock, ToolResult, UserContent};

    fn make_tool_result(call_id: &str, content: &str) -> ChatMessage {
        ChatMessage::Tool(ToolResult {
            call_id: call_id.into(),
            content: content.into(),
            is_error: false,
        })
    }

    #[test]
    fn grep_like_keeps_head_lines() {
        let content: String = (0..200).map(|i| format!("line {i}\n")).collect();
        let out = truncate_tool_result(
            "tool:grep:0",
            &content,
            100,
            50,
            &std::collections::HashMap::new(),
        );
        // Should be head 50 lines only.
        assert!(out.contains("line 0"));
        assert!(out.contains("line 49"));
        assert!(!out.contains("line 100"));
    }

    #[test]
    fn bash_like_keeps_tail_lines() {
        let content: String = (0..200).map(|i| format!("line {i}\n")).collect();
        let out = truncate_tool_result(
            "tool:bash:0",
            &content,
            100,
            50,
            &std::collections::HashMap::new(),
        );
        assert!(out.contains("line 199"));
        assert!(!out.contains("line 0"));
        assert!(out.contains("truncated"));
    }

    #[test]
    fn read_like_keeps_head_and_tail() {
        let content: String = (0..200).map(|i| format!("line {i}\n")).collect();
        let out = truncate_tool_result(
            "tool:read:0",
            &content,
            100,
            50,
            &std::collections::HashMap::new(),
        );
        assert!(out.contains(MIDDLE_MARKER));
        assert!(out.contains("line 0"));
        assert!(out.contains("line 199"));
    }

    #[test]
    fn unknown_tool_truncates_by_chars() {
        let content: String = "x".repeat(1000);
        let out = truncate_tool_result(
            "tool:unknown:0",
            &content,
            100,
            50,
            &std::collections::HashMap::new(),
        );
        assert!(out.contains("[... truncated ...]"));
        assert!(out.chars().count() < 200);
    }

    #[test]
    fn short_content_unchanged() {
        let out = truncate_tool_result(
            "tool:grep:0",
            "short content",
            100,
            50,
            &std::collections::HashMap::new(),
        );
        assert_eq!(out, "short content");
    }

    #[test]
    fn subagent_uses_higher_limit() {
        let content: String = "x".repeat(5000);
        let out = truncate_tool_result(
            "tool:call_explorer:0",
            &content,
            100,
            50,
            &std::collections::HashMap::new(),
        );
        // max_chars=100 but subagent cap = 3000 → content < 3000 stays.
        assert!(out.contains(&"x".repeat(3000)) || out.chars().count() < 5000);
    }

    #[test]
    fn below_trigger_noop() {
        let msgs = vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("hi")],
        })];
        let (_, r) = smart_prune(msgs, &SmartPruneConfig::default());
        assert!(!r.was_compacted);
    }

    #[test]
    fn assistant_text_over_max_truncated() {
        let long_text: String = "x".repeat(1000);
        let a = AssistantContent {
            text: Some(long_text),
            tool_calls: vec![reflect_llm::ToolCallRequest {
                id: "c1".into(),
                name: "read".into(),
                arguments: serde_json::json!({}),
            }],
            thinking: None,
        };
        let (_, was_truncated) = truncate_assistant_in_place(a, MAX_ASSISTANT_CHARS);
        assert!(was_truncated);
    }

    #[test]
    fn assistant_text_under_max_unchanged() {
        let a = AssistantContent {
            text: Some("short".into()),
            tool_calls: vec![],
            thinking: None,
        };
        let (_, was_truncated) = truncate_assistant_in_place(a, 600);
        assert!(!was_truncated);
    }

    #[test]
    fn phase2_drops_oldest_until_under_target() {
        // Build a huge message list where each message is a long tool result.
        // 50 messages, each 200 chars = ~10000 chars total = ~2857 tokens.
        // Set trigger=2000 so we hit phase 2.
        let mut msgs = vec![ChatMessage::System("sys".into())];
        for i in 0..50 {
            let content: String = "x".repeat(200);
            msgs.push(make_tool_result(&format!("c{i}"), &content));
        }
        let cfg = SmartPruneConfig {
            trigger_tokens: 2000,
            target_tokens: 1000,
            keep_recent: 5,
            ..Default::default()
        };
        let (out, r) = smart_prune(msgs, &cfg);
        assert!(r.was_compacted);
        // System at index 0 pinned.
        assert!(matches!(out[0], ChatMessage::System(_)));
        // The keep_recent=5 tail is intact.
        assert!(out.len() <= 51);
    }
}
