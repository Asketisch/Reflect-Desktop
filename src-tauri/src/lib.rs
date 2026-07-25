//! `reflect-desktop` Tauri 2 backend lib。
//!
//! ## 里程碑
//!
//! - **M1.x** (已): 协议桥 —— 19 个 Tauri command + Event 转发到前端,MinimalAgent stub。
//! - **M2.x** (当前): 真后端 —— MinimalAgent 改为嵌入 `reflect_core::AgentThread`
//!   (stub model + EchoTool,不依赖网络);4 个产品联动 (tray / menu / shortcut / dock) 实装;
//!   macOS close-to-tray。
//!
//! 蓝图（历史）: Reflect-Agent `docs/gui/03-architecture.md` §5 (M1.x 设计沉淀)。

pub mod commands;
pub mod dock;
pub mod events;
mod hook_store;
pub mod mcp;
mod memory_store;
pub mod menu;
mod shell_sessions;
pub mod shortcut;
pub mod state;
pub mod tray;
mod workspace_state;

use commands::{
    reflect_add_memory, reflect_agent_status, reflect_ask_user_input_response,
    reflect_ask_user_question_response, reflect_check_allowlist, reflect_check_update,
    reflect_compact, reflect_current_workspace, reflect_cycle_permission_mode,
    reflect_delete_session, reflect_enter_plan_mode, reflect_exit_plan_mode,
    reflect_export_session, reflect_export_session_markdown, reflect_get_config, reflect_git_diff,
    reflect_git_log, reflect_git_status, reflect_hook_approval, reflect_interrupt,
    reflect_kill_shell, reflect_list_dir, reflect_list_hooks, reflect_list_memory,
    reflect_list_sessions, reflect_list_shell_sessions, reflect_list_skills, reflect_list_tools,
    reflect_list_workspaces, reflect_load_allowlist, reflect_plan_approval, reflect_read_file,
    reflect_remove_memory, reflect_rename_session, reflect_replay_session, reflect_rewind,
    reflect_run_shell, reflect_save_allowlist, reflect_save_config, reflect_search_files,
    reflect_set_effort, reflect_set_permission_mode, reflect_set_workspace, reflect_shutdown,
    reflect_submit, reflect_toggle_hook, reflect_tool_approval,
};
use serde::Serialize;
use state::MinimalAgent;
use tauri::Manager;

#[derive(Debug, Serialize)]
struct PingResp {
    msg: String,
    version: String,
}

/// 启动 Tauri 应用:
///
/// 1. 注册 `MinimalAgent` (M2.x = 真 `reflect_core::AgentThread` + stub model);
/// 2. 注册 19 个 reflect command + ping + `reflect_set_dock_badge`;
/// 3. 注册 `tauri-plugin-global-shortcut` plugin;
/// 4. 在 setup 中:
///     - 启动 `forward_agent_events` 把 agent event 推到前端;
///     - 注册全局快捷键 + 应用菜单 + 托盘图标;
///     - 启动 `AgentThread` 的 session event forwarder;
/// 5. `on_window_event` 拦截 macOS 关闭按钮 → hide (close-to-tray)。
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        // ====== Plugins ======
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // ====== State ======
        .manage(MinimalAgent::new_empty())
        // ====== Menu (在 builder 阶段静态注入) ======
        .enable_macos_default_menu(false)
        .menu(menu::build_menu)
        .on_menu_event(menu::handle_menu_event)
        // ====== IPC commands ======
        .invoke_handler(tauri::generate_handler![
            ping,
            reflect_submit,
            reflect_interrupt,
            reflect_compact,
            reflect_rewind,
            reflect_shutdown,
            reflect_tool_approval,
            reflect_hook_approval,
            reflect_enter_plan_mode,
            reflect_exit_plan_mode,
            reflect_plan_approval,
            reflect_set_effort,
            reflect_ask_user_question_response,
            reflect_ask_user_input_response,
            reflect_set_permission_mode,
            reflect_cycle_permission_mode,
            reflect_list_sessions,
            reflect_rename_session,
            reflect_delete_session,
            reflect_replay_session,
            reflect_export_session,
            // 诊断 / config / tools
            reflect_agent_status,
            reflect_get_config,
            reflect_save_config,
            reflect_list_tools,
            // B1-07: domain management (B9-06 / B11-*)
            reflect_list_workspaces,
            reflect_set_workspace,
            reflect_current_workspace,
            reflect_list_skills,
            reflect_list_memory,
            reflect_add_memory,
            reflect_remove_memory,
            reflect_list_hooks,
            reflect_toggle_hook,
            reflect_git_status,
    reflect_git_diff,
    reflect_git_log,
            // B8-01: terminal shell exec + streaming
            reflect_run_shell,
            reflect_kill_shell,
            reflect_list_shell_sessions,
            // B9-01: file tree + read_file
            reflect_list_dir,
            reflect_read_file,
            // B13-B16: approval allowlist, markdown export, update check, file search
            reflect_load_allowlist,
            reflect_save_allowlist,
            reflect_check_allowlist,
            reflect_export_session_markdown,
            reflect_check_update,
            reflect_search_files,
            dock::reflect_set_dock_badge,
        ])
        // ====== Window 事件:macOS close-to-tray ======
        .on_window_event(|window, event| {
            menu::handle_close_to_tray(window, event);
        })
        // ====== Setup ======
        .setup(|app| {
            // 1. 系统托盘 (macOS-only,其他平台为 no-op)。Tray 必须在 setup 同步阶段建
            //    (因为 tray 注册在 sync builder context)。
            tray::build_tray(&app.handle())?;

            // 2. 全局快捷键 (跨平台,plugin 注册)。
            shortcut::register_global_shortcuts(&app.handle())?;

            // 3. 启动时清空 dock badge。
            dock::set_dock_badge(&app.handle(), None);

            // 4. 二段构造 AgentThread —— Tauri setup 闭包**不是** tokio runtime
            //    上下文,必须用 tauri::async_runtime::spawn 把 install 推迟到
            //    Tauri 内部 runtime 起来之后 (event loop 阶段)。
            //
            //    install_agent_thread() 内部会构造真 AgentThread (内部 tokio::spawn
            //    submission_loop) 并启动 session forwarder。
            let agent_for_install: MinimalAgent = (*app.state::<MinimalAgent>()).clone();
            tauri::async_runtime::spawn(async move {
                agent_for_install.install_agent_thread();
            });

            // 5. 启动 broadcast → Tauri event 转发 (独立于 install,可在 sync setup
            //    后立刻 spawn,broadcast channel 已 ready,subscriber 可以先于 agent
            //    安装;eventual lag 由 Tauri Emitter 端容忍)。
            let agent_for_forward: MinimalAgent = (*app.state::<MinimalAgent>()).clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let session_rx = agent_for_forward.subscribe_session();
                events::forward_agent_events(app_handle, session_rx).await;
            });

            tracing::info!(
                "[reflect-gui] setup complete (model={}, workspace={}); agent thread installing in async runtime",
                app.state::<MinimalAgent>().model_spec(),
                app.state::<MinimalAgent>().workspace().display()
            );

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running reflect-desktop");
}

/// M1.x 占位 command:验证 IPC 通路。
#[tauri::command]
fn ping() -> PingResp {
    PingResp {
        msg: "pong".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}
