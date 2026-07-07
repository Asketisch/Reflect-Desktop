//! OpenAI Responses API stub —— 与 Chat Completions 并行的 endpoint 占位。
//!
//! 真实 HTTP 实现留 v2.x;当前提供 config + trait 包装,供 builder 切换。

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::capabilities::Capabilities;
use crate::client::ModelClient;
use crate::error::LlmError;
use crate::event::ChatEvent;
use crate::request::ChatRequest;

/// Responses API 配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponsesConfig {
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// 模型 id,如 `gpt-4o`。
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_timeout_secs() -> u64 {
    60
}

fn default_model() -> String {
    "gpt-4o".into()
}

impl Default for OpenAIResponsesConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: None,
            timeout_secs: default_timeout_secs(),
            model: default_model(),
        }
    }
}

/// OpenAI Responses API stub client。
///
/// `chat_stream` 返回单条 `ChatEvent::Error`,提示尚未实现;
/// 不消耗 API quota。
pub struct OpenAIResponsesClient {
    config: OpenAIResponsesConfig,
    #[allow(dead_code)]
    http: Client,
}

impl OpenAIResponsesClient {
    pub fn new(config: OpenAIResponsesConfig) -> Result<Self, LlmError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(LlmError::from)?;
        Ok(Self { config, http })
    }

    pub fn endpoint_path(&self) -> &'static str {
        "/v1/responses"
    }

    pub fn is_stub(&self) -> bool {
        true
    }

    pub fn stub_message(&self) -> String {
        format!(
            "OpenAI Responses API stub: model={} endpoint={} — 真实 streaming 留 v2.x",
            self.config.model,
            self.endpoint_path()
        )
    }
}

#[async_trait]
impl ModelClient for OpenAIResponsesClient {
    fn name(&self) -> &str {
        "openai-responses"
    }

    fn provider_kind(&self) -> crate::ProviderKind {
        crate::ProviderKind::OpenAI
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tool_use: true,
            vision: true,
            json_mode: true,
            ..Capabilities::default()
        }
    }

    async fn stream(
        &self,
        _request: ChatRequest,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError> {
        let msg = self.stub_message();
        let stream =
            futures::stream::once(async move { Ok(ChatEvent::Error(LlmError::Internal(msg))) });
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_message_mentions_responses() {
        let c = OpenAIResponsesClient::new(OpenAIResponsesConfig::default()).unwrap();
        assert!(c.stub_message().contains("Responses"));
    }

    #[tokio::test]
    async fn stream_returns_error_event() {
        use futures::StreamExt;
        let c = OpenAIResponsesClient::new(OpenAIResponsesConfig::default()).unwrap();
        let cancel = tokio_util::sync::CancellationToken::new();
        let req = ChatRequest::default();
        let mut s = c.stream(req, cancel).await.unwrap();
        let ev = s.next().await.unwrap().unwrap();
        assert!(matches!(ev, ChatEvent::Error(_)));
    }
}
