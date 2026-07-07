//! LLM-backed summarization.
//!
//! Port of reflect `summarizer.py`. Produces a 9-section Chinese summary
//! of a conversation, wrapped in `<summary>...</summary>` tags. Used as
//! the last-resort strategy in [`crate::strategy::Compactor`] when
//! microcompact + smart_prune still leave the conversation over budget.
//!
//! Cycle avoidance: the trait `Summarizer` is the abstract type
//! `Compactor` depends on; the concrete `LlmSummarizer` lives in this
//! crate but only takes an `Arc<dyn ModelClient>` injected by the
//! caller. This means `reflect-compact` only depends on
//! `reflect-llm` for the type, not on any provider implementation.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use thiserror::Error;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use reflect_llm::{
    AssistantContent, ChatEvent, ChatMessage, ChatRequest, ContentBlock, ModelClient, Role,
    RoutingPolicy, SharedModelRegistry, SystemBlocks, ToolSpec, UserContent,
};

/// Default timeout for a single summarize call. Matches reflect.
pub const SUMMARIZE_TIMEOUT: Duration = Duration::from_secs(90);

/// 9-section Chinese prompt. Mirrors reflect's `summarizer.py` SUMMARIZE_PROMPT.
pub const SUMMARIZE_PROMPT_FULL: &str = r#"你是一个专业的对话摘要助手。请分析以下对话历史,并按照以下9个部分生成结构化摘要。

## 输出格式
请用中文输出,使用以下9个章节(标题保持中文):

# 1. 主要请求和意图
# 2. 关键技术概念
# 3. 文件和代码部分
# 4. 错误和修复
# 5. 问题解决
# 6. 所有用户消息
# 7. 待处理任务
# 8. 当前工作
# 9. 可选的下一步

请将完整的摘要包裹在 <summary>...</summary> 标签中,例如:
<summary>
# 1. 主要请求和意图
...
</summary>

## 对话历史
{{ conversation }}
"#;

/// Prompt for incremental summarization (when a previous summary exists).
pub const SUMMARIZE_PROMPT_RECENT: &str = r#"你是一个专业的对话摘要助手。请基于之前的摘要和最近的新对话,生成更新后的摘要。

## 之前的摘要
<previous-summary>
{{ previous_summary }}
</previous-summary>

## 最近的对话
{{ recent_conversation }}

## 输出格式
请用中文输出9个章节(同之前格式),包裹在 <summary>...</summary> 标签中。
"#;

/// Errors from summarization.
#[derive(Debug, Error)]
pub enum SummarizerError {
    /// LLM call failed.
    #[error("summarizer llm error: {0}")]
    Llm(String),
    /// Timeout.
    #[error("summarizer timeout after {0:?}")]
    Timeout(Duration),
    /// Stream ended without `<summary>` tag.
    #[error("summarizer output missing <summary> tag")]
    NoSummaryTag,
    /// Cancelled.
    #[error("summarizer cancelled")]
    Cancelled,
}

/// Abstract summarizer. Implementations may use any backend (LLM, mock,
/// fixture file).
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// Summarize the full conversation.
    async fn summarize_full(&self, msgs: &[ChatMessage]) -> Result<String, SummarizerError>;

    /// Summarize a recent slice, given the previous summary. Falls back
    /// to summarize_full if the implementation does not support
    /// incremental mode.
    async fn summarize_recent(
        &self,
        msgs: &[ChatMessage],
        previous_summary: Option<&str>,
    ) -> Result<String, SummarizerError>;
}

/// LLM-backed summarizer. Streams the model response, concatenates
/// `ContentDelta` events, and parses out the `<summary>...</summary>`。
///
/// v1.0 多 Provider 路由:持 `SharedModelRegistry` + `Arc<RoutingPolicy>`,
/// `call_summarizer` 内部走 `Role::Compact` slot,失败时由
/// `ModelRegistry::next_for` 在 pool 内自动切下一个 credential。
pub struct LlmSummarizer {
    registry: SharedModelRegistry,
    policy: Arc<RoutingPolicy>,
    /// 初始 spec 起点(由 `Role::Compact` slot primary 提供)。
    model_name: String,
    timeout: Duration,
}

impl std::fmt::Debug for LlmSummarizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmSummarizer")
            .field("model_name", &self.model_name)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl LlmSummarizer {
    /// v1.0 多 Provider 路由:`registry` + `policy` 注入,初次调用按
    /// `Role::Compact` slot primary 派位 client。失败时由 registry
    /// 自动在 pool 内切换下一个 credential。
    pub fn new(
        registry: SharedModelRegistry,
        policy: Arc<RoutingPolicy>,
        model_name: impl Into<String>,
    ) -> Self {
        Self {
            registry,
            policy,
            model_name: model_name.into(),
            timeout: SUMMARIZE_TIMEOUT,
        }
    }

    /// 旧 API(单 client,无 failover)—— 仅留作 `LlmSummarizer::with_registry`
    /// 缺省值的兼容 stub,实际生产代码应调 `new(registry, policy, model)`。
    #[deprecated(note = "use new(registry, policy, model) for multi-credential failover")]
    pub fn with_client(client: Arc<dyn ModelClient>, model_name: impl Into<String>) -> Self {
        // 构造一个单 entry "default" pool,policy 走 default;无 failover 能力。
        let registry = SharedModelRegistry::default();
        registry.register_pool(
            "compact",
            reflect_llm::CredentialPool {
                entries: vec![reflect_llm::PoolEntry {
                    client,
                    label: "default".into(),
                    weight: 1,
                }],
            },
        );
        Self {
            registry,
            policy: Arc::new(RoutingPolicy::default()),
            model_name: model_name.into(),
            timeout: SUMMARIZE_TIMEOUT,
        }
    }

    /// 显式 timeout(给测试或自定义超时用)。
    pub fn with_timeout(
        registry: SharedModelRegistry,
        policy: Arc<RoutingPolicy>,
        model_name: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            registry,
            policy,
            model_name: model_name.into(),
            timeout,
        }
    }
}

#[async_trait]
impl Summarizer for LlmSummarizer {
    async fn summarize_full(&self, msgs: &[ChatMessage]) -> Result<String, SummarizerError> {
        let conversation = serialize_messages(msgs);
        let prompt = SUMMARIZE_PROMPT_FULL.replace("{{ conversation }}", &conversation);
        call_summarizer(self, &prompt, None).await
    }

    async fn summarize_recent(
        &self,
        msgs: &[ChatMessage],
        previous_summary: Option<&str>,
    ) -> Result<String, SummarizerError> {
        let recent = serialize_messages(msgs);
        let prompt = match previous_summary {
            Some(prev) if !prev.is_empty() => SUMMARIZE_PROMPT_RECENT
                .replace("{{ previous_summary }}", prev)
                .replace("{{ recent_conversation }}", &recent),
            _ => {
                // No previous summary → behave like full.
                let full = SUMMARIZE_PROMPT_FULL.replace("{{ conversation }}", &recent);
                return call_summarizer(self, &full, None).await;
            }
        };
        call_summarizer(self, &prompt, Some(previous_summary.unwrap_or(""))).await
    }
}

async fn call_summarizer(
    s: &LlmSummarizer,
    prompt: &str,
    _previous: Option<&str>,
) -> Result<String, SummarizerError> {
    let request = ChatRequest {
        model: s.model_name.clone(),
        system: SystemBlocks::default(),
        messages: vec![ChatMessage::User(UserContent {
            blocks: vec![ContentBlock::text(prompt)],
        })],
        tools: vec![ToolSpec::Function {
            name: "noop".into(),
            description: "no-op tool to satisfy tool-use-only providers".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        }],
        ..Default::default()
    };
    let cancel = CancellationToken::new();

    // v1.0 多 Provider 路由:从 `Role::Compact` slot 拿初始 spec,
    // 失败时由 `next_for` 在 pool 内切下一个 credential。简化版的
    // failover 循环:`max_attempts = candidates * 2` 之内反复试,
    // 速率限制 / Auth / 5xx 直接 mark_cooldown 切下一个。
    let initial_spec = s.policy.resolve(Role::Compact).primary.clone();
    let spec = if initial_spec.is_empty() {
        s.model_name.clone()
    } else {
        initial_spec
    };
    let mut exclude: Vec<Arc<dyn ModelClient>> = Vec::new();
    let max_attempts = s.policy.max_attempts;
    let mut attempt: u32 = 0;
    let stream = loop {
        attempt += 1;
        if attempt > max_attempts {
            return Err(SummarizerError::Llm("all credentials exhausted".into()));
        }
        let nc = match s.registry.next_for(&spec, &exclude) {
            Some(nc) => nc,
            None => return Err(SummarizerError::Llm("no credential available".into())),
        };
        let client = nc.client.clone();
        let label = nc.label.clone();
        let provider = client.name().to_string();
        let req = ChatRequest {
            model: spec.clone(),
            ..request.clone()
        };
        let stream_fut = client.stream(req, cancel.clone());
        match timeout(s.timeout, stream_fut).await {
            Ok(Ok(stream)) => {
                s.registry.clear_cooldown(&provider, &label);
                break stream;
            }
            Ok(Err(e)) => {
                use reflect_llm::{CooldownReason, LlmError};
                let cooldown = match &e {
                    LlmError::Auth => Duration::from_secs(3600),
                    LlmError::RateLimited { retry_after_ms } => {
                        Duration::from_millis(*retry_after_ms)
                            .max(s.policy.default_cooldown_rate_limited)
                    }
                    LlmError::Overloaded { retry_after_ms } => {
                        Duration::from_millis(*retry_after_ms)
                    }
                    LlmError::Provider { status, .. } if *status >= 500 => Duration::from_secs(60),
                    LlmError::ContextLengthExceeded { .. } | LlmError::InvalidRequest { .. } => {
                        // 客户端错误,不重试
                        return Err(SummarizerError::Llm(e.to_string()));
                    }
                    _ => Duration::from_secs(0), // 瞬时网络/解析错误,试下一个
                };
                if !cooldown.is_zero() {
                    let reason = match &e {
                        LlmError::Auth => CooldownReason::Auth,
                        LlmError::RateLimited { retry_after_ms } => CooldownReason::RateLimited {
                            retry_after_ms: *retry_after_ms,
                        },
                        LlmError::Overloaded { .. } => CooldownReason::Overloaded,
                        LlmError::Provider { status, .. } => {
                            CooldownReason::Provider5xx { status: *status }
                        }
                        _ => CooldownReason::Auth,
                    };
                    s.registry
                        .mark_cooldown(&provider, &label, cooldown, reason);
                }
                exclude.push(client);
            }
            Err(_) => {
                // 单次 timeout → 切下一个 credential,不 mark_cooldown
                // (timeout 不代表 credential 不可用,可能只是网络抖动)。
                exclude.push(client);
            }
        }
    };
    let mut s_pin = std::pin::pin!(stream);
    let mut buf = String::new();
    let mut cancelled = false;
    loop {
        let evt = match s_pin.next().await {
            Some(Ok(e)) => e,
            Some(Err(e)) => {
                if cancelled {
                    return Err(SummarizerError::Cancelled);
                }
                return Err(SummarizerError::Llm(e.to_string()));
            }
            None => break,
        };
        if cancel.is_cancelled() {
            cancelled = true;
            continue;
        }
        match evt {
            ChatEvent::ContentDelta(d) => buf.push_str(&d),
            ChatEvent::ThinkingDelta(d) => buf.push_str(&d), // tolerate
            ChatEvent::MessageStop => break,
            ChatEvent::Error(e) => return Err(SummarizerError::Llm(e.to_string())),
            _ => {}
        }
    }
    if cancelled {
        return Err(SummarizerError::Cancelled);
    }
    parse_summary(&buf).ok_or(SummarizerError::NoSummaryTag)
}

/// Serialize messages to a plain-text transcript the LLM can read.
/// Caps at ~200k chars to avoid pathological inputs.
pub fn serialize_messages(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    let mut total = 0usize;
    for m in messages {
        let s = match m {
            ChatMessage::System(s) => format!("[System]\n{s}\n"),
            ChatMessage::User(u) => {
                let mut s = String::from("[User]\n");
                for b in &u.blocks {
                    match b {
                        ContentBlock::Text { text } => s.push_str(text),
                        ContentBlock::Image { .. } => s.push_str("[image]"),
                    }
                }
                s.push('\n');
                s
            }
            ChatMessage::Assistant(a) => {
                let mut s = String::from("[Assistant]\n");
                if let Some(text) = &a.text {
                    s.push_str(text);
                }
                if !a.tool_calls.is_empty() {
                    s.push_str(&format!(
                        "\n[Tools: {}]\n",
                        a.tool_calls
                            .iter()
                            .map(|t| t.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                s.push('\n');
                s
            }
            ChatMessage::Tool(t) => format!("[Tool({})]\n{}\n", t.call_id, t.content),
        };
        if total + s.len() > 200_000 {
            out.push_str("...[truncated for length]...");
            break;
        }
        out.push_str(&s);
        out.push_str("\n---\n");
        total += s.len();
    }
    out
}

/// Extract `<summary>...</summary>` content from raw LLM output. Returns
/// `None` if no tags are present.
pub fn parse_summary(raw: &str) -> Option<String> {
    let start = raw.find("<summary>")?;
    let after = start + "<summary>".len();
    let end = raw[after..].find("</summary>")?;
    Some(raw[after..after + end].trim().to_string())
}

/// Wrap a summary text in `<summary>` tags (for `summarize_recent`
/// callers that need to feed it back into a future call).
pub fn wrap_summary(text: &str) -> String {
    format!("<summary>\n{text}\n</summary>")
}

// Re-export the public trait + struct via crate root; the imports
// above only need the type name to be in scope.
#[allow(unused_imports)]
use AssistantContent as _AssistantContent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_summary_extracts_content() {
        let raw = "<analysis>blah</analysis><summary>hello world</summary>extra";
        assert_eq!(parse_summary(raw).as_deref(), Some("hello world"));
    }

    #[test]
    fn parse_summary_trims_whitespace() {
        let raw = "<summary>\n  hello\n  world  \n</summary>";
        assert_eq!(parse_summary(raw).as_deref(), Some("hello\n  world"));
    }

    #[test]
    fn parse_summary_returns_none_when_no_tag() {
        assert!(parse_summary("no tags here").is_none());
    }

    #[test]
    fn parse_summary_returns_none_when_unclosed() {
        assert!(parse_summary("<summary>unfinished").is_none());
    }

    #[test]
    fn wrap_summary_roundtrips() {
        let wrapped = wrap_summary("hi");
        assert_eq!(parse_summary(&wrapped).as_deref(), Some("hi"));
    }

    #[test]
    fn serialize_messages_includes_all_roles() {
        let msgs = vec![
            ChatMessage::System("sys".into()),
            ChatMessage::User(UserContent {
                blocks: vec![ContentBlock::text("hi")],
            }),
            ChatMessage::Assistant(AssistantContent {
                text: Some("hello".into()),
                tool_calls: vec![],
                thinking: None,
            }),
            ChatMessage::Tool(reflect_llm::ToolResult {
                call_id: "c1".into(),
                content: "ok".into(),
                is_error: false,
            }),
        ];
        let s = serialize_messages(&msgs);
        assert!(s.contains("[System]"));
        assert!(s.contains("[User]"));
        assert!(s.contains("[Assistant]"));
        assert!(s.contains("[Tool(c1)]"));
    }

    #[test]
    fn serialize_messages_truncates_at_200k() {
        let huge = "x".repeat(300_000);
        let msgs = vec![ChatMessage::System(huge)];
        let s = serialize_messages(&msgs);
        assert!(s.contains("[truncated for length]"));
        assert!(s.len() < 210_000);
    }

    #[test]
    fn summarize_prompts_have_placeholders() {
        assert!(SUMMARIZE_PROMPT_FULL.contains("{{ conversation }}"));
        assert!(SUMMARIZE_PROMPT_RECENT.contains("{{ previous_summary }}"));
        assert!(SUMMARIZE_PROMPT_RECENT.contains("{{ recent_conversation }}"));
    }
}
