//! 应用菜单 (macOS 顶栏 / Windows 菜单栏 / Linux app menu)。
//!
//! 菜单结构：
//! - `enable_macos_default_menu(false)` 关闭 Tauri 默认菜单;
//! - 5 个 submenu: Reflect / Edit / Composer / View / Window;
//! - 每个菜单项 click → `app.emit("menu-<id>", ())` 通知前端,
//!   前端 reducer 按 id 决定后续动作 (cycle model / open settings / ...)。
//! - 4 个 inline accelerator: cycle_model / cycle_reasoning / new_agent / interrupt
//!   (M1.x → M3.x 阶段再做 13 个剩余的 settings 化快捷键)。

use tauri::{
    menu::{Menu, MenuItemBuilder, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, Manager, Runtime, WindowEvent,
};

// ====== Reflect (App) 菜单 ======
pub const MENU_ABOUT: &str = "menu_about";
pub const MENU_CHECK_UPDATES: &str = "menu_check_updates";
pub const MENU_SETTINGS: &str = "menu_settings";
pub const MENU_QUIT: &str = "menu_quit";

// ====== Edit 菜单（PredefinedMenuItem）======
// Edit 菜单全部走 PredefinedMenuItem, 不需要自定义 id.

// ====== Composer 菜单 ======
pub const MENU_CYCLE_MODEL: &str = "menu_cycle_model";
pub const MENU_CYCLE_REASONING: &str = "menu_cycle_reasoning";
pub const MENU_NEW_AGENT: &str = "menu_new_agent";
pub const MENU_INTERRUPT: &str = "menu_interrupt";

// ====== View 菜单 ======
pub const MENU_TOGGLE_SIDEBAR: &str = "menu_toggle_sidebar";
pub const MENU_TOGGLE_TERMINAL: &str = "menu_toggle_terminal";

// ====== Window 菜单 ======
pub const MENU_MINIMIZE: &str = "menu_minimize";
pub const MENU_ZOOM: &str = "menu_zoom";

/// 构建完整应用菜单。
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let reflect = build_reflect_menu(app)?;
    let edit = build_edit_menu(app)?;
    let composer = build_composer_menu(app)?;
    let view = build_view_menu(app)?;
    let window = build_window_menu(app)?;
    Menu::with_items(app, &[&reflect, &edit, &composer, &view, &window])
}

fn build_reflect_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let app_name = "Reflect";
    let about = MenuItemBuilder::with_id(MENU_ABOUT, format!("About {app_name}")).build(app)?;
    let check = MenuItemBuilder::with_id(MENU_CHECK_UPDATES, "Check for Updates…").build(app)?;
    let settings = MenuItemBuilder::with_id(MENU_SETTINGS, "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let hide = PredefinedMenuItem::hide(app, Some(app_name))?;
    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
    let show_all = PredefinedMenuItem::show_all(app, None)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = PredefinedMenuItem::quit(app, None)?;
    Submenu::with_items(
        app,
        app_name,
        true,
        &[&about, &check, &settings, &sep, &hide, &hide_others, &show_all, &sep2, &quit],
    )
}

fn build_edit_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;
    Submenu::with_items(
        app,
        "Edit",
        true,
        &[&undo, &redo, &sep, &cut, &copy, &paste, &select_all],
    )
}

fn build_composer_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let cycle_model = MenuItemBuilder::with_id(MENU_CYCLE_MODEL, "Cycle Model")
        .accelerator("CmdOrCtrl+Shift+M")
        .build(app)?;
    let cycle_reasoning = MenuItemBuilder::with_id(MENU_CYCLE_REASONING, "Cycle Reasoning Effort")
        .accelerator("CmdOrCtrl+Shift+R")
        .build(app)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let new_agent = MenuItemBuilder::with_id(MENU_NEW_AGENT, "New Agent")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let interrupt = MenuItemBuilder::with_id(MENU_INTERRUPT, "Interrupt")
        .accelerator("CmdOrCtrl+Shift+I")
        .build(app)?;
    Submenu::with_items(
        app,
        "Composer",
        true,
        &[&cycle_model, &cycle_reasoning, &sep, &new_agent, &interrupt],
    )
}

fn build_view_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let toggle_sidebar = MenuItemBuilder::with_id(MENU_TOGGLE_SIDEBAR, "Toggle Sidebar")
        .accelerator("CmdOrCtrl+B")
        .build(app)?;
    let toggle_terminal = MenuItemBuilder::with_id(MENU_TOGGLE_TERMINAL, "Toggle Terminal")
        .accelerator("CmdOrCtrl+`")
        .build(app)?;
    Submenu::with_items(app, "View", true, &[&toggle_sidebar, &toggle_terminal])
}

fn build_window_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let minimize = PredefinedMenuItem::minimize(app, None)?;
    let zoom = MenuItemBuilder::with_id(MENU_ZOOM, "Zoom").build(app)?;
    let close = PredefinedMenuItem::close_window(app, None)?;
    Submenu::with_items(app, "Window", true, &[&minimize, &zoom, &close])
}

/// 菜单 click 派发 —— Tauri builder `.on_menu_event(handle_menu_event)` 调用。
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        MENU_ABOUT => {
            // 弹出主窗口并 emit about 事件,前端决定如何展示 (modal / new window)。
            show_main_window(app);
            let _ = app.emit("menu-about", ());
        }
        MENU_CHECK_UPDATES => {
            show_main_window(app);
            let _ = app.emit("menu-check-updates", ());
        }
        MENU_SETTINGS => {
            show_main_window(app);
            let _ = app.emit("menu-settings", ());
        }
        MENU_CYCLE_MODEL => {
            let _ = app.emit("menu-cycle-model", ());
        }
        MENU_CYCLE_REASONING => {
            let _ = app.emit("menu-cycle-reasoning", ());
        }
        MENU_NEW_AGENT => {
            let _ = app.emit("menu-new-agent", ());
        }
        MENU_INTERRUPT => {
            let _ = app.emit("menu-interrupt", ());
        }
        MENU_TOGGLE_SIDEBAR => {
            let _ = app.emit("menu-toggle-sidebar", ());
        }
        MENU_TOGGLE_TERMINAL => {
            let _ = app.emit("menu-toggle-terminal", ());
        }
        MENU_ZOOM => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.maximize();
            }
        }
        MENU_QUIT => {
            app.exit(0);
        }
        _ => {
            tracing::debug!("[reflect-gui] menu: unhandled id={id}");
        }
    }
}

/// 弹出并聚焦主窗口 (tray.rs 与 menu.rs 共用)。
fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// macOS close-to-tray:拦截 CloseRequested,隐藏而不是退出。
///
/// 只有 macOS 关闭按钮触发 hide;Windows / Linux 行为保持默认 (直接退出)。
/// 由 `lib.rs` 的 `.on_window_event(...)` 闭包调用。
pub fn handle_close_to_tray<R: Runtime>(window: &tauri::Window<R>, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    #[cfg(target_os = "macos")]
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window; // silence unused
        let _ = event;
    }
}