//! How data flows between a parent and a subagent.
//!
//! - [`DataTransferConfig::pass_context_messages`] — number of trailing
//!   parent messages to copy into the child's initial message list.
//! - [`ResultExtractor`] — how the child's final answer is pulled out of
//!   the `TurnHandle` event stream and returned to the parent's tool.

use reflect_llm::ChatMessage;
use reflect_protocol::{ContentBlock, Event, EventMsg};

/// Configuration for parent↔child data flow on a single `spawn()` call.
#[derive(Debug, Clone)]
pub struct DataTransferConfig {
    /// Number of trailing messages from the parent's `state.messages` to
    /// prepend to the child's initial prompt. `0` means the child gets
    /// only its system prompt + the user input passed by the tool.
    pub pass_context_messages: usize,
    /// How the child's final answer is extracted.
    pub result_extractor: ResultExtractor,
}

impl Default for DataTransferConfig {
    fn default() -> Self {
        Self {
            pass_context_messages: 0,
            result_extractor: ResultExtractor::LastAssistantText,
        }
    }
}

/// Strategy for picking the child's reply out of its event stream.
#[derive(Debug, Clone)]
pub enum ResultExtractor {
    /// Take the text of the last `AgentMessage` (or concatenate all
    /// `AgentMessageDelta`s) before `TurnComplete`.
    LastAssistantText,
    /// Take the `content` of the `ToolResult` whose `call_id` matches
    /// `call_id`. Used when the child is expected to call a specific
    /// final tool (e.g. `finish_subagent`).
    LastToolResult(String),
}

/// Walk a `TurnHandle`'s events and extract the answer using the chosen
/// [`ResultExtractor`]. Returns `None` if the stream ended without a
/// matching event.
pub fn extract_result(events: &[Event], extractor: &ResultExtractor) -> Option<String> {
    match extractor {
        ResultExtractor::LastAssistantText => {
            let mut acc = String::new();
            for e in events {
                match &e.msg {
                    EventMsg::AgentMessageDelta(d) => acc.push_str(&d.delta),
                    EventMsg::AgentMessage(m) => return Some(m.text.clone()),
                    _ => {}
                }
            }
            if acc.is_empty() { None } else { Some(acc) }
        }
        ResultExtractor::LastToolResult(want_id) => {
            for e in events.iter().rev() {
                if let EventMsg::ToolCallEnd(end) = &e.msg {
                    if &end.call_id == want_id {
                        for c in &end.output.content {
                            if let ContentBlock::Text { text } = c {
                                return Some(text.clone());
                            }
                        }
                    }
                }
            }
            None
        }
    }
}

/// 子代理返回后附加 `[Coordinator Principle]` footer(父协调者 reminder)。
pub fn append_coordinator_principle_footer(result: &str, footer: Option<&str>) -> String {
    let Some(footer) = footer.filter(|f| !f.trim().is_empty()) else {
        return result.to_string();
    };
    format!("{result}\n\n[Coordinator Principle]\n{footer}")
}

/// Build the child's initial message list: copy the parent's trailing
/// `pass_context_messages` chat messages, then append the freshly
/// supplied user input.
pub fn build_child_initial_messages(
    parent_tail: &[ChatMessage],
    user_input: ChatMessage,
) -> Vec<ChatMessage> {
    let mut out: Vec<ChatMessage> = parent_tail.to_vec();
    out.push(user_input);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_protocol::{AgentMessage, Event, ToolOutput};

    fn evt_text(t: &str) -> Event {
        Event::new(
            "sub",
            EventMsg::AgentMessage(AgentMessage { text: t.into() }),
        )
    }

    fn evt_delta(d: &str) -> Event {
        Event::new(
            "sub",
            EventMsg::AgentMessageDelta(reflect_protocol::AgentMessageDelta { delta: d.into() }),
        )
    }

    fn evt_tool_result(call_id: &str, text: &str) -> Event {
        Event::new(
            "sub",
            EventMsg::ToolCallEnd(reflect_protocol::ToolCallEndEvent {
                call_id: call_id.into(),
                output: ToolOutput {
                    content: vec![ContentBlock::Text { text: text.into() }],
                    is_error: false,
                    metadata: serde_json::Value::Null,
                    elapsed_ms: 0,
                },
                is_error: false,
                elapsed_ms: 0,
            }),
        )
    }

    #[test]
    fn extract_last_assistant_text_from_delta_acc() {
        let events = vec![evt_delta("hel"), evt_delta("lo")];
        let got = extract_result(&events, &ResultExtractor::LastAssistantText);
        assert_eq!(got, Some("hello".into()));
    }

    #[test]
    fn extract_last_assistant_text_from_message_event() {
        let events = vec![evt_delta("hel"), evt_text("lo")];
        let got = extract_result(&events, &ResultExtractor::LastAssistantText);
        assert_eq!(got, Some("lo".into()));
    }

    #[test]
    fn extract_last_tool_result() {
        let events = vec![
            evt_tool_result("c1", "first"),
            evt_tool_result("c2", "second"),
        ];
        let got = extract_result(&events, &ResultExtractor::LastToolResult("c2".into()));
        assert_eq!(got, Some("second".into()));
    }

    #[test]
    fn extract_returns_none_when_no_match() {
        let events = vec![evt_tool_result("c1", "x")];
        let got = extract_result(&events, &ResultExtractor::LastToolResult("missing".into()));
        assert!(got.is_none());
    }

    #[test]
    fn build_child_messages_appends_user_input() {
        use reflect_llm::{ContentBlock, UserContent};
        let parent = vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("old1")],
        })];
        let new = ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text("new")],
        });
        let msgs = build_child_initial_messages(&parent, new);
        assert_eq!(msgs.len(), 2);
    }
}
