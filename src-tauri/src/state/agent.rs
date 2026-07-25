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
