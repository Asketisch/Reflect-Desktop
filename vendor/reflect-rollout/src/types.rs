//! Compile-time constants for the rollout pipeline.
//!
//! Centralised so the writer, redaction layer, and tests share the same
//! numbers — no magic `256 * 1024` sprinkled across modules.

/// Above this byte size, the active JSONL file is rotated. Matches claw.
pub const ROTATE_AFTER_BYTES: u64 = 256 * 1024;

/// At most this many rotated copies are kept (`foo.1.jsonl`, `foo.2.jsonl`,
/// `foo.3.jsonl`). Older copies are deleted. Matches claw.
pub const MAX_ROTATED_FILES: usize = 3;

/// Above this character count, a single JSON string is truncated and tagged
/// with [`JSONL_REDACTION_MARKER`]. Matches claw.
pub const MAX_JSONL_FIELD_CHARS: usize = 16 * 1024;

/// Suffix appended to redacted string fields.
pub const JSONL_REDACTION_MARKER: &str = "[redacted]";

/// Default base directory under `$HOME` if the caller does not override.
pub const DEFAULT_ROLLOUT_DIR: &str = ".reflect/sessions";
