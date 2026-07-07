//! `reflect-sandbox` — eval / hindsight / OS 沙箱扩展。
//!
//! v1.2 P0-1:`os_sandbox` 从 stub 升级为真实沙箱(macOS Seatbelt +
//! Linux Landlock)。

pub mod eval_cells;
pub mod hindsight_memory;
pub mod os_sandbox;

pub use eval_cells::{EvalCellStore, EvalCellStubStatus, eval_cell_stub};
pub use hindsight_memory::{HindsightMemory, HindsightStubStatus};
pub use os_sandbox::{detect_backend, OsSandbox, OsSandboxStatus, OsSandboxStubStatus};
