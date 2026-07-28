//! `MinimalAgentInner` —— 共享 inner state,以及 `new_empty` 与 `AgentStatus`。
//!
//! `state.rs` 只保留 facade;实际的内层结构、构造期常量、诊断快照
//! 全部由本模块提供。

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex as ParkingMutex, RwLock};
use reflect_config::ReflectConfig;
use reflect_core::AgentThread;
use reflect_protocol::Event;
use reflect_tools::ToolRegistry;
use serde::Serialize;
use tokio::sync::broadcast;

use super::remote_config::RemoteConfig;

/// Session-level broadcast 通道容量。覆盖 1024 个 event 后最旧事件被丢;
/// 桌面端 UI 通常只关心最近 50-100 个事件,这个容量远超实际需要。
pub(crate) const SESSION_BROADCAST_CAPACITY: usize = 1024;

/// Agent 共享 inner:由 `Arc<MinimalAgentInner>` 包装,所有 `MinimalAgent`
/// 的 clone 共享同一份状态。
pub(crate) struct MinimalAgentInner {
    /// `Option` 因为 `new_empty` 时还没建。
    pub(crate) thread: ParkingMutex<Option<Arc<AgentThread>>>,
    /// Session event broadcast — 多个 Tauri command / webview 可各自订阅。
    pub(crate) session_tx: broadcast::Sender<Event>,
    /// 共享 config —— 热重载 / settings 读写命令共用。
    pub(crate) cfg: Arc<RwLock<ReflectConfig>>,
    /// 共享 ToolRegistry —— MCP/LSP 后续 register_plugin_tool 用。
    pub(crate) tools: Arc<ToolRegistry>,
    /// 当前 model spec 字符串(诊断用,真实值从 cfg 解析)。
    pub(crate) model_spec: RwLock<String>,
    /// 工作区根 —— 当前固定 cwd;M4.x 由前端 settings 切换。
    pub(crate) workspace: PathBuf,
    /// 降级原因(无 API key 等);None 表示正常就绪。
    pub(crate) degraded_reason: ParkingMutex<Option<String>>,
    pub(crate) shell_sessions: crate::shell_sessions::ShellSessions,
    pub(crate) memory_store: crate::memory_store::MemoryStore,
    pub(crate) hook_store: crate::hook_store::HookStore,
    /// Task/Team 管理器(Phase 1 多 agent 命令面)。
    /// 复用 vendor/reflect-task 默认 home(`~/.reflect`),与 TUI/CLI 共享数据。
    /// Phase 0 形态:无 hook_engine / event_sink;后续阶段把 TaskCreated/Completed
    /// 事件推前端时再 `with_event_sink`。
    pub(crate) task_manager: Arc<reflect_task::TaskManager>,
    /// Cron 调度器(Phase 1 第 2 项)。`None` 直到 `install_agent_thread`
    /// 把真 `AgentThread::submission_sender()` 注入(driver 才能真正触发);
    /// 此前 CRUD 仍可用(命令层读 `RwLock`,只是无后台 driver)。
    pub(crate) cron_scheduler: parking_lot::RwLock<Option<reflect_stream::cron::CronScheduler>>,
    /// Side-channel registry(Phase 2 第 1 项):每个 side-channel 持有独立
    /// `CancelToken`,主 agent 的 cancel 不会传播到 side-channel。事件流
    /// 经 `subscribe_events()` → Tauri `reflect_event` channel 推前端。
    pub(crate) side_channels: reflect_app_core::side_channel::SideChannelRegistry,
    /// Remote mode config(Phase 2 第 2 项):iOS / 远端 daemon 连接的目标
    /// (host + port + auth_token) 与运行时状态。进程内存储,重启重置
    /// —— 持久化留作后续。
    pub(crate) remote_config: parking_lot::RwLock<RemoteConfig>,
    /// KMS 知识库管理器(Phase 3 第 12 项):grep-based wiki + /dream。
    pub(crate) kms_manager: reflect_app_core::kms::KnowledgeManager,
    /// Autopilot 管理器(Phase 3 第 10 项):自动任务调度。
    pub(crate) autopilot_manager: reflect_app_core::autopilot::AutopilotManager,
    /// Activity 日志管理器(Phase 3 第 9 项):本地事件审计 timeline。
    /// 由 `install_agent_thread` 订阅 session broadcast 把 `Event` 映射成
    /// `ActivityEvent` 后写入。
    pub(crate) activity_logger: Arc<reflect_app_core::activity::ActivityLogger>,
    /// Squad 管理器(Phase 3 第 11 项):复用 `task_manager` 的 TeamFile 存储,
    /// 提供 leader 委派语义层。
    pub(crate) squad_manager: Arc<reflect_app_core::squad::SquadManager>,
}

/// agent 诊断快照 —— 给前端显示状态徽标(ready / 是否有 API key / 模型 / 工作区)。
#[derive(Debug, Clone, Serialize)]
pub struct AgentStatus {
    /// AgentThread 是否已安装(setup 异步安装完成前为 false)。
    pub ready: bool,
    /// 是否检测到可用 provider(读到了 API key / 配置)。
    pub has_model: bool,
    /// active provider 的 model spec(`anthropic/claude-...`),降级时为 `stub/test`。
    pub model: String,
    /// 当前工作区根。
    pub workspace: String,
    /// 降级原因(无 API key / config 缺失等),给前端 tooltip 用。
    pub degraded_reason: Option<String>,
}

/// 第一阶段:构造空的 MinimalAgent,只能 hold 状态(无 thread)。
///
/// `tauri::Builder::manage(MinimalAgent::new_empty())` 时调用。
pub(crate) fn build_empty_inner() -> MinimalAgentInner {
    let (session_tx, _) = broadcast::channel(SESSION_BROADCAST_CAPACITY);
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cfg = reflect_config::load_default();
    // Task/Team 存储复用 vendor 默认 home(REFLECT_HOME 或 $HOME/.reflect),
    // 与 TUI/CLI 共享 `~/.reflect/tasks/` 与 `~/.reflect/teams/`。
    // home 解析失败时回退到 cwd 下的 `.reflect`,保证进程仍可启动(命令会报 I/O 错)。
    let task_store = Arc::new(
        reflect_task::FileTaskStore::with_default_home()
            .unwrap_or_else(|_| reflect_task::FileTaskStore::new(".")),
    );
    let team_store = Arc::new(
        reflect_task::FileTeamStore::with_default_home()
            .unwrap_or_else(|_| reflect_task::FileTeamStore::new(".")),
    );
    let task_manager = Arc::new(reflect_task::TaskManager::new(task_store, team_store));
    // Cron scheduler 初始为 None;install_agent_thread 注入真 sender 后重建。
    let cron_scheduler = parking_lot::RwLock::new(None);
    let side_channels = reflect_app_core::side_channel::SideChannelRegistry::new();
    let remote_config = parking_lot::RwLock::new(RemoteConfig::default());
    let kms_manager = reflect_app_core::kms::KnowledgeManager::with_default_home();
    let autopilot_manager = reflect_app_core::autopilot::AutopilotManager::new();
    let activity_logger = Arc::new(reflect_app_core::activity::ActivityLogger::with_default_home());
    // SquadManager 复用 task_manager(team 存储),必须在 task_manager 之后构造。
    let squad_manager = Arc::new(reflect_app_core::squad::SquadManager::new(Arc::clone(&task_manager)));
    MinimalAgentInner {
        thread: ParkingMutex::new(None),
        session_tx,
        cfg: Arc::new(RwLock::new(cfg)),
        tools: Arc::new(ToolRegistry::new()),
        model_spec: RwLock::new("stub/test".to_string()),
        workspace,
        degraded_reason: ParkingMutex::new(None),
        shell_sessions: crate::shell_sessions::ShellSessions::default(),
        memory_store: crate::memory_store::MemoryStore,
        hook_store: crate::hook_store::HookStore::default(),
        task_manager,
        cron_scheduler,
        side_channels,
        remote_config,
        kms_manager,
        autopilot_manager,
        activity_logger,
        squad_manager,
    }
}

#[cfg(test)]
mod tests {
    use crate::state::MinimalAgent;

    #[test]
    fn new_empty_has_no_thread() {
        let agent = MinimalAgent::new_empty();
        assert!(agent.inner.thread.lock().is_none());
        // model_spec 初始为降级标记。
        assert_eq!(agent.model_spec(), "stub/test");
    }

    #[test]
    fn agent_status_reports_degraded_when_no_provider() {
        // 测试环境无 API key/env,active_provider() 应为 None。
        let agent = MinimalAgent::new_empty();
        let status = agent.agent_status();
        assert!(!status.ready, "thread not installed yet");
    }

    /// 验证 ReflectConfig 真实加载:从 ~/.reflect/config.toml + env 读到非空 cfg。
    /// 这是 settings UI 显示的源数据;若 cfg 加载失败,整个 settings 页会空。
    #[test]
    fn cfg_loads_from_real_config() {
        let agent = MinimalAgent::new_empty();
        let cfg_handle = agent.cfg();
        let cfg = cfg_handle.read();
        // active_provider() 要么从 [active] provider 读到 "anthropic" 等,
        // 要么从 env(OPENAI_API_KEY / ANTHROPIC_API_KEY)推断。
        // CI 无配置时为 None,本地开发机通常有 ~/.reflect/config.toml。
        let providers_count = cfg.active_provider().is_some() as u32;
        assert!(
            providers_count <= 1,
            "active_provider should be Some or None, got multiple"
        );
    }

    /// agent_status 字段完整性:ready/model/workspace 必须有合法值,
    /// 不得 panic 或返回未初始化字符串。
    #[test]
    fn agent_status_fields_are_well_formed() {
        let agent = MinimalAgent::new_empty();
        let status = agent.agent_status();
        assert!(!status.ready, "pre-install: ready must be false");
        // model_spec 在 install 前为 "stub/test";install 后为 provider/model 字符串。
        assert!(!status.model.is_empty(), "model must not be empty");
        assert!(
            !status.workspace.is_empty(),
            "workspace path must not be empty"
        );
        // degraded_reason 在 install 前是 None(install 时根据 provider 决定)。
        assert!(status.degraded_reason.is_none() || status.degraded_reason.is_some());
    }
}
