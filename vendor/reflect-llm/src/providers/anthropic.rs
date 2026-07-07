//! Anthropic provider — Messages streaming with prompt caching.
//!
//! Request body: `POST /v1/messages` with `stream: true`. Parses the SSE
//! event stream (typed events like `message_start`, `content_block_start`,
//! `content_block_delta`, `message_delta`, `message_stop`).

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use eventsource_stream::EventStream;
use futures::{Stream, StreamExt};
use reqwest::Response;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::capabilities::Capabilities;
use crate::client::ModelClient;
use crate::error::LlmError;
use crate::event::ChatEvent;
use crate::request::{
    CacheControl, CacheControlKind, CacheTtl, ChatMessage, ChatRequest, ContentBlock,
    ThinkingConfig, ToolSpec,
};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// M8: number of trailing tools to attach `cache_control: ephemeral` to.
/// Anthropic supports up to 4 cache breakpoints per request; we reserve the
/// last one for the system prompt and use the remaining 3 for tool defs.
const CACHE_TAIL_TOOLS: usize = 3;

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub timeout: Duration,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: None,
            timeout: Duration::from_secs(60),
        }
    }
}

pub struct AnthropicClient {
    config: AnthropicConfig,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(config: AnthropicConfig) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(LlmError::from)?;
        Ok(Self { config, http })
    }
}

#[async_trait]
impl ModelClient for AnthropicClient {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn provider_kind(&self) -> crate::ProviderKind {
        crate::ProviderKind::Anthropic
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tool_use: true,
            prompt_caching: true,
            extended_thinking: true,
            vision: true,
            json_mode: true, // via tool-forced JSON
            system_blocks: true,
        }
    }

    async fn stream(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError> {
        let body: Value = serde_json::to_value(AnthropicRequest::from(request))
            .map_err(|e| LlmError::Internal(e.to_string()))?;

        let base = self
            .config
            .base_url
            .as_deref()
            .unwrap_or(DEFAULT_BASE_URL)
            .trim_end_matches('/');
        let url = format!("{base}/v1/messages");

        if cancel.is_cancelled() {
            return Err(LlmError::Cancelled);
        }

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .body(body.to_string())
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let headers = response.headers().clone();
            let text = response.text().await.unwrap_or_default();
            return Err(classify_status(status_code, &text, &headers));
        }

        Ok(Box::pin(stream_sse(response, cancel)))
    }
}

fn classify_status(status: u16, body: &str, headers: &reqwest::header::HeaderMap) -> LlmError {
    match status {
        401 => LlmError::Auth,
        429 => {
            let retry_after_ms = parse_retry_after_ms(headers).unwrap_or(1000);
            LlmError::RateLimited { retry_after_ms }
        }
        529 => LlmError::Overloaded {
            retry_after_ms: 1000,
        },
        400 if body.contains("prompt is too long") => {
            LlmError::ContextLengthExceeded { used: 0, limit: 0 }
        }
        400 => LlmError::InvalidRequest {
            message: body.to_string(),
        },
        500..=599 => LlmError::Provider {
            status,
            message: body.to_string(),
        },
        _ => LlmError::Provider {
            status,
            message: body.to_string(),
        },
    }
}

fn parse_retry_after_ms(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let v = headers.get("retry-after")?.to_str().ok()?;
    v.parse::<f64>().ok().map(|s| (s * 1000.0) as u64)
}

// ── SSE → ChatEvent ──────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct StreamState {
    /// Whether the current content block is a thinking block.
    in_thinking: bool,
    /// Accumulated input tokens (from `message_start.message.usage.input_tokens`).
    /// Note: Anthropic's `input_tokens` already includes the
    /// `cache_creation_input_tokens` subsegment.
    input_tokens: u32,
    /// Accumulated output tokens (updated by `message_delta.usage.output_tokens`).
    output_tokens: u32,
    /// Cached input tokens (from `message_start.message.usage.cache_read_input_tokens`).
    /// A subset of `input_tokens` (cache_read discount segment).
    cached_tokens: u32,
    /// M8: cache_creation input tokens (from `message_start.message.usage.cache_creation_input_tokens`).
    /// A subset of `input_tokens` (cache_write billable segment).
    cache_write_tokens: u32,
    /// Whether the Usage event has been emitted (we emit one at message_stop).
    usage_emitted: bool,
}

fn stream_sse(
    response: Response,
    cancel: CancellationToken,
) -> impl Stream<Item = Result<ChatEvent, LlmError>> + Send {
    let byte_stream = response.bytes_stream();
    let mut sse = EventStream::new(byte_stream).peekable();
    let mut state = StreamState::default();

    async_stream::stream! {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    yield Ok(ChatEvent::Error(LlmError::Cancelled));
                    return;
                }
                next = sse.next() => {
                    let Some(item) = next else { return };
                    let evt = match item {
                        Ok(ev) => ev,
                        Err(e) => {
                            yield Err(LlmError::SseParse(e.to_string()));
                            return;
                        }
                    };
                    // event field is in evt.event; data is in evt.data
                    for ev in parse_sse_event(&evt.event, &evt.data, &mut state) {
                        yield Ok(ev);
                    }
                }
            }
        }
    }
}

fn parse_sse_event(event: &str, data: &str, state: &mut StreamState) -> Vec<ChatEvent> {
    let mut out = Vec::new();
    let v: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_e) => {
            // Malformed SSE data — drop the event and continue the stream.
            return vec![];
        }
    };
    match event {
        "message_start" => {
            let id = v
                .pointer("/message/id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let model = v
                .pointer("/message/model")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(input) = v
                .pointer("/message/usage/input_tokens")
                .and_then(|x| x.as_u64())
            {
                state.input_tokens = input as u32;
            }
            if let Some(cached) = v
                .pointer("/message/usage/cache_read_input_tokens")
                .and_then(|x| x.as_u64())
            {
                state.cached_tokens = cached as u32;
            }
            // M8: capture the cache_creation segment (billable at the
            // write-tier multiplier, distinct from the cache_read discount).
            if let Some(write) = v
                .pointer("/message/usage/cache_creation_input_tokens")
                .and_then(|x| x.as_u64())
            {
                state.cache_write_tokens = write as u32;
            }
            if let Some(output) = v
                .pointer("/message/usage/output_tokens")
                .and_then(|x| x.as_u64())
            {
                state.output_tokens = output as u32;
            }
            out.push(ChatEvent::MessageStart { id, model });
        }
        "content_block_start" => {
            let block_type = v
                .pointer("/content_block/type")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            state.in_thinking = block_type == "thinking";
            if block_type == "tool_use" {
                let id = v
                    .pointer("/content_block/id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = v
                    .pointer("/content_block/name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                out.push(ChatEvent::ToolUseStart {
                    id,
                    name,
                    input_json: String::new(),
                });
            }
        }
        "content_block_delta" => {
            let delta_type = v
                .pointer("/delta/type")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            match delta_type {
                "text_delta" => {
                    if let Some(text) = v.pointer("/delta/text").and_then(|x| x.as_str()) {
                        out.push(ChatEvent::ContentDelta(text.to_string()));
                    }
                }
                "input_json_delta" => {
                    if let Some(partial) = v.pointer("/delta/partial_json").and_then(|x| x.as_str())
                    {
                        out.push(ChatEvent::ToolUseDelta(partial.to_string()));
                    }
                }
                "thinking_delta" => {
                    if let Some(text) = v.pointer("/delta/thinking").and_then(|x| x.as_str()) {
                        out.push(ChatEvent::ThinkingDelta(text.to_string()));
                    }
                }
                _ => {}
            }
        }
        "content_block_stop" => {
            state.in_thinking = false;
        }
        "message_delta" => {
            if let Some(output) = v.pointer("/usage/output_tokens").and_then(|x| x.as_u64()) {
                state.output_tokens = output as u32;
            }
            // Emit a final Usage event on message_delta (message_stop is next).
            if !state.usage_emitted {
                out.push(ChatEvent::Usage {
                    input_tokens: state.input_tokens,
                    output_tokens: state.output_tokens,
                    cached_tokens: state.cached_tokens,
                    cache_write_tokens: state.cache_write_tokens,
                });
                state.usage_emitted = true;
            }
        }
        "message_stop" => {
            out.push(ChatEvent::MessageStop);
        }
        _ => {
            // ping / error / other typed events — ignore
        }
    }
    out
}

// ── ChatRequest → Anthropic request body ────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    model: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    system: Vec<Value>,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop_sequences: Vec<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<Value>,
}

impl From<ChatRequest> for AnthropicRequest {
    /// Public for test/snapshot access. Production code uses
    /// `AnthropicClient::stream` instead.
    fn from(req: ChatRequest) -> Self {
        // Inject cache_control on the last system block if none is present.
        let mut system_blocks: Vec<Value> = req
            .system
            .0
            .iter()
            .map(|b| {
                let mut v = serde_json::json!({"type": "text", "text": b.text});
                if let Some(cc) = b.cache_control {
                    v["cache_control"] = cache_control_json(cc);
                }
                v
            })
            .collect();
        if !system_blocks.is_empty()
            && !system_blocks
                .last()
                .and_then(|v| v.get("cache_control"))
                .is_some()
        {
            let cc = serde_json::json!({"type": "ephemeral", "ttl": "5m"});
            system_blocks.last_mut().unwrap()["cache_control"] = cc;
        }

        // Build messages
        let mut messages = Vec::new();
        for m in req.messages {
            push_message(m, &mut messages);
        }

        // M8: attach `cache_control: ephemeral` to the last N message
        // content blocks when the prompt has them (driven by
        // `req.cache_control: Vec<CacheBreak>` populated upstream by
        // `reflect_prompt::inject_cache_control`). Skip thinking blocks —
        // Anthropic API rejects cache_control on thinking content.
        let message_break_indices: std::collections::HashSet<usize> = req
            .cache_control
            .iter()
            .map(|b| b.after_message_index)
            .filter(|&i| i < messages.len())
            .collect();
        if !message_break_indices.is_empty() {
            for idx in &message_break_indices {
                if let Some(msg) = messages.get_mut(*idx) {
                    attach_cache_break_to_message(msg, req.thinking.is_some());
                }
            }
        }

        // Tools (Anthropic has no `type:"function"` wrapper)
        let mut tools: Vec<Value> = req
            .tools
            .iter()
            .map(|t| match t {
                ToolSpec::Function {
                    name,
                    description,
                    parameters,
                } => serde_json::json!({
                    "name": name,
                    "description": description,
                    "input_schema": parameters,
                }),
            })
            .collect();

        // M8: trailing N tools get `cache_control: ephemeral` when
        // `caching.rs` flagged it via `req.metadata["cache_break_tool"]`.
        // This is the highest-leverage breakpoint after the system prompt
        // because tool definitions are large and rarely change between turns.
        if !tools.is_empty()
            && req.metadata.get("cache_break_tool").map(String::as_str) == Some("true")
        {
            let start = tools.len().saturating_sub(CACHE_TAIL_TOOLS);
            for t in &mut tools[start..] {
                t["cache_control"] = serde_json::json!({"type": "ephemeral", "ttl": "5m"});
            }
        }

        // max_tokens is required by Anthropic
        let max_tokens = req.max_tokens.unwrap_or(4096);

        // Thinking — when enabled, temperature must be 1.
        let thinking = match &req.thinking {
            Some(ThinkingConfig::Enabled { budget_tokens }) => Some(serde_json::json!({
                "type": "enabled",
                "budget_tokens": budget_tokens
            })),
            Some(ThinkingConfig::OpenAIReasoning { .. }) => None,
            _ => None,
        };
        let temperature = if thinking.is_some() {
            // Anthropic requires temperature=1 with thinking enabled.
            Some(1.0)
        } else {
            req.temperature
        };

        Self {
            model: req.model,
            system: system_blocks,
            messages,
            tools,
            max_tokens,
            temperature,
            top_p: req.top_p,
            stop_sequences: req.stop,
            stream: true,
            thinking,
        }
    }
}

/// M8: attach `cache_control: ephemeral` to the last content block of an
/// Anthropic message. Skips thinking blocks when `thinking_enabled` is true
/// because the Anthropic API rejects cache_control on thinking content.
/// For a `{role: "user", content: [tool_result]}` we attach to the tool_result
/// block; for `{role: "user", content: [text,...]}` we attach to the last text
/// block; for `{role: "assistant", content: [text|tool_use|thinking]}` we
/// attach to the last non-thinking block (or skip if all are thinking).
fn attach_cache_break_to_message(msg: &mut Value, thinking_enabled: bool) {
    let cc = || serde_json::json!({"type": "ephemeral", "ttl": "5m"});
    if let Some(content_arr) = msg.get_mut("content").and_then(|c| c.as_array_mut()) {
        // Walk content blocks right-to-left; pick the first one that isn't
        // a thinking block (when thinking is enabled).
        for block in content_arr.iter_mut().rev() {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if thinking_enabled && block_type == "thinking" {
                continue;
            }
            // Only attach to types that accept cache_control per Anthropic
            // API: text, image, tool_use, tool_result, document. Skip
            // thinking (already filtered) and any unknown type.
            if matches!(
                block_type,
                "text" | "image" | "tool_use" | "tool_result" | "document"
            ) {
                block["cache_control"] = cc();
                return;
            }
            // Unknown type — give up to avoid sending an invalid wire shape.
            return;
        }
    }
}

fn cache_control_json(cc: CacheControl) -> Value {
    let mut v = serde_json::json!({"type": match cc.kind {
        CacheControlKind::Ephemeral => "ephemeral",
    }});
    if let Some(ttl) = cc.ttl {
        v["ttl"] = match ttl {
            CacheTtl::FiveMinutes => Value::String("5m".into()),
            CacheTtl::OneHour => Value::String("1h".into()),
        };
    } else {
        v["ttl"] = Value::String("5m".into());
    }
    v
}

fn push_message(m: ChatMessage, out: &mut Vec<Value>) {
    match m {
        ChatMessage::System(s) => {
            // System is a top-level field for Anthropic, not a message.
            // If a stray ChatMessage::System appears, ignore (we already
            // handled `request.system` at the top level).
            let _ = s;
        }
        ChatMessage::User(u) => {
            let content: Vec<Value> = u
                .blocks
                .into_iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => {
                        serde_json::json!({"type": "text", "text": text})
                    }
                    ContentBlock::Image { data, mime_type } => {
                        let b64 = base64_encode(&data);
                        serde_json::json!({
                            "type": "image",
                            "source": {"type": "base64", "media_type": mime_type, "data": b64}
                        })
                    }
                })
                .collect();
            out.push(serde_json::json!({"role": "user", "content": content}));
        }
        ChatMessage::Assistant(a) => {
            let mut content: Vec<Value> = Vec::new();
            if let Some(t) = a.text {
                content.push(serde_json::json!({"type": "text", "text": t}));
            }
            if let Some(think) = a.thinking {
                content.push(serde_json::json!({"type": "thinking", "thinking": think}));
            }
            for tc in a.tool_calls {
                content.push(serde_json::json!({
                    "type": "tool_use",
                    "id": tc.id,
                    "name": tc.name,
                    "input": tc.arguments,
                }));
            }
            out.push(serde_json::json!({"role": "assistant", "content": content}));
        }
        ChatMessage::Tool(t) => {
            out.push(serde_json::json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": t.call_id,
                    "content": t.content,
                    "is_error": t.is_error,
                }]
            }));
        }
    }
}

// Tiny base64 encoder (same as OpenAI module; duplicated for crate-internal clarity).
fn base64_encode(input: &[u8]) -> String {
    const ALPH: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(ALPH[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPH[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(ALPH[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(ALPH[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{CacheBreak, ContentBlock as CB, SystemBlock, SystemBlocks, UserContent};

    #[test]
    fn system_gets_cache_control_injected() {
        let req = ChatRequest {
            model: "claude-3-5-sonnet-latest".into(),
            messages: vec![],
            tools: vec![],
            system: SystemBlocks(vec![SystemBlock {
                text: "you are helpful".into(),
                cache_control: None,
                ephemeral: false,
            }]),
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            thinking: None,
            cache_control: vec![],
            metadata: Default::default(),
            stop: vec![],
        };
        let v: Value = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        let system = v["system"].as_array().unwrap();
        assert_eq!(system.len(), 1);
        assert_eq!(system[0]["type"], "text");
        assert_eq!(
            system[0]["cache_control"],
            serde_json::json!({"type": "ephemeral", "ttl": "5m"})
        );
    }

    #[test]
    fn existing_cache_control_is_preserved() {
        let req = ChatRequest {
            model: "claude-3-5-sonnet-latest".into(),
            messages: vec![],
            tools: vec![],
            system: SystemBlocks(vec![SystemBlock {
                text: "x".into(),
                cache_control: Some(CacheControl {
                    kind: CacheControlKind::Ephemeral,
                    ttl: Some(CacheTtl::OneHour),
                }),
                ephemeral: false,
            }]),
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            thinking: None,
            cache_control: vec![],
            metadata: Default::default(),
            stop: vec![],
        };
        let v: Value = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        assert_eq!(
            v["system"][0]["cache_control"],
            serde_json::json!({"type": "ephemeral", "ttl": "1h"})
        );
    }

    #[test]
    fn thinking_forces_temperature_one() {
        let req = ChatRequest {
            model: "claude-3-5-sonnet-latest".into(),
            messages: vec![],
            tools: vec![],
            system: SystemBlocks::default(),
            temperature: Some(0.5),
            max_tokens: Some(1024),
            top_p: None,
            thinking: Some(ThinkingConfig::Enabled {
                budget_tokens: 1024,
            }),
            cache_control: vec![],
            metadata: Default::default(),
            stop: vec![],
        };
        let v: Value = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        assert_eq!(v["temperature"], 1.0);
        assert_eq!(v["thinking"]["type"], "enabled");
        assert_eq!(v["thinking"]["budget_tokens"], 1024);
    }

    #[test]
    fn max_tokens_defaults_to_4096() {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![],
            tools: vec![],
            system: SystemBlocks::default(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            thinking: None,
            cache_control: vec![],
            metadata: Default::default(),
            stop: vec![],
        };
        let v: Value = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        assert_eq!(v["max_tokens"], 4096);
    }

    #[test]
    fn parse_message_start_emits_message_start_event() {
        let mut state = StreamState::default();
        let data = r#"{"message":{"id":"msg_1","model":"claude-3-5-sonnet-latest","usage":{"input_tokens":10,"output_tokens":1,"cache_read_input_tokens":7}}}"#;
        let evs = parse_sse_event("message_start", data, &mut state);
        assert!(
            matches!(&evs[0], ChatEvent::MessageStart { id, model } if id == "msg_1" && model.starts_with("claude"))
        );
        assert_eq!(state.input_tokens, 10);
        assert_eq!(state.cached_tokens, 7);
    }

    #[test]
    fn parse_content_block_delta_text_and_tool() {
        let mut state = StreamState::default();
        // text delta
        let data = r#"{"index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let evs = parse_sse_event("content_block_delta", data, &mut state);
        assert!(matches!(&evs[0], ChatEvent::ContentDelta(s) if s == "Hello"));

        // tool_use start
        let data = r#"{"index":1,"content_block":{"type":"tool_use","id":"toolu_1","name":"bash"},"delta":{}}"#;
        let evs = parse_sse_event("content_block_start", data, &mut state);
        assert!(
            matches!(&evs[0], ChatEvent::ToolUseStart { id, name, .. } if id == "toolu_1" && name == "bash")
        );

        // input_json_delta
        let data = r#"{"index":1,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":"}}"#;
        let evs = parse_sse_event("content_block_delta", data, &mut state);
        assert!(matches!(&evs[0], ChatEvent::ToolUseDelta(s) if s.contains("cmd")));
    }

    #[test]
    fn parse_thinking_delta() {
        let mut state = StreamState::default();
        let data = r#"{"index":0,"content_block":{"type":"thinking","thinking":""},"delta":{}}"#;
        parse_sse_event("content_block_start", data, &mut state);
        assert!(state.in_thinking);
        let data = r#"{"index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}"#;
        let evs = parse_sse_event("content_block_delta", data, &mut state);
        assert!(matches!(&evs[0], ChatEvent::ThinkingDelta(s) if s == "hmm"));
    }

    #[test]
    fn classify_status_401_and_429() {
        let h = reqwest::header::HeaderMap::new();
        assert!(matches!(classify_status(401, "", &h), LlmError::Auth));
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("retry-after", "3".parse().unwrap());
        match classify_status(429, "", &h) {
            LlmError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 3000),
            _ => panic!(),
        }
    }

    #[test]
    fn message_stop_emits_stop() {
        let mut state = StreamState::default();
        let evs = parse_sse_event("message_stop", "{}", &mut state);
        assert!(matches!(&evs[0], ChatEvent::MessageStop));
    }

    // Suppress dead-code for UserContent/CB (used in upstream test scaffolding)
    #[allow(dead_code)]
    fn _suppress(_: UserContent, _: CB) {}

    // ── M8: cache_control wire-up ───────────────────────────────────────

    fn make_req_with_tools_and_cache() -> ChatRequest {
        let mut req = ChatRequest {
            model: "claude-3-5-sonnet-latest".into(),
            messages: vec![ChatMessage::User(UserContent {
                blocks: vec![CB::text("hi")],
            })],
            tools: (0..5)
                .map(|i| ToolSpec::Function {
                    name: format!("t{i}"),
                    description: "".into(),
                    parameters: serde_json::json!({"type": "object"}),
                })
                .collect(),
            system: SystemBlocks::default(),
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            thinking: None,
            cache_control: vec![],
            metadata: Default::default(),
            stop: vec![],
        };
        req.metadata
            .insert("cache_break_tool".to_string(), "true".to_string());
        req
    }

    #[test]
    fn from_request_attaches_cache_control_to_last_n_tools() {
        let req = make_req_with_tools_and_cache();
        let v = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        let tools = v["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        // First (5 - 3 = 2) tools untouched; last 3 get cache_control.
        for (i, t) in tools.iter().enumerate() {
            let has_cc = t.get("cache_control").is_some();
            if i < 2 {
                assert!(!has_cc, "tool[{i}] should NOT have cache_control, got {t}");
            } else {
                assert!(has_cc, "tool[{i}] should have cache_control, got {t}");
                assert_eq!(t["cache_control"]["type"], "ephemeral");
                assert_eq!(t["cache_control"]["ttl"], "5m");
            }
        }
    }

    #[test]
    fn from_request_skips_tool_cache_control_when_metadata_flag_missing() {
        let mut req = make_req_with_tools_and_cache();
        req.metadata.remove("cache_break_tool");
        let v = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        let tools = v["tools"].as_array().unwrap();
        for (i, t) in tools.iter().enumerate() {
            assert!(
                t.get("cache_control").is_none(),
                "tool[{i}] should NOT have cache_control when flag absent, got {t}"
            );
        }
    }

    #[test]
    fn from_request_anchor_message_gets_cache_control() {
        use crate::request::CacheBreak;
        let req = ChatRequest {
            model: "claude-3-5-sonnet-latest".into(),
            messages: vec![
                ChatMessage::User(UserContent {
                    blocks: vec![CB::text("first")],
                }),
                ChatMessage::Assistant(crate::request::AssistantContent {
                    text: Some("second".into()),
                    tool_calls: vec![],
                    thinking: None,
                }),
                ChatMessage::User(UserContent {
                    blocks: vec![CB::text("third — anchor")],
                }),
            ],
            tools: vec![],
            system: SystemBlocks::default(),
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            thinking: None,
            cache_control: vec![CacheBreak {
                after_message_index: 2,
                ttl: CacheTtl::FiveMinutes,
            }],
            metadata: Default::default(),
            stop: vec![],
        };
        let v = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        let messages = v["messages"].as_array().unwrap();
        // Third message last content block has cache_control.
        let last = messages[2]["content"].as_array().unwrap().last().unwrap();
        assert_eq!(last["cache_control"]["type"], "ephemeral");
        assert_eq!(last["cache_control"]["ttl"], "5m");
        // First two messages untouched.
        for (i, m) in messages.iter().enumerate().take(2) {
            let blocks = m["content"].as_array().unwrap();
            for b in blocks {
                assert!(
                    b.get("cache_control").is_none(),
                    "messages[{i}] should not have cache_control, got {b}"
                );
            }
        }
    }

    #[test]
    fn from_request_anchor_skips_thinking_block_when_thinking_enabled() {
        // M8: when thinking is enabled, the anchor's `attach_cache_break_to_message`
        // helper must skip thinking blocks (Anthropic API rejects cache_control
        // on thinking content). Use a hand-built assistant message that has
        // [text, thinking] so we can assert only the text block gets the CC.
        use crate::request::AssistantContent;
        let req = ChatRequest {
            model: "claude-3-5-sonnet-latest".into(),
            messages: vec![ChatMessage::Assistant(AssistantContent {
                text: Some("hi".into()),
                tool_calls: vec![],
                thinking: Some("internal monologue".into()),
            })],
            tools: vec![],
            system: SystemBlocks::default(),
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            thinking: Some(ThinkingConfig::Enabled {
                budget_tokens: 1024,
            }),
            cache_control: vec![CacheBreak {
                after_message_index: 0,
                ttl: CacheTtl::FiveMinutes,
            }],
            metadata: Default::default(),
            stop: vec![],
        };
        let v = serde_json::to_value(AnthropicRequest::from(req)).unwrap();
        let blocks = v["messages"][0]["content"].as_array().unwrap();
        // Order: [text, thinking] — text should have cache_control, thinking should NOT.
        assert_eq!(blocks[0]["type"], "text");
        assert_eq!(blocks[0]["cache_control"]["type"], "ephemeral");
        assert_eq!(blocks[1]["type"], "thinking");
        assert!(
            blocks[1].get("cache_control").is_none(),
            "thinking block must NOT carry cache_control, got {blocks:?}"
        );
    }

    /// v1.0.0-rc2: AnthropicClient override `provider_kind() = Anthropic`。
    #[test]
    fn provider_kind_reports_anthropic() {
        let client = AnthropicClient::new(AnthropicConfig::default()).unwrap();
        assert_eq!(client.provider_kind(), crate::ProviderKind::Anthropic);
        assert_eq!(client.name(), "anthropic");
    }
}
