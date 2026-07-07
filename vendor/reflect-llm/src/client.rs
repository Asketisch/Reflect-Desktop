//! `ModelClient` trait — the abstract interface all providers implement.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use tokio_util::sync::CancellationToken;

use crate::capabilities::{Capabilities, ProviderKind};
use crate::error::LlmError;
use crate::event::ChatEvent;
use crate::request::ChatRequest;

pub type BoxedModelClient = std::sync::Arc<dyn ModelClient>;

/// Streaming-first LLM client. Implementations map a `ChatRequest` to a
/// provider-specific HTTP call, parse the SSE response, and yield `ChatEvent`s.
#[async_trait]
pub trait ModelClient: Send + Sync {
    /// Provider name (e.g. `"openai"`).
    fn name(&self) -> &str;

    /// Static capabilities advertised.
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }

    /// v1.0.0-rc2: 强类型 provider 标识。默认 `Custom` —— 让 plugin /
    /// test stub 无需 override 即可编译。3 个内置 client 各自 override
    /// 为 `Anthropic` / `OpenAI` / `Ollama`。
    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Custom
    }

    /// Stream a chat completion. Returns a `Result` to allow synchronous
    /// failure (auth, model not found). Streaming errors flow as
    /// `ChatEvent::Error` and may or may not terminate the stream.
    async fn stream(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatEvent, LlmError>> + Send>>, LlmError>;
}
