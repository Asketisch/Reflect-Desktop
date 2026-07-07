//! `AppState` —— M2.x 真后端：直接嵌入 `reflect_core::AgentThread`。
//!
//! ## 进度
//!
//! - [x] **M1.2 协议桥 (MinimalAgent stub)**:证明 Submission→Event 通路。
//! - [x] **M2.x 真后端**:
//!     - 用 `reflect_core::AgentThread::new(...)` 取代手撸 submission loop;
//!     - Model 路由 = `ModelRegistry::new()` (空注册表 → 提交后会 emit
//!       `EventMsg::Error("no model client for spec ...")` 作为 smoke 验证);
//!     - Tools = `EchoTool` 单条占位 (M3.x 接 22 builtin);
//!     - Session / per-turn event 通过 `tokio::sync::broadcast` 多订阅 fan-out。
//!     - 不依赖网络 / API key / `~/.reflect/config.toml`,纯本地 stub 后端。
//!
//! ## 替换语义
//!
//! 公开 API (`new_empty` / `install_agent_thread` / `start` / `submit` /
//! `subscribe_session` / `interrupt` / `model_spec` / `workspace`)
//! 与 M1.x 同形,`commands/mod.rs` 与 `lib.rs` 不需要改协议面。

use std::path::PathBuf;
use std::sync::Arc;

use reflect_core::{AgentConfig, AgentThread, TurnHandle};
use reflect_llm::ModelRegistry;
use reflect_protocol::Event;
use reflect_tools::{builtins::EchoTool, ToolRegistry};
use tokio::sync::broadcast;

/// Session-level broadcast 通道容量。覆盖 1024 个 event 后最旧事件被丢;
/// 桌面端 UI 通常只关心最近 50-100 个事件,这个容量远超实际需要。
const SESSION_BROADCAST_CAPACITY: usize = 1024;

/// 主结构：封装一个 `AgentThread` + session 级 broadcast + interrupt token。
///
/// Clone 是廉价的（内部全 Arc/broadcast channel）。
///
/// M2.x 二段构造:
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
    /// 模型 spec 字符串 —— 仅用于诊断与 Tauri command 反馈。
    model_spec: String,
    /// 工作区根 —— 当前固定 cwd。M3.x 由前端 settings 切换。
    workspace: PathBuf,
}

use parking_lot::Mutex as ParkingMutex;

impl MinimalAgent {
    /// 第一阶段:构造空的 MinimalAgent,只能 hold 状态(无 thread)。
    ///
    /// `tauri::Builder::manage(MinimalAgent::new_empty())` 时调用。
    pub fn new_empty() -> Self {
        let (session_tx, _) = broadcast::channel(SESSION_BROADCAST_CAPACITY);
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            inner: Arc::new(MinimalAgentInner {
                thread: ParkingMutex::new(None),
                session_tx,
                model_spec: "stub/test".to_string(),
                workspace,
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
        // 1. 配置:stub 模型 spec + 当前 cwd。
        let model_spec = "stub/test".to_string();
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let cfg = AgentConfig::new(model_spec.clone(), workspace.clone());

        // 2. 空 ModelRegistry —— 没有 model client,提交后 emit Error event,
        //    作为 M2.x smoke test 的预期路径。
        let registry = Arc::new(ModelRegistry::new());

        // 3. ToolRegistry: 注册 EchoTool。M3.x 接入 22 builtin (Bash/Read/Write/...)。
        let tools = Arc::new(ToolRegistry::default());
        tools.register(Arc::new(EchoTool));

        // 4. 真 AgentThread —— 这里会 tokio::spawn submission_loop。
        let thread = Arc::new(AgentThread::new(cfg, registry, tools, None));

        // 5. 写回 inner.thread。
        *self.inner.thread.lock() = Some(thread);

        // 6. 启动 session event forwarder (AgentThread.subscribe_session → broadcast)。
        self.start();

        tracing::info!("[reflect-gui] AgentThread installed (model={}, workspace={})", model_spec, workspace.display());
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
            guard.clone().ok_or_else(|| {
                anyhow::anyhow!("agent thread not installed yet; setup not complete")
            })?
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

    /// 订阅 session 级 broadcast。前端 Tauri listener 拿到的就是这个 receiver。
    pub fn subscribe_session(&self) -> broadcast::Receiver<Event> {
        self.inner.session_tx.subscribe()
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
    pub fn model_spec(&self) -> &str {
        &self.inner.model_spec
    }

    /// 诊断接口:返回当前 workspace (供前端显示)。
    pub fn workspace(&self) -> &PathBuf {
        &self.inner.workspace
    }
}