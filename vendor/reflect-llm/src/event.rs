//! `ChatEvent` — the streaming event the model returns.
//!
//! Provider-agnostic. Providers translate their SSE protocol into these
//! events; `reflect-core` translates these into `EventMsg` events.

use serde::{Deserialize, Serialize};

use crate::error::LlmError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    /// First chunk with response id and model name.
    MessageStart { id: String, model: String },

    /// Incremental text content from the assistant.
    ContentDelta(String),

    /// Begin of a tool use block.
    ToolUseStart {
        id: String,
        name: String,
        input_json: String,
    },

    /// Incremental JSON arguments of the active tool use block.
    ToolUseDelta(String),

    /// Incremental thinking content (Anthropic extended thinking).
    ThinkingDelta(String),

    /// Stream finished.
    MessageStop,

    /// Token usage snapshot. `cached_tokens` is the cache_read discount
    /// segment (a subset of `input_tokens`); `cache_write_tokens` is the
    /// cache_creation segment (also folded into `input_tokens`). On
    /// providers without prompt caching both are zero.
    Usage {
        input_tokens: u32,
        output_tokens: u32,
        cached_tokens: u32,
        cache_write_tokens: u32,
    },

    /// Mid-stream error (does not necessarily terminate the stream).
    Error(LlmError),
}

impl ChatEvent {
    /// Stable string discriminator.
    pub fn discriminant(&self) -> &'static str {
        match self {
            ChatEvent::MessageStart { .. } => "message_start",
            ChatEvent::ContentDelta(_) => "content_delta",
            ChatEvent::ToolUseStart { .. } => "tool_use_start",
            ChatEvent::ToolUseDelta(_) => "tool_use_delta",
            ChatEvent::ThinkingDelta(_) => "thinking_delta",
            ChatEvent::MessageStop => "message_stop",
            ChatEvent::Usage { .. } => "usage",
            ChatEvent::Error(_) => "error",
        }
    }
}
