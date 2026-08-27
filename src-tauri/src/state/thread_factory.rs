//! `construct_thread` —— 把 install 与 rebind 共享的 AgentThread 构造路径
//! 抽到一处,避免两份分叉的 registry/thread 构造逻辑各自漂移。
//!
//! 调用方:
//! - `state/install.rs::install_agent_thread`:首次构造,`sid = None`,
//!   `preload = []`(走随机 session_id 的占位 thread,供 `reflect_bind_session`
//!   重建前作为兜底)。
//! - `state/rebind.rs::rebind_session`:每次切换会话,`sid = Some(...)`,
//!   `preload` 由 `records_to_preload` 从 rollout replay 算出。
//!
//! 持久化:recorder 绑定的 base 目录与 `commands/sessions.rs::sessions_base`
//! 一致(都是 `reflect_rollout::path::default_base()`),保证 record 与
//! replay 同源 —— 这是为什么我们替换了原 `dirs::home_dir().join(...)` 的
//! 直接拼装。

use std::sync::Arc;

use reflect_core::{AgentConfig, AgentThread};
use reflect_llm::{ChatMessage, ModelRegistry, SharedModelRegistry};
use reflect_protocol::ThreadId;
use reflect_tools::{Sanitizer, ToolRegistry, builtins::EchoTool};
use tracing::warn;

use super::MinimalAgent;

/// 构造共享 `ModelRegistry`。逻辑原 `install.rs:47-70` 整体搬过来:
/// provider 配置齐 → `cfg.to_registry()`;否则降级到空 registry + EchoTool,
/// 让命令层不依赖 API key 也能启动(但实际 chat 跑不动)。
///
/// `tools` 在降级分支里会被注册 EchoTool;调用方**之前**已注册内置工具。
pub(crate) fn build_registry(
    cfg_snapshot: &reflect_config::ReflectConfig,
    tools: Arc<ToolRegistry>,
) -> SharedModelRegistry {
    let has_provider = cfg_snapshot.active_provider().is_some();
    if has_provider {
        match cfg_snapshot.to_registry() {
            Ok(r) => Arc::new(r),
            Err(e) => {
                warn!("[reflect-gui] to_registry failed ({}); falling back to empty registry", e);
                Arc::new(ModelRegistry::new())
            }
        }
    } else {
        warn!(
            "[reflect-gui] no provider configured (set OPENAI_API_KEY / ANTHROPIC_API_KEY \
             or edit ~/.reflect/config.toml); running in degraded mode"
        );
        tools.register(Arc::new(EchoTool));
        Arc::new(ModelRegistry::new())
    }
}

/// 构造 `AgentThread`,把 recorder / session_id / preload 一并接进 config。
///
/// `sid == None` 时不挂 recorder 也不固定 id —— 这是 install 阶段的占位
/// thread,首条 submission 触发 `reflect_bind_session` 后再换为带 recorder
/// 的真 thread。
pub(crate) fn construct_thread(
    agent: &MinimalAgent,
    sid: Option<ThreadId>,
    preload: Vec<ChatMessage>,
) -> anyhow::Result<Arc<AgentThread>> {
    let cfg_snapshot = agent.inner.cfg.read().clone();
    let model_spec = cfg_snapshot
        .resolved_model_spec()
        .unwrap_or_else(|| "stub/test".to_string());
    let registry = agent
        .inner
        .model_registry
        .lock()
        .clone()
        .expect("model_registry 必须在 install 阶段已建");

    let sanitizer = Arc::new(Sanitizer::with_defaults());
    let hook_engine: Arc<reflect_hooks::HookEngine> = Arc::new(
        reflect_hooks::config::HooksConfig::from_reflect_section(&cfg_snapshot.hooks).build_engine(),
    );

    // 持久化:仅在 sid = Some 时挂 recorder + 固定 id。bind 命令前不挂,
    // 避免随机 id 产生空文件污染 `~/.reflect/sessions/`。
    let mut cfg = AgentConfig::new(model_spec, agent.workspace()).with_approvals(true);
    if let Some(sid) = sid {
        let m4 = match reflect::m4_bootstrap::build_default_m4(
            &agent.workspace(),
            "desktop",
            &cfg.current_model(),
            &registry,
            sid,
        ) {
            Ok(mut m4) => {
                m4.recorder = Some(Arc::new(reflect_rollout::JsonlRolloutWriter::new(
                    reflect_rollout::path::default_base(),
                    sid,
                )));
                m4
            }
            Err(e) => {
                // 持久化是承诺而非 nice-to-have:失败必须可见。
                anyhow::bail!("build_default_m4 失败,recorder 接线回退为无持久化: {e:#}");
            }
        };
        cfg = cfg.with_m4(m4).with_session_id(sid);
    }
    if !preload.is_empty() {
        cfg = cfg.with_preload_messages(preload);
    }

    Ok(Arc::new(AgentThread::new(
        cfg,
        registry,
        agent.inner.tools.clone(),
        Some(sanitizer),
        Some(hook_engine),
    )))
}
