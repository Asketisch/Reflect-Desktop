//! Tool-pair preservation helpers.
//!
//! Both microcompact and smart_prune can drop or truncate individual
//! messages, but a `tool_use` block in an `Assistant` message and its
//! corresponding `ChatMessage::Tool` (matched by `call_id`) must always
//! travel together — OpenAI / Anthropic reject half-pairs. This module
//! exposes `expand_keep_window_to_preserve_pairs` so the strategy layer
//! can pre-compute an expanded `keep_start` that protects every pair.

use std::collections::HashMap;

use reflect_llm::ChatMessage;

/// Index every `tool_use` id emitted by an `Assistant` message back to the
/// message's position in the list.
pub fn find_tool_use_indices(msgs: &[ChatMessage]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for (i, m) in msgs.iter().enumerate() {
        if let ChatMessage::Assistant(a) = m {
            for tc in &a.tool_calls {
                out.insert(tc.id.clone(), i);
            }
        }
    }
    out
}

/// Index every `ChatMessage::Tool`'s `call_id` back to its position.
pub fn find_tool_result_indices(msgs: &[ChatMessage]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for (i, m) in msgs.iter().enumerate() {
        if let ChatMessage::Tool(t) = m {
            out.insert(t.call_id.clone(), i);
        }
    }
    out
}

/// Build a `call_id → tool_name` map by scanning every `Assistant`
/// message's `tool_calls`.
///
/// This lets microcompact / smart_prune know *which* tool produced a
/// given `Tool` result without relying on a `tool:<name>:` prefix in
/// `call_id` (which is never applied in the production path).
pub fn build_call_id_to_tool_name_map(msgs: &[ChatMessage]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for m in msgs {
        if let ChatMessage::Assistant(a) = m {
            for tc in &a.tool_calls {
                out.insert(tc.id.clone(), tc.name.clone());
            }
        }
    }
    out
}

/// Given a candidate `keep_start` (so messages `keep_start..n` are kept
/// verbatim), expand it to also include any message whose `tool_use`
/// partner lies in the keep window but it does not (or vice versa).
///
/// Returns the new (possibly smaller) `keep_start`.
pub fn expand_keep_window_to_preserve_pairs(msgs: &[ChatMessage], keep_start: usize) -> usize {
    let tool_use_idx = find_tool_use_indices(msgs);
    let tool_result_idx = find_tool_result_indices(msgs);
    let mut new_keep_start = keep_start;

    // Walk the keep window. For every Tool result, ensure its Assistant is
    // also in the window (if not, expand). For every Assistant with a
    // tool_use, ensure its Tool is also in the window.
    for (_i, m) in msgs.iter().enumerate().skip(keep_start) {
        match m {
            ChatMessage::Tool(t) => {
                if let Some(&use_idx) = tool_use_idx.get(&t.call_id)
                    && use_idx < new_keep_start
                {
                    new_keep_start = use_idx;
                }
            }
            ChatMessage::Assistant(a) => {
                for tc in &a.tool_calls {
                    if let Some(&res_idx) = tool_result_idx.get(&tc.id)
                        && res_idx < new_keep_start
                    {
                        new_keep_start = res_idx;
                    }
                }
            }
            _ => {}
        }
    }
    new_keep_start
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_llm::{AssistantContent, ToolCallRequest, ToolResult, UserContent};

    fn assistant_with_tools(ids: &[&str]) -> ChatMessage {
        ChatMessage::Assistant(AssistantContent {
            text: None,
            tool_calls: ids
                .iter()
                .map(|id| ToolCallRequest {
                    id: id.to_string(),
                    name: "bash".into(),
                    arguments: serde_json::json!({}),
                })
                .collect(),
            thinking: None,
        })
    }

    fn tool_result(call_id: &str) -> ChatMessage {
        ChatMessage::Tool(ToolResult {
            call_id: call_id.into(),
            content: "out".into(),
            is_error: false,
        })
    }

    #[test]
    fn pair_inside_window_no_expansion() {
        let msgs = vec![
            ChatMessage::User(UserContent { blocks: vec![] }),
            assistant_with_tools(&["c1"]),
            tool_result("c1"),
            ChatMessage::User(UserContent { blocks: vec![] }),
            assistant_with_tools(&["c2"]),
            tool_result("c2"),
        ];
        // keep_start = 3 → window contains indices 3..6 which includes the
        // c2 Assistant + Tool. No expansion needed.
        let expanded = expand_keep_window_to_preserve_pairs(&msgs, 3);
        assert_eq!(expanded, 3);
    }

    #[test]
    fn pair_straddles_window_expand_backwards() {
        // Indices: 0=User, 1=A(c1), 2=T(c1), 3=A(c2), 4=T(c2), 5=User
        // keep_start=4 → window is [A(c2), T(c2)] but A(c2) is at 3, so we
        // need to expand back to 3 (and the T(c2) partner check from A(c2)
        // pulls index 4 in, but it's already in the window).
        let msgs = vec![
            ChatMessage::User(UserContent { blocks: vec![] }),
            assistant_with_tools(&["c1"]),
            tool_result("c1"),
            assistant_with_tools(&["c2"]),
            tool_result("c2"),
            ChatMessage::User(UserContent { blocks: vec![] }),
        ];
        let expanded = expand_keep_window_to_preserve_pairs(&msgs, 4);
        assert!(
            expanded <= 3,
            "should expand to include A(c2), got {expanded}"
        );
    }

    #[test]
    fn tool_result_in_window_assistant_outside_expands() {
        // Index 2 = T(c1) but the Assistant(c1) at index 1 is outside the
        // keep window → must expand back to 1.
        let msgs = vec![
            assistant_with_tools(&["c1"]),                     // 0
            ChatMessage::User(UserContent { blocks: vec![] }), // 1
            tool_result("c1"),                                 // 2
        ];
        let expanded = expand_keep_window_to_preserve_pairs(&msgs, 2);
        assert_eq!(expanded, 0, "must pull Assistant(c1) back into window");
    }

    #[test]
    fn tool_result_outside_window_assistant_in_window_expands() {
        // Index 1 = A(c1); T(c1) at index 2 is in window but the check goes
        // both ways: A(c1)'s tool call id "c1" finds T at 2 which is in the
        // window, so no expansion needed.
        // Edge case: A(c1) at 1 with T(c1) at 0 — T outside, A inside.
        let msgs = vec![
            tool_result("c1"),                                 // 0
            assistant_with_tools(&["c1"]),                     // 1
            ChatMessage::User(UserContent { blocks: vec![] }), // 2
        ];
        // keep_start=1 → window is [A(c1), User]. A(c1) needs T(c1) which is
        // at index 0 → expand back to 0.
        let expanded = expand_keep_window_to_preserve_pairs(&msgs, 1);
        assert_eq!(expanded, 0);
    }
}
