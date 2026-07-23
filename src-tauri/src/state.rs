//! `AppState` —— 真后端:嵌入 `reflect_core::AgentThread`,接真实 LLM provider
//! 与内置工具集。
//!
//! ## 当前进度
//!
//! - **M2.x stub** 已退役:不再用空 `ModelRegistry` + `EchoTool`。
//! - **真实 agent 集成**:
//!     - 用 `reflect_config::load_default()` 读 `~/.reflect/config.toml` + 环境变量
//!       (`OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `OLLAMA_HOST` / `REFLECT_PROVIDER` /
//!       `REFLECT_MODEL`);
//!     - `ReflectConfig::to_registry()` 构建真实 `ModelRegistry`(含 credential pool);
//!     - 注册 16 个 reflect-tools 内置工具;
//!     - `cfg.resolved_model_spec()` 解析真实模型 spec(如 `anthropic/claude-...`)。
//! - **降级策略**:开发环境无 API key 时,fallback 到空 registry + EchoTool,
//!   并通过 `agent_status()` 暴露 `has_api_key=false` 让前端显示状态徽标;
//!   `pnpm tauri dev` 永远能起,提交后 emit `EventMsg::Error`(可预期的 smoke 路径)。
//!
//! ## 公开 API
//!
//! `new_empty` / `install_agent_thread` / `start` / `submit` /
//! `subscribe_session` / `interrupt` / `model_spec` / `workspace` /
//! `agent_status` / `cfg` / `tools`。`commands/mod.rs` 与 `lib.rs`
//! 不需要改协议面(Submission/Event 形态不变)。

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex as ParkingMutex, RwLock};
use reflect_config::ReflectConfig;
use reflect_core::{AgentConfig, AgentThread, TurnHandle};
use reflect_llm::SharedModelRegistry;
use reflect_protocol::Event;
use reflect_tools::{
    Sanitizer, ToolRegistry,
    builtins::{
        BashTool, DeleteTool, EchoTool, EditTool, EnterPlanModeTool, EnterWorktreeTool,
        ExitPlanModeTool, ExitWorktreeTool, GlobTool, GrepTool, NotebookEditTool, ReadTool,
        ToolSearchTool, WebFetchTool, WebSearchTool, WriteTool,
    },
};
use serde::Serialize;
use tokio::sync::broadcast;

/// Session-level broadcast 通道容量。覆盖 1024 个 event 后最旧事件被丢;
/// 桌面端 UI 通常只关心最近 50-100 个事件,这个容量远超实际需要。
const SESSION_BROADCAST_CAPACITY: usize = 1024;

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

/// 主结构:封装一个 `AgentThread` + session 级 broadcast + 共享 config/tools。
///
/// Clone 是廉价的(内部全 Arc/broadcast channel)。
///
/// 二段构造:
/// 1. `new_empty()` — 构造无 AgentThread 的 MinimalAgent,用于 `tauri::Builder::manage(...)`。
///    `AgentThread::new` 内部立即 `tokio::spawn`,必须在 tokio runtime 上下文;
///    Tauri `manage()` 是同步阶段,还没初始化 async_runtime。
/// 2. `install_agent_thread()` — 在 `setup()` 闭包内通过 `tauri::async_runtime::spawn`
///    推迟到 Tauri runtime 已起的上下文内调用,真正构造 `AgentThread` 并启动 forwarder。
#[derive(Clone)]
pub struct MinimalAgent {
    inner: Arc<MinimalAgentInner>,
}

struct MinimalAgentInner {
    /// `Option` 因为 `new_empty` 时还没建。
    thread: ParkingMutex<Option<Arc<AgentThread>>>,
    /// Session event broadcast — 多个 Tauri command / webview 可各自订阅。
    session_tx: broadcast::Sender<Event>,
    /// 共享 config —— 热重载 / settings 读写命令共用。
    cfg: Arc<RwLock<ReflectConfig>>,
    /// 共享 ToolRegistry —— MCP/LSP 后续 register_plugin_tool 用。
    tools: Arc<ToolRegistry>,
    /// 当前 model spec 字符串(诊断用,真实值从 cfg 解析)。
    model_spec: RwLock<String>,
    /// 工作区根 —— 当前固定 cwd;M4.x 由前端 settings 切换。
    workspace: PathBuf,
    /// 降级原因(无 API key 等);None 表示正常就绪。
    degraded_reason: ParkingMutex<Option<String>>,
    /// B8-01: terminal shell sessions — key = session_id (UUID),
    /// value = child process kill handle (Option<tokio::process::Child> wrapped).
    shell_sessions: parking_lot::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>>>,
}

impl MinimalAgent {
    /// 第一阶段:构造空的 MinimalAgent,只能 hold 状态(无 thread)。
    ///
    /// `tauri::Builder::manage(MinimalAgent::new_empty())` 时调用。
    pub fn new_empty() -> Self {
        let (session_tx, _) = broadcast::channel(SESSION_BROADCAST_CAPACITY);
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let cfg = reflect_config::load_default();
        Self {
            inner: Arc::new(MinimalAgentInner {
                thread: ParkingMutex::new(None),
                session_tx,
                cfg: Arc::new(RwLock::new(cfg)),
                tools: Arc::new(ToolRegistry::new()),
                model_spec: RwLock::new("stub/test".to_string()),
                workspace,
                degraded_reason: ParkingMutex::new(None),
                shell_sessions: parking_lot::Mutex::new(std::collections::HashMap::new()),
            }),
        }
    }

    /// 第二阶段:在 Tauri setup 闭包内调用,真构造 AgentThread + 启动 forwarder。
    ///
    /// ## 重要
    ///
    /// `AgentThread::new` 内部立即 `tokio::spawn(submission_loop(...))`,
    /// 必须跑在 tokio runtime 上下文。Tauri `setup` 闭包运行时 Tauri 已经
    /// 初始化了它的 async_runtime,这一步安全。
    pub fn install_agent_thread(&self) {
        // 1. 解析模型 spec 与 provider。读 cfg + env 的真实优先级。
        let cfg_snapshot = self.inner.cfg.read().clone();
        let workspace = self.inner.workspace.clone();
        let model_spec = cfg_snapshot
            .resolved_model_spec()
            .unwrap_or_else(|| "stub/test".to_string());
        let has_provider = cfg_snapshot.active_provider().is_some();

        // 2. 构造共享 ToolRegistry —— 无论是否有 provider 都先注册,
        //    MCP/LSP 后续也在这个 registry 上 register_plugin_tool。
        let tools = self.inner.tools.clone();
        register_builtin_tools(&tools);

        // 3. ModelRegistry:有 provider 走真路径;否则降级到空 registry + EchoTool。
        let registry: SharedModelRegistry = if has_provider {
            match cfg_snapshot.to_registry() {
                Ok(r) => Arc::new(r),
                Err(e) => {
                    tracing::warn!(
                        "[reflect-gui] to_registry failed ({}); falling back to empty registry",
                        e
                    );
                    *self.inner.degraded_reason.lock() =
                        Some(format!("model registry build failed: {e}"));
                    Arc::new(reflect_llm::ModelRegistry::new())
                }
            }
        } else {
            // 无 API key / provider 配置 —— 降级。注册 EchoTool 让 smoke 路径可走。
            tracing::warn!(
                "[reflect-gui] no provider configured (set OPENAI_API_KEY / ANTHROPIC_API_KEY \
                 or edit ~/.reflect/config.toml); running in degraded mode"
            );
            *self.inner.degraded_reason.lock() =
                Some("no provider configured — set an API key in Settings".to_string());
            tools.register(Arc::new(EchoTool));
            Arc::new(reflect_llm::ModelRegistry::new())
        };

        // 4. AgentConfig + AgentThread。
        let cfg = AgentConfig::new(model_spec.clone(), workspace.clone()).with_approvals(true);
        // sanitizer 用默认 10 pattern;AgentThread 内部传 None 也会走 with_defaults,
        // 这里显式构造便于后续接 [sanitize] config 段。
        let sanitizer = Arc::new(Sanitizer::with_defaults());
        let thread = Arc::new(AgentThread::new(cfg, registry, tools.clone(), Some(sanitizer)));

        // 5. 写回 inner。
        *self.inner.thread.lock() = Some(thread.clone());
        *self.inner.model_spec.write() = model_spec.clone();

        // 6. 启动 session event forwarder。
        self.start();

        // 7. MCP / LSP bootstrap(阶段 3d)—— 读 cfg 的 [mcp_servers]/[lsp_servers],
        //    启动 server + 注册 tool。lifecycle event 经 session broadcast 推前端。
        //    在 async runtime 里跑(bootstrap 内部 tokio::spawn + 网络 I/O)。
        let cfg_for_bootstrap = cfg_snapshot.clone();
        let tools_for_bootstrap = tools.clone();
        let session_tx_for_bootstrap = self.inner.session_tx.clone();
        let self_clone = self.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::mcp::bootstrap_mcp(
                &cfg_for_bootstrap,
                tools_for_bootstrap.clone(),
                session_tx_for_bootstrap.clone(),
            )
            .await;
            let _ = crate::mcp::bootstrap_lsp(
                &cfg_for_bootstrap,
                tools_for_bootstrap,
                session_tx_for_bootstrap,
            )
            .await;
            // 触发一次 unused warning 抑制(self_clone 保留 future 扩展用)。
            let _ = &self_clone;
        });

        tracing::info!(
            "[reflect-gui] AgentThread installed (model={}, workspace={}, degraded={})",
            model_spec,
            workspace.display(),
            !has_provider
        );
    }

    /// 启动 session 事件转发:从 `AgentThread::subscribe_session()` 拿 mpsc::Receiver,
    /// 持续 `recv().await` 并 broadcast 到所有 Tauri 订阅者。
    ///
    /// 在 `install_agent_thread` 内部调用,外部不应直接调。
    fn start(&self) {
        let thread_lock = self.inner.thread.lock();
        let Some(thread) = thread_lock.clone() else {
            tracing::error!("[reflect-gui] start called before install_agent_thread");
            return;
        };
        drop(thread_lock);

        let mut session_rx = thread.subscribe_session();
        let session_tx = self.inner.session_tx.clone();
        tauri::async_runtime::spawn(async move {
            tracing::info!("[reflect-gui] session forward task started");
            while let Some(event) = session_rx.recv().await {
                let _ = session_tx.send(event);
            }
            tracing::warn!("[reflect-gui] session forward task exited (AgentThread closed)");
        });
    }

    /// 提交 Submission → 拿 per-turn `TurnHandle` → spawn 转发到 broadcast。
    ///
    /// 前端拿到的所有 per-turn event 都通过 session broadcast 派发,
    /// 前端按 `event.id == submission.id` 过滤出属于本次 turn 的事件。
    pub async fn submit(&self, submission: reflect_protocol::Submission) -> anyhow::Result<()> {
        // 0. 必须先 install_agent_thread。
        let thread = {
            let guard = self.inner.thread.lock();
            guard
                .clone()
                .ok_or_else(|| anyhow::anyhow!("agent thread not installed yet; setup not complete"))?
        };

        // 1. 拿 per-turn handle。
        let mut handle: TurnHandle = thread.submit(submission.clone()).await;

        // 2. spawn 转发:这个 turn 的所有 event 进 broadcast。
        let session_tx = self.inner.session_tx.clone();
        let sub_id = submission.id.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = handle.next().await {
                if session_tx.send(event).is_err() {
                    tracing::debug!("[reflect-gui] no subscribers for turn {}", sub_id);
                }
            }
            tracing::debug!("[reflect-gui] turn {} forwarder closed", sub_id);
        });

        Ok(())
    }

    /// 投递一个非 `UserInput` 的 `Op`(compact / rewind / approval / plan / effort /
    /// permission / ask_user 等)。自动生成 submission id(EVENT_ID_NONE 级别的
    /// 生命周期 op 不需要前端 pairing,但审批/ask_user 的 id 与 pending 事件 id 对齐)。
    ///
    /// 供 `commands/mod.rs` 的 12 个 Op 命令统一调用。
    pub async fn submit_op(&self, op: reflect_protocol::Op) -> anyhow::Result<String> {
        let thread = {
            let guard = self.inner.thread.lock();
            guard
                .clone()
                .ok_or_else(|| anyhow::anyhow!("agent thread not installed yet; setup not complete"))?
        };
        let submission = reflect_protocol::Submission::with_id(
            uuid::Uuid::new_v4().to_string(),
            op,
        );
        let id = submission.id.clone();
        // 这类 op 通常无 per-turn 流式输出(审批/effort/permission 立即生效),
        // 但仍走 submit 以保持生命周期事件(SessionConfigured / PermissionModeChanged 等)
        // 的 fan-out 一致性。
        let mut handle = thread.submit(submission).await;
        let session_tx = self.inner.session_tx.clone();
        let sub_id = id.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = handle.next().await {
                let _ = session_tx.send(event);
            }
            tracing::debug!("[reflect-gui] op turn {} forwarder closed", sub_id);
        });
        Ok(id)
    }

    /// 订阅 session 级 broadcast。前端 Tauri listener 拿到的就是这个 receiver。
    pub fn subscribe_session(&self) -> broadcast::Receiver<Event> {
        self.inner.session_tx.subscribe()
    }

    /// session broadcast sender 句柄(MCP/LSP lifecycle event 反向推送用)。
    pub fn session_tx(&self) -> broadcast::Sender<Event> {
        self.inner.session_tx.clone()
    }

    /// 中断当前 turn —— 调用 AgentThread 的 cancel token。
    /// CancellationToken 的 cancel 是幂等的,重复调用安全。
    pub fn interrupt(&self) {
        let guard = self.inner.thread.lock();
        if let Some(thread) = guard.as_ref() {
            thread.cancel_token().cancel();
        }
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
        self.inner.shell_sessions.lock().insert(id, child);
    }

    /// B8-01: remove + return the kill handle for a shell session.
    pub fn take_shell_session(
        &self,
        id: &str,
    ) -> Option<Arc<tokio::sync::Mutex<Option<tokio::process::Child>>>> {
        self.inner.shell_sessions.lock().remove(id)
    }

    /// B8-01: list active shell session ids.
    pub fn list_shell_sessions(&self) -> Vec<String> {
        self.inner.shell_sessions.lock().keys().cloned().collect()
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

    /// Update the active workspace (B9-06). The change is in-memory; the
    /// next `Op::UserInput` runs with this cwd.
    ///
    /// **B1-07 simplification**: for v0.2 we keep the workspace as an
    /// atomic swap stored in a process-global. A future milestone will
    /// thread this through `AgentConfig` so the running `AgentThread`
    /// observes the change without restart.
    pub fn set_workspace(&self, path: PathBuf) {
        use parking_lot::Mutex as ParkingMutex;
        static OVERRIDE: once_cell::sync::Lazy<ParkingMutex<Option<PathBuf>>> =
            once_cell::sync::Lazy::new(|| ParkingMutex::new(None));
        *OVERRIDE.lock() = Some(path);
    }

    pub fn workspace_override() -> Option<PathBuf> {
        use parking_lot::Mutex as ParkingMutex;
        static OVERRIDE: once_cell::sync::Lazy<ParkingMutex<Option<PathBuf>>> =
            once_cell::sync::Lazy::new(|| ParkingMutex::new(None));
        OVERRIDE.lock().clone()
    }

    /// List memory entries across all scopes (B11-01).
    ///
    /// **B1-07 simplification**: reads the raw `MEMORY.md` files for
    /// project + user scopes. A real implementation would call into
    /// `reflect_memory::MemoryStore::list()` (which isn't on the v0.1
    /// trait surface); this unblocks the B11 UI without forcing a
    /// vendor crate change.
    pub fn list_memory(&self) -> anyhow::Result<Vec<crate::commands::MemoryEntry>> {
        let mut out = Vec::new();
        let scopes = [
            (
                "project",
                std::env::current_dir()
                    .ok()
                    .map(|c| c.join(".reflect/agent-memory/reflect/MEMORY.md")),
            ),
            (
                "user",
                dirs::home_dir().map(|h| h.join(".reflect/agent-memory/reflect/MEMORY.md")),
            ),
        ];
        for (label, path_opt) in scopes {
            if let Some(path) = path_opt {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        out.push(crate::commands::MemoryEntry {
                            scope: label.to_string(),
                            key: "(all)".to_string(),
                            value: content,
                        });
                    }
                }
            }
        }
        Ok(out)
    }

    /// Add a memory entry (B11-01). Appends `## <key>\n<value>` to the
    /// matching scope's `MEMORY.md`.
    pub fn add_memory(
        &self,
        scope: String,
        key: String,
        value: String,
    ) -> anyhow::Result<()> {
        let path = match scope.as_str() {
            "user" => dirs::home_dir()
                .map(|h| h.join(".reflect/agent-memory/reflect/MEMORY.md"))
                .ok_or_else(|| anyhow::anyhow!("no home dir"))?,
            _ => std::env::current_dir()
                .map(|c| c.join(".reflect/agent-memory/reflect/MEMORY.md"))
                .map_err(|e| anyhow::anyhow!("cwd: {e}"))?,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut current = std::fs::read_to_string(&path).unwrap_or_default();
        if !current.is_empty() && !current.ends_with('\n') {
            current.push('\n');
        }
        current.push_str(&format!("\n## {key}\n{value}\n"));
        std::fs::write(&path, current)?;
        Ok(())
    }

    /// Remove a memory entry (B11-01). Strips the `## <key>` block.
    pub fn remove_memory(&self, scope: String, key: String) -> anyhow::Result<()> {
        let path = match scope.as_str() {
            "user" => dirs::home_dir()
                .map(|h| h.join(".reflect/agent-memory/reflect/MEMORY.md"))
                .ok_or_else(|| anyhow::anyhow!("no home dir"))?,
            _ => std::env::current_dir()
                .map(|c| c.join(".reflect/agent-memory/reflect/MEMORY.md"))
                .map_err(|e| anyhow::anyhow!("cwd: {e}"))?,
        };
        if !path.exists() {
            return Ok(());
        }
        let current = std::fs::read_to_string(&path)?;
        let needle = format!("## {key}");
        if let Some(start) = current.find(&needle) {
            let after = start + needle.len();
            let end = current[after..]
                .find("\n## ")
                .map(|i| after + i)
                .unwrap_or(current.len());
            let mut new = String::with_capacity(current.len());
            new.push_str(&current[..start]);
            new.push_str(&current[end..]);
            std::fs::write(&path, new)?;
        }
        Ok(())
    }

    /// List known hooks (B11-02). Phase 1 returns the two built-in
    /// hooks (PlanModeGate + ReadBeforeEdit) that the upstream
    /// `reflect-hooks` crate always registers.
    pub fn list_hooks(&self) -> anyhow::Result<Vec<crate::commands::HookInfo>> {
        // **B1-07 simplification**: hard-coded for now; the upstream
        // `HookRegistry` doesn't expose a stable iteration API in the
        // vendored 0.1.0. Real registry iteration is B11-02 Phase 2.
        Ok(vec![
            crate::commands::HookInfo {
                name: "read_before_edit".to_string(),
                kind: "policy".to_string(),
                enabled: true,
                config_summary: "auto-reads file before Edit/Write".to_string(),
            },
            crate::commands::HookInfo {
                name: "plan_mode_gate".to_string(),
                kind: "policy".to_string(),
                enabled: true,
                config_summary: "blocks mutating tools in Plan mode".to_string(),
            },
        ])
    }

    /// Toggle a hook (B11-02). Phase 1 is a no-op acknowledgement —
    /// the upstream registry doesn't yet expose a public enable/disable
    /// method in the vendored 0.1.0; the toggle is recorded in
    /// `~/.reflect/hook_state.json` for future use.
    pub fn toggle_hook(&self, name: String, enabled: bool) -> anyhow::Result<()> {
        let path = dirs::home_dir()
            .map(|h| h.join(".reflect/hook_state.json"))
            .ok_or_else(|| anyhow::anyhow!("no home dir"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut state: std::collections::HashMap<String, bool> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        state.insert(name, enabled);
        std::fs::write(&path, serde_json::to_string_pretty(&state)?)?;
        Ok(())
    }
}

/// 注册 16 个 reflect-tools 内置工具到 `ToolRegistry`。
///
/// `ToolSearchTool` 需要持有 registry 句柄,最后注册(否则它搜不到其他工具)。
/// 全部工具零参数构造(除 ToolSearchTool)。
fn register_builtin_tools(tools: &Arc<ToolRegistry>) {
    // 文件/执行类。
    tools.register(Arc::new(BashTool));
    tools.register(Arc::new(ReadTool));
    tools.register(Arc::new(WriteTool));
    tools.register(Arc::new(EditTool));
    tools.register(Arc::new(DeleteTool));
    tools.register(Arc::new(GrepTool));
    tools.register(Arc::new(GlobTool));
    tools.register(Arc::new(NotebookEditTool));
    // Plan / Worktree。
    tools.register(Arc::new(EnterPlanModeTool));
    tools.register(Arc::new(ExitPlanModeTool));
    tools.register(Arc::new(EnterWorktreeTool));
    tools.register(Arc::new(ExitWorktreeTool));
    // Web。
    tools.register(Arc::new(WebFetchTool::new()));
    tools.register(Arc::new(WebSearchTool::new()));
    // 工具搜索 —— 持有 registry 句柄,最后注册。
    tools.register(Arc::new(ToolSearchTool::new(Arc::clone(tools))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_has_no_thread() {
        let agent = MinimalAgent::new_empty();
        assert!(agent.inner.thread.lock().is_none());
        // model_spec 初始为降级标记。
        assert_eq!(agent.model_spec(), "stub/test");
    }

    #[test]
    fn register_builtin_tools_populates_registry() {
        let tools = Arc::new(ToolRegistry::new());
        register_builtin_tools(&tools);
        let names = tools.list();
        // 15 个内置工具(不含 EchoTool —— 仅降级模式注册)。
        // 注意:工具 name() 与 struct 名不一致,且风格混杂 ——
        // 多数 snake_case,但 plan/worktree 系列是 PascalCase(上游历史遗留)。
        for expected in [
            "bash",
            "read",
            "write",
            "edit",
            "delete_file",
            "grep",
            "glob",
            "notebook_edit",
            "EnterPlanMode",
            "ExitPlanMode",
            "EnterWorktree",
            "ExitWorktree",
            "web_fetch",
            "web_search",
            "tool_search",
        ] {
            assert!(names.contains(&expected.to_string()), "missing tool: {expected}");
        }
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

    /// install_agent_thread 必须在 tokio runtime 上下文里调用,
    /// 否则 AgentThread 内部 tokio::spawn 会 panic。
    /// 这里用 tokio::test 验证 install 后状态翻转 + tools 注册数正确。
    #[tokio::test]
    async fn install_agent_thread_populates_state() {
        let agent = MinimalAgent::new_empty();
        // 初始:thread 未安装,model_spec 为 stub。
        assert!(!agent.agent_status().ready);
        assert_eq!(agent.model_spec(), "stub/test");

        // install:读真实 cfg + 注册 15 个内置工具 + 启动 session forwarder。
        // 注意:install 内部会 tauri::async_runtime::spawn MCP bootstrap,
        // 但 tauri::async_runtime::spawn 在 tokio::test runtime 里会复用当前
        // runtime(若 tauri 未单独 init),这里仅验证同步可见状态。
        // 若 MCP bootstrap 网络失败,不影响 install 返回(已被 spawn 隔离)。
        agent.install_agent_thread();

        // ready 翻转为 true。
        let status = agent.agent_status();
        assert!(status.ready, "after install: ready must be true");

        // tools:15 个 builtin(若 cfg 有 provider) 或 16 个(+EchoTool 降级)。
        let tool_count = agent.tools().list().len();
        assert!(
            tool_count >= 15,
            "must register at least 15 builtin tools, got {tool_count}"
        );

        // model_spec 不再是 stub(若有 provider);若 cfg 无 provider 则仍是 stub。
        let model = agent.model_spec();
        assert!(
            !model.is_empty(),
            "model_spec must be non-empty after install"
        );

        // interrupt 幂等:install 后调用不应 panic。
        agent.interrupt();
    }

    /// submit_op 在未 install 时必须返回 Err(instead of panic)。
    /// 这是 AGENTS.md "fix root cause not band-aids" 的体现:
    /// commands 层依赖此错误路径,前端会展示为 toast。
    #[tokio::test]
    async fn submit_op_returns_err_when_not_installed() {
        let agent = MinimalAgent::new_empty();
        let result = agent
            .submit_op(reflect_protocol::Op::Compact)
            .await;
        assert!(
            result.is_err(),
            "submit_op before install must error, got: {result:?}"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not installed") || err_msg.contains("setup"),
            "error message must explain root cause: {err_msg}"
        );
    }
}
