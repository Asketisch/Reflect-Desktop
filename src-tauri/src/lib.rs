//! `reflect-desktop` Tauri 2 后端库。
//!
//! ## 里程碑
//!
//! - **M1.x** (已): 协议桥 —— 19 个 Tauri command + Event 转发到前端,MinimalAgent stub。
//! - **M2.x** (当前): 真后端 —— MinimalAgent 改为嵌入 `reflect_core::AgentThread`
//!   (stub model + EchoTool,不依赖网络);4 个产品联动 (tray / menu / shortcut / dock) 实装;
//!   macOS close-to-tray。
//!
//! 蓝图:Reflect-Agent `docs/gui/03-architecture.md` §5 (M1.x 设计沉淀)。

pub mod commands;
pub mod dock;
pub mod events;
mod hook_store;
pub mod mcp;
mod media_backend;
mod memory_store;
pub mod menu;
mod shell_sessions;
pub mod shortcut;
pub mod state;
pub mod tray;
mod workspace_state;

use commands::{
    reflect_activity_count, reflect_add_memory, reflect_add_schedule, reflect_agent_status,
    reflect_archive_session, reflect_ask_user_input_response, reflect_ask_user_question_response,
    reflect_assign_squad_task, reflect_autopilot_history, reflect_bind_session,
    reflect_cancel_side_channel, reflect_check_allowlist, reflect_check_update, reflect_claim_task,
    reflect_clear_activity, reflect_compact, reflect_computer_use, reflect_create_session,
    reflect_create_squad, reflect_create_task, reflect_current_workspace,
    reflect_cycle_permission_mode, reflect_delegate_next, reflect_delete_agent_def,
    reflect_delete_session, reflect_delete_squad, reflect_delete_task, reflect_delete_team,
    reflect_dream, reflect_enter_goal_mode, reflect_enter_plan_mode, reflect_exit_goal_mode,
    reflect_exit_plan_mode, reflect_export_session, reflect_export_session_markdown,
    reflect_fork_session, reflect_generate_session_title, reflect_get_agent_def,
    reflect_get_autopilot_config,
    reflect_get_config, reflect_get_effort, reflect_get_remote_config, reflect_get_remote_status,
    reflect_get_schedule_status, reflect_get_side_channel, reflect_get_squad, reflect_get_task,
    reflect_get_team, reflect_gh_pr_list, reflect_git_commit, reflect_git_diff, reflect_git_log,
    reflect_git_stage, reflect_git_status, reflect_git_unstage, reflect_hook_approval,
    reflect_image_process, reflect_interrupt, reflect_kill_shell, reflect_kms_create,
    reflect_kms_delete, reflect_kms_get_page, reflect_kms_list, reflect_kms_list_pages,
    reflect_kms_save_page, reflect_kms_search, reflect_list_activity, reflect_list_agent_defs,
    reflect_list_archived_sessions, reflect_list_dir, reflect_list_hooks, reflect_list_media,
    reflect_list_memory, reflect_list_provider_models, reflect_list_schedules,
    reflect_list_sessions, reflect_list_shell_sessions, reflect_list_side_channels,
    reflect_list_skills, reflect_list_squads, reflect_list_tasks, reflect_list_teams,
    reflect_list_tools, reflect_list_workspaces, reflect_load_allowlist, reflect_lsp_set_enabled,
    reflect_lsp_status, reflect_lsp_warmup, reflect_media_capabilities, reflect_parse_agent_md,
    reflect_pick_workspace_folder, reflect_plan_approval, reflect_query_plan_quota,
    reflect_query_subagents, reflect_read_file, reflect_read_image_base64, reflect_remove_memory,
    reflect_remove_schedule, reflect_rename_session, reflect_replay_session, reflect_reveal_path,
    reflect_rewind, reflect_run_shell, reflect_save_agent_def, reflect_save_allowlist,
    reflect_save_config, reflect_screenshot, reflect_search_activity, reflect_search_files,
    reflect_search_sessions, reflect_set_dock_badge, reflect_set_effort, reflect_set_model,
    reflect_set_permission_mode, reflect_set_workspace, reflect_shutdown,
    reflect_start_side_channel, reflect_steer, reflect_submit,
    reflect_tailscale_daemon_command_preview, reflect_tailscale_daemon_start,
    reflect_tailscale_daemon_status, reflect_tailscale_daemon_stop, reflect_tailscale_status,
    reflect_test_provider_chat, reflect_toggle_hook, reflect_tool_approval,
    reflect_unarchive_session, reflect_update_autopilot_config, reflect_update_remote_config,
    reflect_update_schedule, reflect_update_task, reflect_upsert_team, reflect_write_file,
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
///     - 启动 `AgentThread` 的 会话事件转发器;
/// 5. `on_window_event` 拦截 macOS 关闭按钮 → hide (close-to-tray)。
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        // ====== Plugins ======
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        // ====== 状态 ======
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
            reflect_enter_goal_mode,
            reflect_exit_goal_mode,
            reflect_plan_approval,
            reflect_set_effort,
            reflect_get_effort,
            reflect_ask_user_question_response,
            reflect_ask_user_input_response,
            reflect_set_permission_mode,
            reflect_cycle_permission_mode,
            reflect_steer,
            reflect_query_subagents,
            reflect_create_session,
            reflect_bind_session,
            reflect_list_sessions,
            reflect_rename_session,
            reflect_fork_session,
            reflect_generate_session_title,
            reflect_delete_session,
            reflect_archive_session,
            reflect_unarchive_session,
            reflect_list_archived_sessions,
            reflect_replay_session,
            reflect_search_sessions,
            reflect_export_session,
            // 诊断 / config / tools
            reflect_agent_status,
            reflect_get_config,
            reflect_save_config,
            reflect_set_model,
            reflect_query_plan_quota,
            reflect_list_provider_models, reflect_lsp_set_enabled, reflect_lsp_status,
    reflect_lsp_warmup,
            reflect_test_provider_chat,
            reflect_list_tools,
            // 各领域 domain 管理(workspace / skills / memory / hooks / git)
            reflect_list_workspaces,
            reflect_set_workspace,
            reflect_pick_workspace_folder,
            reflect_reveal_path,
            reflect_current_workspace,
            reflect_list_skills,
            reflect_list_memory,
            reflect_add_memory,
            reflect_remove_memory,
            reflect_list_hooks,
            reflect_toggle_hook,
            reflect_git_status,
            reflect_git_stage,
            reflect_git_unstage,
            reflect_git_commit,
            reflect_gh_pr_list,
    reflect_git_diff,
    reflect_git_log,
            // 终端 shell exec + streaming
            reflect_run_shell,
            reflect_kill_shell,
            reflect_list_shell_sessions,
            // 文件树 + read_file
            reflect_list_dir,
            reflect_read_file,
            reflect_read_image_base64,
            reflect_write_file,
            // 审批白名单 / Markdown 导出 / 更新检查 / 文件搜索
            reflect_load_allowlist,
            reflect_save_allowlist,
            reflect_check_allowlist,
            reflect_export_session_markdown,
            reflect_check_update,
            reflect_search_files,
            // Task/Team 管理命令(多 agent 协调,包装 reflect_task::TaskManager)
            reflect_list_tasks,
            reflect_create_task,
            reflect_get_task,
            reflect_update_task,
            reflect_claim_task,
            reflect_delete_task,
            reflect_list_teams,
            reflect_upsert_team,
            reflect_get_team,
            reflect_delete_team,
            // Schedule(cron)管理命令(包装 reflect_stream::cron::CronScheduler)
            reflect_list_schedules,
            reflect_add_schedule,
            reflect_update_schedule,
            reflect_remove_schedule,
            reflect_get_schedule_status,
            // Agent definition 管理命令(包装 reflect_agent_def)
            reflect_list_agent_defs,
            reflect_get_agent_def,
            reflect_save_agent_def,
            reflect_delete_agent_def,
            reflect_parse_agent_md,
            // Side-channel(用户驱动的并发 agent)
            reflect_start_side_channel,
            reflect_cancel_side_channel,
            reflect_list_side_channels,
            reflect_get_side_channel,
            // 远程模式（iOS 远端 daemon）
            reflect_get_remote_config,
            reflect_update_remote_config,
            reflect_get_remote_status,
            reflect_tailscale_status,
            reflect_tailscale_daemon_command_preview,
            reflect_tailscale_daemon_start,
            reflect_tailscale_daemon_stop,
            reflect_tailscale_daemon_status,
            // KMS（基于 grep 的 wiki + /dream）
            reflect_kms_list,
            reflect_kms_create,
            reflect_kms_delete,
            reflect_kms_save_page,
            reflect_kms_get_page,
            reflect_kms_list_pages,
            reflect_kms_search,
            reflect_dream,
            // Autopilot(自动任务调度)
            reflect_get_autopilot_config,
            reflect_update_autopilot_config,
            reflect_autopilot_history,
            // Activity timeline(本地事件审计日志)
            reflect_list_activity,
            reflect_search_activity,
            reflect_clear_activity,
            reflect_activity_count,
            // Squad + Leader delegation
            reflect_list_squads,
            reflect_create_squad,
            reflect_get_squad,
            reflect_delete_squad,
            reflect_delegate_next,
            reflect_assign_squad_task,
            // 媒体工作室 + 电脑操控（仅元数据的后端）
            reflect_list_media,
            reflect_image_process,
            reflect_screenshot,
            reflect_computer_use,
            reflect_media_capabilities,
            reflect_set_dock_badge,
        ])
        // ====== Window 事件:macOS close-to-tray ======
        .on_window_event(|window, event| {
            menu::handle_close_to_tray(window, event);
        })
        // ====== Setup ======
        .setup(|app| {
            // 1. 系统托盘 (macOS-only,其他平台为 no-op)。Tray 必须在 setup 同步阶段建
            //    (因为 tray 注册在 sync builder context)。
            tray::build_tray(app.handle())?;

            // 2. 全局快捷键 (跨平台,plugin 注册)。
            //    注册失败(如 Cmd+Shift+Space 被截图/启动器类应用占用)只降级
            //    记日志,不阻断启动 —— 此前 `?` 上抛会让 run(...) panic,
            //    应用完全无法打开。
            if let Err(e) = shortcut::register_global_shortcuts(app.handle()) {
                tracing::warn!("[reflect-gui] global shortcut registration failed, continuing without it: {e}");
            }

            // 3. 启动时清空 dock badge。
            dock::set_dock_badge(app.handle(), None);

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

/// 占位 command:验证 IPC 通路。
#[tauri::command]
fn ping() -> PingResp {
    PingResp {
        msg: "pong".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}
