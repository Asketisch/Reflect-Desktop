//! `reflect-desktop` Tauri 2 backend lib.
//!
//! M1.x 里程碑 —— 协议桥接通：19 个 Tauri command + Event 转发到前端。
//! M2.x 补: 真实 reflect-core::AgentThread + 4 product 联动 (tray/menu/shortcut/dock)。
//!
//! 设计蓝图（历史）: docs/gui/03-architecture.md (Reflect-Agent 仓 docs/, M1.x 沉淀)。

pub mod commands;
pub mod dock;
pub mod events;
pub mod menu;
pub mod shortcut;
pub mod state;
pub mod tray;

use commands::{
    reflect_ask_user_input_response, reflect_ask_user_question_response, reflect_compact,
    reflect_cycle_permission_mode, reflect_delete_session, reflect_enter_plan_mode,
    reflect_exit_plan_mode, reflect_hook_approval, reflect_interrupt, reflect_list_sessions,
    reflect_plan_approval, reflect_rename_session, reflect_replay_session, reflect_rewind,
    reflect_set_effort, reflect_set_permission_mode, reflect_shutdown, reflect_submit,
    reflect_tool_approval,
};
use serde::Serialize;
use state::MinimalAgent;
use tauri::Manager;

#[derive(Debug, Serialize)]
struct PingResp {
    msg: String,
    version: String,
}

/// 启动 Tauri 应用：
///   1. 构造并注册 `MinimalAgent`(M1.x 暂用 stub 后端;真实 AgentThread 见 M2.x)。
///   2. 在 setup 中启动 `forward_agent_events`。
///   3. 注册 19 个 Tauri command + ping 占位。
///
/// M2.x 起增 tray/menu/shortcut/dock。当前 stub 模块占位。
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(MinimalAgent::spawn())
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
        ])
        .setup(|app| {
            // M1.2: 启动 submission loop (在 Tauri runtime 内部) + forward events。
            let agent = app.state::<MinimalAgent>();
            agent.start();
            let session_rx = agent.subscribe_session();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                events::forward_agent_events(app_handle, session_rx).await;
            });
            // M2.x 在此调 tray::build_tray / shortcut::register_global_shortcuts / dock::set_dock_badge
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

#[allow(dead_code)]
fn _ensure_delete_registered() {
    let _ = commands::reflect_delete_session;
    let _ = tray::TR;  // silence module
    let _ = menu::MK; // silence module
    let _ = dock::DK;
    let _ = shortcut::SK;
}

mod _stub_modules {
    use super::*;
    pub(crate) struct Marker<T>(T);
    impl tray::Stub for Marker<()> {}
    impl menu::Stub for Marker<()> {}
    impl dock::Stub for Marker<()> {}
    impl shortcut::Stub for Marker<()> {}
}

trait Stub {}
