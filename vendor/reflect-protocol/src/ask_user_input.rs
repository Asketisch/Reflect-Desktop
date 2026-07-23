//! v1.1.0 P1 #15:`ask_user` 自由文本询问协议类型。
//!
//! 与 `ask_user_question`(结构化多选题)区分:`ask_user` 只带一条 prompt,
//! 用户在 TUI 单行 modal 输入自由文本,通过 `Op::AskUserInputResponse`
//! 回执给 `ApprovalGate::ask_user`。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// LLM 通过 `ask_user` 工具向用户发起的自由文本询问。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct AskUserInputEvent {
    pub request_id: String,
    pub prompt: String,
}

impl AskUserInputEvent {
    pub fn new(request_id: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            prompt: prompt.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ask_user_input_event_serde_roundtrip() {
        let ev = AskUserInputEvent::new("u1", "Which API key should I use?");
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains(r#""request_id":"u1""#), "got: {j}");
        assert!(j.contains("Which API key"), "got: {j}");
        let back: AskUserInputEvent = serde_json::from_str(&j).unwrap();
        assert_eq!(back.request_id, "u1");
        assert_eq!(back.prompt, "Which API key should I use?");
    }
}
