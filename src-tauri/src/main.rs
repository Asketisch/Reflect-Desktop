//! `reflect-gui` Tauri 2 binary entry.
//!
//! M1.1：仅占位，确保 `pnpm tauri dev` 能拉起一个空白窗口（无 IPC commands）。
//! 实际 command 列表与 event 转发在 M1.2 协议桥里程碑实现。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    reflect_desktop_lib::run();
}
