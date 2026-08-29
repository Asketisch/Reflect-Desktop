//! LSP 开关 / 状态 / 文件预热命令。
//!
//! 设计(对齐 VSCode 的按需模型):
//! - LSP **默认关闭**,且按工作区由用户手动开启 —— 多个项目同时撑起
//!   多份语言服务会加重系统负担;
//! - 未开启时不注册 `lsp` tool(不给模型注入用不到的工具);
//! - 开启后只对**用户真正打开的代码文件**做预热(`ensure_open` → didOpen,
//!   server 侧建立索引),未打开的文件不预热。
//!
//! 开启状态的前端持久化(localStorage,按 workspace 路径)在 GUI 层;
//! 后端只负责当前生效实例的启停。

use serde::Serialize;
use std::str::FromStr;

use tauri::State;
use lsp_types::Uri;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// LSP 当前状态。
#[derive(Debug, Clone, Serialize)]
pub struct LspStatus {
    /// 是否已开启(注册了 `lsp` tool 且 manager 存活)。
    pub enabled: bool,
    /// 已启动的 server 名单。
    pub servers: Vec<String>,
}

/// 文件预热结果。
#[derive(Debug, Clone, Serialize)]
pub struct LspWarmupResult {
    /// 是否完成了预热(false = LSP 未开启 / 无匹配 server)。
    pub warmed: bool,
    /// 命中的 server 名(warmed 时)。
    pub server: Option<String>,
}

async fn status(agent: &MinimalAgent) -> LspStatus {
    let mgr = agent.inner.lsp_manager.lock().clone();
    match mgr {
        Some(m) => LspStatus {
            enabled: true,
            servers: m.server_names().await,
        },
        None => LspStatus {
            enabled: false,
            servers: vec![],
        },
    }
}

/// 开启 / 关闭 LSP(按当前工作区生效)。
///
/// 开启 = bootstrap(`[lsp_servers]` 配置 → 注册 `lsp` tool + 起 server),
/// 幂等;关闭 = 反注册 `lsp` tool + 停掉全部 server 并释放 manager。
#[tauri::command]
pub async fn reflect_lsp_set_enabled(
    agent: State<'_, MinimalAgent>,
    enabled: bool,
) -> CommandResult<LspStatus> {
    if enabled {
        if agent.inner.lsp_manager.lock().is_none() {
            let cfg = agent.cfg().read().clone();
            crate::mcp::bootstrap_lsp(&agent, &cfg)
                .await
                .map_err(CommandError::from)?;
        }
    } else {
        // 先 take(锁守卫不跨 await),再异步 shutdown。
        let mgr = agent.inner.lsp_manager.lock().take();
        if let Some(mgr) = mgr {
            agent.tools().unregister("lsp");
            mgr.shutdown().await;
            tracing::info!("[reflect-gui] LSP disabled: lsp tool unregistered, servers stopped");
        }
    }
    Ok(status(&agent).await)
}

/// 查询 LSP 状态(前端恢复开关 / 徽标展示用)。
#[tauri::command]
pub async fn reflect_lsp_status(agent: State<'_, MinimalAgent>) -> CommandResult<LspStatus> {
    Ok(status(&agent).await)
}

/// 预热一个代码文件:对命中的 server 做 `didOpen`(读盘 + 建立索引),
/// 让后续 agent 的 LSP 查询(hover / definition / diagnostics)即时可用。
/// LSP 未开启或无匹配 server 时不做事(warmed = false),**不报错**——
/// 预热是尽力而为的后台优化。
#[tauri::command]
pub async fn reflect_lsp_warmup(
    agent: State<'_, MinimalAgent>,
    path: String,
) -> CommandResult<LspWarmupResult> {
    let mgr = agent.inner.lsp_manager.lock().clone();
    let Some(mgr) = mgr else {
        return Ok(LspWarmupResult { warmed: false, server: None });
    };
    let abs = crate::commands::files::resolve_under_workspace(&agent.workspace(), &path)?;
    let handles = mgr.all_handles().await;
    if handles.is_empty() {
        return Ok(LspWarmupResult { warmed: false, server: None });
    }
    let workspace = agent.workspace();
    let Some(route) =
        reflect_lsp::matching::pick_server_for(&abs, &workspace, &handles, None)
    else {
        return Ok(LspWarmupResult { warmed: false, server: None });
    };
    let client = mgr.get_client(&route.server_name).await.ok_or_else(|| CommandError {
        msg: format!("lsp server {} disappeared during warmup", route.server_name),
    })?;
    let uri = file_uri(&abs)?;
    client
        .ensure_open(&abs, &uri, &route.language_id)
        .await
        .map_err(|e| CommandError { msg: format!("lsp warmup: {e}") })?;
    tracing::info!(server = %route.server_name, path = %abs.display(), "file warmed");
    Ok(LspWarmupResult {
        warmed: true,
        server: Some(route.server_name),
    })
}

/// 绝对路径 → LSP `file://` URI。与 reflect-lsp `tool.rs::path_to_uri`
/// 同款 `url::Url` 中转(该 helper 非 pub,应用层平移一份)。
fn file_uri(path: &std::path::Path) -> Result<Uri, CommandError> {
    let url = url::Url::from_file_path(path)
        .map_err(|()| CommandError { msg: "file:// conversion failed (path not absolute)".into() })?;
    Uri::from_str(url.as_str()).map_err(|e| CommandError { msg: format!("uri parse: {e}") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_roundtrip() {
        let uri = file_uri(std::path::Path::new("/tmp/x/a b.rs")).unwrap();
        assert!(uri.as_str().starts_with("file:///tmp/x/a%20b.rs") || uri.as_str().starts_with("file:///tmp/x/a b.rs"));
        // 相对路径必须报错,不允许静默发出错误 URI。
        assert!(file_uri(std::path::Path::new("relative/x.py")).is_err());
    }
}
