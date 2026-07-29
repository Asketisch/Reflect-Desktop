//! Side-channel agent orchestration.
//!
//! A **side-channel** is a user-driven concurrent agent run independent of the
//! main agent's turn lifecycle. Concept reference: thClaws
//! `crates/core/src/side_channel.rs` (M6.34+).
//!
//! ## Differences from `Task` tool (model-driven subagent, `reflect-subagent`)
//!
//! | | Side-channel | Subagent (`Task`) |
//! |---|---|---|
//! | Trigger | User types `/agent <name> <prompt>` | Model calls `Task` tool |
//! | Concurrency | Runs concurrently with main agent | Blocks main's turn |
//! | Main's history | Not affected | Tool result lands in main's history |
//! | Cancel | Independent `CancelToken` — main `Cmd-C` does NOT kill it | Inherits parent's cancel |
//! | UI | `chat_side_channel_*` events on a dedicated tab | Single `Task` tool indicator |
//!
//! ## Registry shape
//!
//! This module ships a `SideChannelRegistry` (process-level) plus a lightweight
//! `SideChannelHandle` (per-run). The registry maps stable ids
//! (`side-<8-hex>`) to handles. Handles own their own `CancelToken`, so
//! `main`'s cancel token never reaches a side-channel.
//!
//! ## View-side emission
//!
//! The GUI layer can subscribe via `subscribe_events()` — a `broadcast::Receiver`
//! that yields `SideChannelEvent` per state change (`Started` / `Output` /
//! `Done` / `Error` / `Cancelled`). The current desktop Tauri shell wires the
//! receiver through the existing session-event forwarder so the frontend gets
//! the same `reflect_event` channel it already consumes.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Stable handle identifier (`side-<8 hex>`).
pub type SideChannelId = String;

/// Lifecycle status of a side-channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SideChannelStatus {
    /// Side-channel is currently executing.
    Running,
    /// Side-channel completed naturally (success or empty result).
    Done,
    /// Side-channel was cancelled by user.
    Cancelled,
    /// Side-channel exited with an error.
    Error,
}

impl SideChannelStatus {
    /// `"running"` / `"done"` / `"cancelled"` / `"error"`.
    pub fn as_str(self) -> &'static str {
        match self {
            SideChannelStatus::Running => "running",
            SideChannelStatus::Done => "done",
            SideChannelStatus::Cancelled => "cancelled",
            SideChannelStatus::Error => "error",
        }
    }
}

/// One entry in the registry's event stream.
///
/// Sent to all subscribers when state changes (Started / Done / Cancelled /
/// Error). `Output` events are emitted by the runner task as the side-channel
/// produces content (currently emitted at done time only — see `run_once`
/// notes; future work streams incremental output).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SideChannelEvent {
    /// Side-channel started; carries its id and the agent name it picked.
    Started {
        /// Side-channel id (`side-<8hex>`).
        id: SideChannelId,
        /// Agent definition name chosen by the user.
        agent_name: String,
        /// Initial prompt submitted to the side-channel.
        prompt: String,
        /// UNIX epoch milliseconds when the side-channel was registered.
        started_at_ms: i64,
    },
    /// Side-channel finished cleanly. Optional short `summary` line for the
    /// chats UI to render in the channel header.
    Done {
        /// Side-channel id.
        id: SideChannelId,
        /// UNIX epoch milliseconds when the run finished.
        finished_at_ms: i64,
        /// Optional summary line (last assistant message or empty).
        summary: Option<String>,
    },
    /// Side-channel was cancelled (`reflect_cancel_side_channel` or its own
    /// token fired).
    Cancelled {
        /// Side-channel id.
        id: SideChannelId,
        /// UNIX epoch milliseconds when cancellation took effect.
        finished_at_ms: i64,
    },
    /// Side-channel exited with an error (message preserved).
    Error {
        /// Side-channel id.
        id: SideChannelId,
        /// UNIX epoch milliseconds when the error was recorded.
        finished_at_ms: i64,
        /// Human-readable error message.
        message: String,
    },
}

/// Public-view snapshot of a side-channel (sent to the frontend / IPC).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideChannelInfo {
    /// Stable id.
    pub id: SideChannelId,
    /// Agent definition name.
    pub agent_name: String,
    /// Initial prompt (truncated by snapshot size, not by storage).
    pub prompt: String,
    /// UNIX epoch milliseconds when the side-channel was registered.
    pub started_at_ms: i64,
    /// Current status (`"running"` / `"done"` / `"cancelled"` / `"error"`).
    pub status: String,
    /// Duration in milliseconds; `null` while still running.
    pub duration_ms: Option<i64>,
}

/// Per-side-channel state held in the registry. Cloned for IPC snapshots.
pub(crate) struct SideChannelHandle {
    /// Agent definition name.
    pub agent_name: String,
    /// Initial prompt.
    pub prompt: String,
    /// Monotonic start time for duration math.
    pub started_at: Instant,
    /// Wall-clock start time (UNIX_EPOCH ms) for IPC serialization. Computed
    /// once on creation so the frontend gets a stable epoch timestamp even if
    /// it snapshots long after the run finished.
    pub started_at_epoch_ms: i64,
    /// Current status.
    pub status: SideChannelStatus,
    /// Independent cancel token (NOT a child of main's). Caller may fire
    /// `cancel.cancel()` to stop just this side-channel.
    pub cancel: CancellationToken,
}

impl SideChannelHandle {
    fn snapshot(&self, id: &str) -> SideChannelInfo {
        let duration_ms = if matches!(self.status, SideChannelStatus::Running) {
            None
        } else {
            Some(self.started_at.elapsed().as_millis() as i64)
        };
        SideChannelInfo {
            id: id.to_string(),
            agent_name: self.agent_name.clone(),
            prompt: self.prompt.clone(),
            started_at_ms: self.started_at_epoch_ms,
            status: self.status.as_str().to_string(),
            duration_ms,
        }
    }
}

/// Process-level registry of all side-channels.
#[derive(Clone)]
pub struct SideChannelRegistry {
    inner: Arc<Mutex<HashMap<SideChannelId, SideChannelHandle>>>,
    events: broadcast::Sender<SideChannelEvent>,
    /// Process-wide counter to guarantee unique ids even when two starts
    /// happen in the same wall-clock millisecond. Pairs with the timestamp
    /// portion of the id so we can still derive ordering from `started_at_ms`.
    next_seq: Arc<AtomicU32>,
}

impl Default for SideChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SideChannelRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SideChannelRegistry")
            .field("count", &self.inner.lock().len())
            .finish_non_exhaustive()
    }
}

impl SideChannelRegistry {
    /// Construct a new registry with default broadcast capacity.
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// Construct a registry with the given broadcast channel capacity.
    pub fn with_capacity(cap: usize) -> Self {
        let (events, _) = broadcast::channel(cap);
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            events,
            next_seq: Arc::new(AtomicU32::new(1)),
        }
    }

    /// Subscribe to the event stream.
    pub fn subscribe_events(&self) -> broadcast::Receiver<SideChannelEvent> {
        self.events.subscribe()
    }

    /// Insert a new side-channel. Returns the assigned id and a cancel handle
    /// the caller must hold for cancellation. Emits a `Started` event.
    pub fn start(
        &self,
        agent_name: String,
        prompt: String,
    ) -> (SideChannelId, CancellationToken) {
        // Compute a fresh id; on collision, retry with a process-unique seq.
        let epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let mut id = format!("side-{:08x}", (epoch_ms as u32).wrapping_mul(2654435761));
        let mut handle = SideChannelHandle {
            agent_name: agent_name.clone(),
            prompt: prompt.clone(),
            started_at: Instant::now(),
            started_at_epoch_ms: epoch_ms,
            status: SideChannelStatus::Running,
            cancel: CancellationToken::new(),
        };
        let cancel = handle.cancel.clone();
        let mut g = self.inner.lock();
        if g.contains_key(&id) {
            // Rare: two starts in the same ms produced the same hash. Salt
            // with the next sequence number (process-unique, monotonic) so
            // we never collide regardless of how tight the timing is.
            let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
            id = format!("side-{:08x}", (epoch_ms as u32).wrapping_add(seq));
            handle.started_at_epoch_ms += seq as i64;
        } else {
            // Normal path still bumps the seq so concurrent collisions see
            // strictly-increasing salts on their retry.
            self.next_seq.fetch_add(1, Ordering::Relaxed);
        }
        g.insert(id.clone(), handle);
        drop(g);
        let _ = self.events.send(SideChannelEvent::Started {
            id: id.clone(),
            agent_name,
            prompt,
            started_at_ms: epoch_ms,
        });
        (id, cancel)
    }

    /// Cancel a side-channel by id. Fires the cancel token and updates
    /// status. Returns `true` if a side-channel was found and cancelled.
    pub fn cancel(&self, id: &str) -> bool {
        let (started_at_epoch_ms,) = {
            let mut g = self.inner.lock();
            match g.get_mut(id) {
                Some(h) if matches!(h.status, SideChannelStatus::Running) => {
                    h.cancel.cancel();
                    h.status = SideChannelStatus::Cancelled;
                    (Some(h.started_at_epoch_ms),)
                }
                _ => (None,),
            }
        };
        if let Some(started_epoch) = started_at_epoch_ms {
            let finished_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(started_epoch);
            let _ = self.events.send(SideChannelEvent::Cancelled {
                id: id.to_string(),
                finished_at_ms: finished_at,
            });
            true
        } else {
            false
        }
    }

    /// Mark a side-channel as done (`Done` or `Error` based on result).
    /// Called by the runner task when the underlying work completes.
    pub fn finish(&self, id: &str, status: SideChannelStatus, summary: Option<String>) {
        let (started_at_epoch_ms,) = {
            let mut g = self.inner.lock();
            match g.get_mut(id) {
                Some(h) => {
                    h.status = status;
                    (Some(h.started_at_epoch_ms),)
                }
                None => (None,),
            }
        };
        if let Some(_started_epoch) = started_at_epoch_ms {
            let finished_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            let event = match status {
                SideChannelStatus::Done => SideChannelEvent::Done {
                    id: id.to_string(),
                    finished_at_ms,
                    summary,
                },
                SideChannelStatus::Error => SideChannelEvent::Error {
                    id: id.to_string(),
                    finished_at_ms,
                    message: summary.unwrap_or_default(),
                },
                SideChannelStatus::Cancelled => SideChannelEvent::Cancelled {
                    id: id.to_string(),
                    finished_at_ms,
                },
                SideChannelStatus::Running => SideChannelEvent::Done {
                    id: id.to_string(),
                    finished_at_ms,
                    summary,
                },
            };
            let _ = self.events.send(event);
        }
    }

    /// List all side-channels (snapshot). Sorted by started_at ascending so
    /// the UI shows oldest first.
    pub fn list(&self) -> Vec<SideChannelInfo> {
        let g = self.inner.lock();
        let mut v: Vec<(SideChannelId, SideChannelInfo)> = g
            .iter()
            .map(|(id, h)| (id.clone(), h.snapshot(id)))
            .collect();
        v.sort_by(|a, b| a.1.started_at_ms.cmp(&b.1.started_at_ms));
        v.into_iter().map(|(_, info)| info).collect()
    }

    /// Lookup a single side-channel by id.
    pub fn get(&self, id: &str) -> Option<SideChannelInfo> {
        let g = self.inner.lock();
        g.get(id).map(|h| h.snapshot(id))
    }

    /// Number of currently-running side-channels.
    pub fn running_count(&self) -> usize {
        self.inner
            .lock()
            .values()
            .filter(|h| matches!(h.status, SideChannelStatus::Running))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn reg() -> Arc<SideChannelRegistry> {
        Arc::new(SideChannelRegistry::new())
    }

    #[test]
    fn start_yields_id_and_cancel_token() {
        let r = reg();
        let (id, cancel) = r.start("default".into(), "hello".into());
        assert!(id.starts_with("side-"));
        assert_eq!(id.len(), 13); // "side-" + 8 hex
        assert!(!cancel.is_cancelled());
        assert_eq!(r.list().len(), 1);
        assert_eq!(r.running_count(), 1);
    }

    #[test]
    fn cancel_marks_handles_and_emits_event() {
        let r = reg();
        let mut sub = r.subscribe_events();
        let (id, token) = r.start("default".into(), "x".into());
        assert!(r.cancel(&id), "should cancel running side-channel");
        // Cancel token fired
        assert!(token.is_cancelled());
        let status = r.get(&id).unwrap().status;
        assert_eq!(status, "cancelled");
        assert_eq!(r.running_count(), 0);
        // Drain the channel: Started + Cancelled
        let evt1 = sub.try_recv().expect("started event");
        assert!(matches!(evt1, SideChannelEvent::Started { .. }));
        let evt2 = sub.try_recv().expect("cancelled event");
        assert!(matches!(evt2, SideChannelEvent::Cancelled { .. }));
    }

    #[test]
    fn cancel_on_already_terminal_is_noop() {
        let r = reg();
        let (id, _) = r.start("default".into(), "x".into());
        r.finish(&id, SideChannelStatus::Done, Some("ok".into()));
        // Already terminal; cancel returns false.
        assert!(!r.cancel(&id));
    }

    #[test]
    fn finish_done_emits_done_event() {
        let r = reg();
        let mut sub = r.subscribe_events();
        let (id, _) = r.start("default".into(), "hi".into());
        r.finish(&id, SideChannelStatus::Done, Some("bye".into()));
        // Drain Started
        let _ = sub.try_recv();
        let evt = sub.try_recv().expect("done event");
        match evt {
            SideChannelEvent::Done { summary, .. } => {
                assert_eq!(summary.as_deref(), Some("bye"));
            }
            _ => panic!("expected Done event"),
        }
    }

    #[test]
    fn list_snapshots_independent_cancels() {
        let r = reg();
        let (id1, t1) = r.start("default".into(), "x".into());
        let (id2, t2) = r.start("default".into(), "y".into());
        assert_ne!(id1, id2);
        // Cancelling id1 must not affect id2.
        r.cancel(&id1);
        assert!(t1.is_cancelled());
        assert!(!t2.is_cancelled());
    }

    #[test]
    fn cancel_does_not_block_future_starts() {
        let r = reg();
        // Stress a tiny bit to confirm the registry isn't permanently
        // wedged by cancelled entries (smoke check; no correctness leak).
        for i in 0..20 {
            let (id, _) = r.start("default".into(), format!("p-{i}"));
            r.cancel(&id);
        }
        assert_eq!(r.running_count(), 0);
        // After a quick sleep, registry can still emit events on new subs.
        std::thread::sleep(Duration::from_millis(1));
        let mut sub = r.subscribe_events();
        let _ = r.start("default".into(), "fresh".into());
        let evt = sub.try_recv().expect("started event");
        assert!(matches!(evt, SideChannelEvent::Started { .. }));
    }
}
