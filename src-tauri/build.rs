//! Tauri 2 build script —— 仅在 `pnpm tauri dev|build` 或 `cargo build` 时触发,
//! 生成 capability schemas 到 OUT_DIR,供 `tauri::generate_context!` 宏读取。

fn main() {
    tauri_build::build()
}
