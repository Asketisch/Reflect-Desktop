//! `reflect-rollout` — JSONL rollout persistence with rotation and redaction.
//!
//! Every [`reflect_protocol::RolloutRecord`] emitted by `reflect-core`
//! is appended to a per-thread file under
//! `~/.reflect/sessions/YYYY/MM/DD/<thread_id>.jsonl`. Files rotate at
//! 256 KiB and at most 3 rotated copies are kept.
//!
//! Large string fields are truncated to 16 KiB and tagged `[redacted]` so a
//! runaway tool output cannot blow up the rollout size. The redaction pass
//! is uniform across record types — we don't maintain a key-name blacklist
//! because provider schemas differ.
//!
//! Resume support is history-only: [`replay::replay`] returns every
//! [`reflect_protocol::RolloutRecord`] for a thread, leaving the caller
//! (`reflect-exec::bootstrap_resume`) to drop in-flight tool pairs and
//! inject a synthetic `<system-reminder>resumed session</system-reminder>`.

pub mod export;
pub mod index;
pub mod path;
pub mod reader;
pub mod redact;
pub mod types;
pub mod writer;

pub use export::{MAX_MARKDOWN_CHARS, to_markdown};
pub use reader::{Reader, replay, replay_path};
pub use writer::JsonlRolloutWriter;

pub use reflect_protocol::{
    MessageRole, NullRecorder, RolloutRecord, RolloutRecorder, SessionInfo,
};
