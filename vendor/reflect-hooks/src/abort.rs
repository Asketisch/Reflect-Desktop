//! `HookAbortSignal` — cooperative cancellation for hook + tool execution.
//!
//! Uses an `Arc<AtomicBool>` design (see
//! `docs/tools-and-hooks.md §4.5`). The owning thread (e.g. `AgentThread`
//! on Ctrl-C) calls [`HookAbortSignal::trigger`]; the `ToolExecutionQueue`
//! polls [`HookAbortSignal::is_triggered`] between hook dispatches and
//! before each tool call.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Cheap-to-clone cancellation flag for the entire hook+tool pipeline.
#[derive(Debug, Clone, Default)]
pub struct HookAbortSignal {
    inner: Arc<AtomicBool>,
}

impl HookAbortSignal {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Mark the signal as triggered. Idempotent.
    pub fn trigger(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    /// Returns `true` once [`trigger`](Self::trigger) has been called.
    pub fn is_triggered(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    /// Reset to untriggered (used between turns; usually not needed).
    pub fn reset(&self) {
        self.inner.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_untriggered() {
        let s = HookAbortSignal::new();
        assert!(!s.is_triggered());
    }

    #[test]
    fn trigger_sets_flag() {
        let s = HookAbortSignal::new();
        s.trigger();
        assert!(s.is_triggered());
    }

    #[test]
    fn clones_share_state() {
        let s = HookAbortSignal::new();
        let c = s.clone();
        c.trigger();
        assert!(s.is_triggered());
    }

    #[test]
    fn reset_clears_flag() {
        let s = HookAbortSignal::new();
        s.trigger();
        s.reset();
        assert!(!s.is_triggered());
    }
}
