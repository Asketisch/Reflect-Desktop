//! LLM error type — what providers can return. Classification to retry vs
//! fail-fast happens in `reflect-core::submission_loop`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmError {
    /// HTTP-level transport error.
    #[error("http: {0}")]
    Http(String),

    /// SSE protocol-level parse error.
    #[error("sse parse: {0}")]
    SseParse(String),

    /// 401 / invalid API key.
    #[error("auth failed")]
    Auth,

    /// 429 with `Retry-After` (in ms).
    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    /// 400 + body matches `context_length_exceeded` / `prompt is too long`.
    #[error("context length exceeded: used {used} > {limit}")]
    ContextLengthExceeded { used: u32, limit: u32 },

    /// 400 (non-context-length).
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    /// 529 (Anthropic overloaded).
    #[error("provider overloaded, retry after {retry_after_ms}ms")]
    Overloaded { retry_after_ms: u64 },

    /// 5xx with body.
    #[error("provider error: {status} {message}")]
    Provider { status: u16, message: String },

    /// CancellationToken fired.
    #[error("cancelled")]
    Cancelled,

    /// Catch-all.
    #[error("internal: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        LlmError::Http(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_useful() {
        assert_eq!(LlmError::Auth.to_string(), "auth failed");
        assert_eq!(
            LlmError::RateLimited {
                retry_after_ms: 1500
            }
            .to_string(),
            "rate limited, retry after 1500ms"
        );
        assert_eq!(
            LlmError::ContextLengthExceeded {
                used: 200_000,
                limit: 100_000
            }
            .to_string(),
            "context length exceeded: used 200000 > 100000"
        );
    }
}
