//! Reflect-app-core: 跨 TUI/GUI 共享的 UI-agnostic 状态、reducer 与 Services bundle。
//!
//! 当前（M1.1）只导出占位结构。后续里程碑会从 `crates/reflect-tui/src/app.rs` 把
//! `RenderState`、`Services`、各 pending 状态机、`external_editor::run_editor_with`
//! 迁入本 crate，并由 `reflect-tui` 通过 `pub use reflect_app_core::*` 重新导出，
//! TUI 调用方完全无感。
//!
//! 关键约束（见 `docs/gui/03-architecture.md` §2.2）：
//! - 不依赖 ratatui / crossterm / tauri / iced
//! - 仅依赖 reflect-protocol + reflect-core + reflect-task 等 UI-agnostic crate
//! - 编译时间 ≤5s（`make check-fast C=reflect-app-core`）
//!
//! v1.1 设计决策见 `docs/gui/05-decision.md`。

#![deny(missing_docs)]
#![warn(unused_extern_crates)]

pub mod state;
pub mod reducer;
pub mod protocol;

/// 语义版本号（与 workspace 同步）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
