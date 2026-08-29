//! macOS Dock badge 命令 —— 薄包装 `crate::dock` 的实现。
//!
//! 前端包装:`src/utils/commands/notifications.ts::reflect_set_dock_badge`。
//! 命令体按领域归入 `commands/`(见 AGENTS.md 命令表约定),使
//! `src/app.smoke.test.tsx` 的 IPC parity 扫描能覆盖。

use tauri::AppHandle;

/// 设置 macOS Dock badge(`None` 清空;非 macOS 为 no-op)。
#[tauri::command]
pub fn reflect_set_dock_badge(app: AppHandle, label: Option<String>) {
    crate::dock::set_dock_badge(&app, label);
}
