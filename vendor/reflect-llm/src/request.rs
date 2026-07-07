//! `ChatRequest` and its nested types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use reflect_protocol::ToolOutput;

// ── Top-level request ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model name WITHOUT provider prefix (e.g. `"gpt-4o"`, `"claude-3-5-sonnet-latest"`).
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub system: SystemBlocks,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    #[serde(default)]
    pub cache_control: Vec<CacheBreak>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub stop: Vec<String>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            system: SystemBlocks::default(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            thinking: None,
            cache_control: Vec::new(),
            metadata: HashMap::new(),
            stop: Vec::new(),
        }
    }
}

// ── Messages ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ChatMessage {
    System(String),
    User(UserContent),
    Assistant(AssistantContent),
    Tool(ToolResult),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserContent {
    pub blocks: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssistantContent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    /// The fully-buffered JSON arguments (concatenated from deltas).
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    /// String form (or pre-serialized JSON) of the tool's output content.
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    pub fn from_output(call_id: impl Into<String>, output: &ToolOutput) -> Self {
        // Serialize the ContentBlock list as a JSON string. M2 may prefer
        // a flat stringification; M1 keeps it simple.
        let s = serde_json::to_string(&output.content).unwrap_or_default();
        Self {
            call_id: call_id.into(),
            content: s,
            is_error: output.is_error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { data: Vec<u8>, mime_type: String },
}

impl ContentBlock {
    pub fn text(s: impl Into<String>) -> Self {
        ContentBlock::Text { text: s.into() }
    }
}

// ── System blocks + cache control ────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemBlocks(pub Vec<SystemBlock>);

impl SystemBlocks {
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|b| b.text.is_empty())
    }
    /// Return a non-empty system as a single string concatenation (OpenAI
    /// has no system blocks; Anthropic does).
    pub fn as_single_string(&self) -> Option<String> {
        let parts: Vec<&str> = self.0.iter().map(|b| b.text.as_str()).collect();
        let joined = parts.join("\n");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlock {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    #[serde(default)]
    pub ephemeral: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheControlKind {
    Ephemeral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CacheTtl {
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub kind: CacheControlKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<CacheTtl>,
}

impl Default for CacheControl {
    fn default() -> Self {
        Self {
            kind: CacheControlKind::Ephemeral,
            ttl: Some(CacheTtl::FiveMinutes),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheBreak {
    pub after_message_index: usize,
    pub ttl: CacheTtl,
}

// ── Thinking ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThinkingConfig {
    Enabled { budget_tokens: u32 },
    Disabled,
    OpenAIReasoning { effort: ReasoningEffort },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

/// v1.x S4:把协议层 `ReasoningEffortMirror` 桥接到 LLM 层 `ReasoningEffort`。
///
/// 协议层独立枚举是为了避免 `protocol → llm` 反向依赖;`submission_loop`
/// / `model_call` 是**唯一**需要该转换的地方,集中在 `From` impl 便于
/// 协议字段扩展(加 `XHigh` 等)时单点同步。
impl From<reflect_protocol::ReasoningEffortMirror> for ReasoningEffort {
    fn from(m: reflect_protocol::ReasoningEffortMirror) -> Self {
        match m {
            reflect_protocol::ReasoningEffortMirror::Low => ReasoningEffort::Low,
            reflect_protocol::ReasoningEffortMirror::Medium => ReasoningEffort::Medium,
            reflect_protocol::ReasoningEffortMirror::High => ReasoningEffort::High,
        }
    }
}

// ── Tool spec (mirror of reflect_tools::ToolSpec for LLM use) ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolSpec {
    Function {
        name: String,
        description: String,
        parameters: Value,
    },
}

impl ToolSpec {
    pub fn name(&self) -> &str {
        match self {
            ToolSpec::Function { name, .. } => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_serde_roundtrip() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage::User(UserContent {
                blocks: vec![ContentBlock::text("hi")],
            })],
            tools: vec![],
            system: SystemBlocks::default(),
            temperature: Some(0.7),
            max_tokens: Some(1024),
            top_p: None,
            thinking: None,
            cache_control: vec![],
            metadata: HashMap::new(),
            stop: vec![],
        };
        let j = serde_json::to_string(&req).unwrap();
        let back: ChatRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(back.model, "gpt-4o");
        assert_eq!(back.messages.len(), 1);
    }

    #[test]
    fn system_blocks_as_single_string_skips_empty() {
        let blocks = SystemBlocks(vec![SystemBlock {
            text: "x".into(),
            cache_control: None,
            ephemeral: false,
        }]);
        assert_eq!(blocks.as_single_string().as_deref(), Some("x"));
        assert!(SystemBlocks::default().as_single_string().is_none());
    }
}
