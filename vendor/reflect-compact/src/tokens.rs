//! Token estimation. Approximation, NOT a billing-grade counter.
//!
//! Heuristic: `len(text) / 3.5` chars per token, image blocks = 1000
//! tokens. Matches reflect's `graph.py:_estimate_tokens`.

use reflect_llm::{ChatMessage, ContentBlock, ToolResult};

/// Estimate the total token count of a slice of chat messages. Pure
/// function — does not call any LLM. Useful for triggering compact
/// strategies; not a substitute for a real tokenizer (e.g. tiktoken).
pub fn estimate_messages(messages: &[ChatMessage]) -> u32 {
    messages.iter().map(estimate_message).sum()
}

fn estimate_message(msg: &ChatMessage) -> u32 {
    match msg {
        ChatMessage::System(s) => chars_to_tokens(s),
        ChatMessage::User(u) => u.blocks.iter().map(estimate_block).sum(),
        ChatMessage::Assistant(a) => {
            let mut t = 0;
            if let Some(text) = &a.text {
                t += chars_to_tokens(text);
            }
            if let Some(thinking) = &a.thinking {
                t += chars_to_tokens(thinking);
            }
            // tool_calls: each call's JSON args count.
            for tc in &a.tool_calls {
                t += chars_to_tokens(&tc.arguments.to_string());
                t += chars_to_tokens(&tc.name);
            }
            t
        }
        ChatMessage::Tool(r) => chars_to_tokens(&tool_result_text(r)),
    }
}

fn estimate_block(b: &ContentBlock) -> u32 {
    match b {
        ContentBlock::Text { text } => chars_to_tokens(text),
        // Images are large fixed-budget blocks in reflect.
        ContentBlock::Image { .. } => 1000,
    }
}

fn tool_result_text(r: &ToolResult) -> String {
    r.content.clone()
}

fn chars_to_tokens(s: &str) -> u32 {
    // Round to nearest u32 to keep arithmetic exact.
    ((s.chars().count() as f64) / 3.5).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_llm::{AssistantContent, ContentBlock, ToolCallRequest, ToolResult, UserContent};

    #[test]
    fn empty_messages_yield_zero() {
        assert_eq!(estimate_messages(&[]), 0);
    }

    #[test]
    fn short_text_rounds_up() {
        // 4 chars / 3.5 = 1.14, ceil = 2
        let msgs = vec![ChatMessage::System("abcd".into())];
        assert_eq!(estimate_messages(&msgs), 2);
    }

    #[test]
    fn long_text_uses_3_5_ratio() {
        // 35 chars / 3.5 = 10
        let s: String = "a".repeat(35);
        let msgs = vec![ChatMessage::System(s)];
        assert_eq!(estimate_messages(&msgs), 10);
    }

    #[test]
    fn image_block_equals_1000_tokens() {
        let msgs = vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::Image {
                data: vec![0xff; 100],
                mime_type: "image/png".into(),
            }],
        })];
        assert_eq!(estimate_messages(&msgs), 1000);
    }

    #[test]
    fn assistant_counts_text_thinking_and_tool_calls() {
        let a = AssistantContent {
            text: Some("hello".into()),
            tool_calls: vec![ToolCallRequest {
                id: "c1".into(),
                name: "read".into(),
                arguments: serde_json::json!({"path": "/tmp/x"}),
            }],
            thinking: Some("thinking about it".into()),
        };
        let msgs = vec![ChatMessage::Assistant(a)];
        let t = estimate_messages(&msgs);
        // "hello" = ceil(5/3.5)=2; "thinking about it" = ceil(18/3.5)=6;
        // "read" = ceil(4/3.5)=2; args JSON = some count
        assert!(t >= 10);
    }

    #[test]
    fn tool_result_counts_content() {
        let r = ToolResult {
            call_id: "c1".into(),
            content: "ok result".into(),
            is_error: false,
        };
        let msgs = vec![ChatMessage::Tool(r)];
        let t = estimate_messages(&msgs);
        // "ok result" = ceil(9/3.5) = 3
        assert_eq!(t, 3);
    }

    #[test]
    fn sums_across_messages() {
        let msgs = vec![
            ChatMessage::System("hello".into()),
            ChatMessage::User(UserContent {
                blocks: vec![ContentBlock::text("world")],
            }),
        ];
        let t = estimate_messages(&msgs);
        // "hello"=2, "world"=2
        assert_eq!(t, 4);
    }
}
