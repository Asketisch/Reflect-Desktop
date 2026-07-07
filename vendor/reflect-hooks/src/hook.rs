//! `Hook` trait + `HookError`.

use async_trait::async_trait;
use thiserror::Error;

use crate::decision::HookDecision;
use crate::event::{HookEvent, HookEventKind};

/// A user-extensible interception point in the agent loop.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Stable identifier (used in tracing and `HookEngine` debug output).
    fn name(&self) -> &str;

    /// One-line human description (used by `/hooks ls` TUI pill, plugin
    /// manifest, `/doctor` diagnostic output)。默认返回空串 —— 简单
    /// 内置 hook 不需要描述也能注册。
    fn description(&self) -> &str {
        ""
    }

    /// Which events this hook cares about. The engine skips hooks whose
    /// `events()` does not contain the event's kind.
    fn events(&self) -> &[HookEventKind];

    /// Handle the event. Default returns `Allow`.
    async fn handle(&self, _event: &HookEvent) -> Result<HookDecision, HookError> {
        Ok(HookDecision::Allow)
    }
}

/// Hook-level error. By default the engine treats a hook error as a `Deny`
/// (fail-closed; see `docs/tools-and-hooks.md` acceptance criteria).
#[derive(Debug, Error)]
pub enum HookError {
    #[error("{0}")]
    Other(String),
}

impl From<String> for HookError {
    fn from(s: String) -> Self {
        HookError::Other(s)
    }
}

impl From<&str> for HookError {
    fn from(s: &str) -> Self {
        HookError::Other(s.to_string())
    }
}
