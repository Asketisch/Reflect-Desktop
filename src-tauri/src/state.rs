//! `AppState` —— 真后端 facade:嵌入 `reflect_core::AgentThread`,接真实 LLM
//! provider 与内置工具集。
//!
//! 真正的实现位于 [`state::agent`] / [`state::install`] / [`state::session`] /
//! [`state::submit`];本文件只暴露 `MinimalAgent` 顶壳与 thin 委托方法。
//!
//! ## 公开 API
//!
//! `new_empty` / `install_agent_thread` / `submit` / `submit_op` /
//! `subscribe_session` / `session_tx` / `interrupt` / `model_spec` /
//! `workspace` / `agent_status` / `cfg` / `tools` / domain APIs。
//! `commands/mod.rs` 与 `lib.rs` 不需要改协议面(Submission/Event 形态不变)。

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use reflect_config::ReflectConfig;
use reflect_protocol::Event;
use reflect_tools::ToolRegistry;
use tokio::sync::broadcast;

mod agent;
mod install;
mod session;
mod submit;

pub use agent::AgentStatus;

/// 主结构:封装一个 `AgentThread` + session 级 broadcast + 共享 config/tools。
///
/// Clone 是廉价的(内部全 Arc/broadcast channel)。二段构造:`new_empty()` 在
/// `tauri::Builder::manage` 同步阶段调用,`install_agent_thread()` 在 setup
/// 内的 async runtime 上下文里调用。
#[derive(Clone)]
pub struct MinimalAgent {
    pub(crate) inner: Arc<agent::MinimalAgentInner>,
}

impl MinimalAgent {
    /// 第一阶段:构造空的 MinimalAgent,只能 hold 状态(无 thread)。
    pub fn new_empty() -> Self {
        Self {
            inner: Arc::new(agent::build_empty_inner()),
        }
    }

    /// 第二阶段:在 Tauri setup 闭包内调用,真构造 AgentThread + 启动 forwarder。
    pub fn install_agent_thread(&self) {
        install::install_agent_thread(self);
    }

    /// 提交 Submission → 拿 per-turn `TurnHandle` → spawn 转发到 broadcast。
    pub async fn submit(&self, submission: reflect_protocol::Submission) -> anyhow::Result<()> {
        submit::submit(self, submission).await
    }

    /// 投递一个非 `UserInput` 的 `Op`(compact / rewind / approval / plan / effort /
    /// permission / ask_user 等)。自动生成 submission id。
    pub async fn submit_op(&self, op: reflect_protocol::Op) -> anyhow::Result<String> {
        submit::submit_op(self, op).await
    }

    /// 订阅 session 级 broadcast。前端 Tauri listener 拿到的就是这个 receiver。
    pub fn subscribe_session(&self) -> broadcast::Receiver<Event> {
        session::subscribe_session(self)
    }

    /// session broadcast sender 句柄(MCP/LSP lifecycle event 反向推送用)。
    pub fn session_tx(&self) -> broadcast::Sender<Event> {
        session::session_tx(self)
    }

    /// 中断当前 turn —— 调用 AgentThread 的 cancel token。
    pub fn interrupt(&self) {
        submit::interrupt(self);
    }

    /// 诊断接口:返回当前 model spec (供前端 settings UI 显示)。
    pub fn model_spec(&self) -> String {
        self.inner.model_spec.read().clone()
    }

    /// 诊断接口:返回当前 workspace (供前端显示)。
    pub fn workspace(&self) -> &PathBuf {
        &self.inner.workspace
    }

    /// 共享 config 只读句柄(settings 读写命令、热重载用)。
    pub fn cfg(&self) -> Arc<RwLock<ReflectConfig>> {
        Arc::clone(&self.inner.cfg)
    }

    /// 共享 ToolRegistry(MCP/LSP 后续 register_plugin_tool、tool 列表命令用)。
    pub fn tools(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.inner.tools)
    }

    /// B8-01: register a shell session; returns the kill handle to put in the map.
    pub fn register_shell_session(
        &self,
        id: String,
        child: Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>,
    ) {
        self.inner.shell_sessions.register(id, child);
    }

    /// B8-01: remove + return the kill handle for a shell session.
    pub fn take_shell_session(
        &self,
        id: &str,
    ) -> Option<Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>> {
        self.inner.shell_sessions.take(id)
    }

    /// B8-01: list active shell session ids.
    pub fn list_shell_sessions(&self) -> Vec<String> {
        self.inner.shell_sessions.list()
    }

    /// 诊断快照:供前端显示状态徽标。
    pub fn agent_status(&self) -> AgentStatus {
        let thread_ready = self.inner.thread.lock().is_some();
        let model = self.model_spec();
        let has_model = model != "stub/test";
        AgentStatus {
            ready: thread_ready,
            has_model,
            model,
            workspace: self.inner.workspace.display().to_string(),
            degraded_reason: self.inner.degraded_reason.lock().clone(),
        }
    }

    // ====== B1-07 / B9-06 / B11-* domain APIs ======

    /// Update the active workspace (B9-06). The change is in-memory.
    pub fn set_workspace(&self, path: PathBuf) {
        crate::workspace_state::WorkspaceState::set(path);
    }

    pub fn workspace_override() -> Option<PathBuf> {
        crate::workspace_state::WorkspaceState::get()
    }

    /// List memory entries across all scopes (B11-01).
    pub fn list_memory(&self) -> anyhow::Result<Vec<crate::commands::MemoryEntry>> {
        self.inner.memory_store.list()
    }

    /// Add a memory entry (B11-01). Appends `## <key>\n<value>` to the
    /// matching scope's `MEMORY.md`.
    pub fn add_memory(&self, scope: String, key: String, value: String) -> anyhow::Result<()> {
        self.inner.memory_store.add(&scope, &key, &value)
    }

    /// Remove a memory entry (B11-01). Strips the `## <key>` block.
    pub fn remove_memory(&self, scope: String, key: String) -> anyhow::Result<()> {
        self.inner.memory_store.remove(&scope, &key)
    }

    /// List known hooks (B11-02).
    pub fn list_hooks(&self) -> anyhow::Result<Vec<crate::commands::HookInfo>> {
        self.inner.hook_store.list()
    }

    /// Toggle a hook (B11-02).
    pub fn toggle_hook(&self, name: String, enabled: bool) -> anyhow::Result<()> {
        self.inner.hook_store.toggle(name, enabled)
    }
}
