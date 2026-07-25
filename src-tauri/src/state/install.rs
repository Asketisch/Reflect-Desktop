//! `install_agent_thread` —— 二段构造的第二阶段。
//!
//! 必须跑在 tokio runtime 上下文(Tauri setup 闭包内或后续被
//! `tauri::async_runtime::spawn` 推迟的 task 内),因为 `AgentThread::new`
//! 内部立即 `tokio::spawn(submission_loop(...))`。
//!
//! 同时承载内置工具注册(`register_builtin_tools`)与 MCP/LSP bootstrap
//! 的 spawn — 后者异步推进,不阻塞 install 返回。

use std::sync::Arc;

use reflect_core::{AgentConfig, AgentThread};
use reflect_llm::SharedModelRegistry;
use reflect_tools::{
    Sanitizer, ToolRegistry,
    builtins::{
        BashTool, DeleteTool, EchoTool, EditTool, EnterPlanModeTool, EnterWorktreeTool,
        ExitPlanModeTool, ExitWorktreeTool, GlobTool, GrepTool, NotebookEditTool, ReadTool,
        ToolSearchTool, WebFetchTool, WebSearchTool, WriteTool,
    },
};

use super::MinimalAgent;

/// 第二阶段:在 Tauri setup 闭包内调用,真构造 AgentThread + 启动 forwarder。
///
/// ## 重要
///
/// `AgentThread::new` 内部立即 `tokio::spawn(submission_loop(...))`,
/// 必须跑在 tokio runtime 上下文。Tauri `setup` 闭包运行时 Tauri 已经
/// 初始化了它的 async_runtime,这一步安全。
pub(crate) fn install_agent_thread(agent: &MinimalAgent) {
    // 1. 解析模型 spec 与 provider。读 cfg + env 的真实优先级。
    let cfg_snapshot = agent.inner.cfg.read().clone();
    let workspace = agent.inner.workspace.clone();
    let model_spec = cfg_snapshot
        .resolved_model_spec()
        .unwrap_or_else(|| "stub/test".to_string());
    let has_provider = cfg_snapshot.active_provider().is_some();

    // 2. 构造共享 ToolRegistry —— 无论是否有 provider 都先注册,
    //    MCP/LSP 后续也在这个 registry 上 register_plugin_tool。
    let tools = agent.inner.tools.clone();
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
                *agent.inner.degraded_reason.lock() =
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
        *agent.inner.degraded_reason.lock() =
            Some("no provider configured — set an API key in Settings".to_string());
        tools.register(Arc::new(EchoTool));
        Arc::new(reflect_llm::ModelRegistry::new())
    };

    // 4. AgentConfig + AgentThread。
    let cfg = AgentConfig::new(model_spec.clone(), workspace.clone()).with_approvals(true);
    // sanitizer 用默认 10 pattern;AgentThread 内部传 None 也会走 with_defaults,
    // 这里显式构造便于后续接 [sanitize] config 段。
    let sanitizer = Arc::new(Sanitizer::with_defaults());
    let thread = Arc::new(AgentThread::new(
        cfg,
        registry,
        tools.clone(),
        Some(sanitizer),
    ));

    // 5. 写回 inner。
    *agent.inner.thread.lock() = Some(thread.clone());
    *agent.inner.model_spec.write() = model_spec.clone();

    // 6. 启动 session event forwarder。
    super::session::start_forwarder(agent);

    // 7. MCP / LSP bootstrap(阶段 3d)—— 读 cfg 的 [mcp_servers]/[lsp_servers],
    //    启动 server + 注册 tool。lifecycle event 经 session broadcast 推前端。
    //    在 async runtime 里跑(bootstrap 内部 tokio::spawn + 网络 I/O)。
    let cfg_for_bootstrap = cfg_snapshot.clone();
    let tools_for_bootstrap = tools.clone();
    let session_tx_for_bootstrap = agent.inner.session_tx.clone();
    let agent_clone = agent.clone();
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
        // 触发一次 unused warning 抑制(agent_clone 保留 future 扩展用)。
        let _ = &agent_clone;
    });

    tracing::info!(
        "[reflect-gui] AgentThread installed (model={}, workspace={}, degraded={})",
        model_spec,
        workspace.display(),
        !has_provider
    );
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
    use crate::state::MinimalAgent;

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
            assert!(
                names.contains(&expected.to_string()),
                "missing tool: {expected}"
            );
        }
    }

    /// install_agent_thread 必须在 tokio runtime 上下文里调用,
    /// 否则 AgentThread 内部 tokio::spawn 会 panic。
    /// 这里用 tokio::test 验证 install 后状态翻转 + tools 注册数正确。
    #[tokio::test]
    async fn install_populates_state() {
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
}
