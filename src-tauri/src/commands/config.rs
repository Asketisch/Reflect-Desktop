//! Config + tool registry 命令。

use serde::Serialize;
use tauri::State;

use reflect_config::{default_config_path, load_from_str};

use crate::commands::error::{CommandError, CommandResult};
use crate::state::{AgentStatus, MinimalAgent};

/// 返回 agent 状态快照(ready / has_model / model / workspace / degraded_reason)。
/// 前端用来显示状态徽标 + 引导用户去 Settings 配 API key。
#[tauri::command]
pub async fn reflect_agent_status(agent: State<'_, MinimalAgent>) -> CommandResult<AgentStatus> {
    Ok(agent.agent_status())
}

/// 返回当前 `~/.reflect/config.toml` 的 TOML 字符串。Settings 页加载用。
#[tauri::command]
pub async fn reflect_get_config(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    let cfg = agent.cfg();
    let toml = toml::to_string_pretty(&*cfg.read()).map_err(|e| CommandError {
        msg: format!("serialize config: {e}"),
    })?;
    Ok(toml)
}

/// 写回 `~/.reflect/config.toml`。写盘前用 `load_from_str` 校验合法性,
/// 防止坏 TOML 损坏配置;校验通过才覆盖,并热更新共享 cfg。
///
/// 注意:**不**在这里直接重建 ModelRegistry / 重连 MCP —— 那些留给
/// `ConfigWatcher` 热重载流程(阶段 4 接入)。
#[tauri::command]
pub async fn reflect_save_config(
    agent: State<'_, MinimalAgent>,
    toml: String,
) -> CommandResult<()> {
    // 1. 校验:能否解析回 ReflectConfig。
    let new_cfg = load_from_str(&toml)?;
    // 2. 写盘。
    let path = default_config_path().ok_or_else(|| CommandError {
        msg: "no HOME dir for config".into(),
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &toml)?;
    // 3. 热更新共享 cfg(下一个 turn 读最新值)。
    *agent.cfg().write() = new_cfg;
    tracing::info!(
        "[reflect-gui] config saved to {} (hot-reloaded in-memory)",
        path.display()
    );
    Ok(())
}

/// 列出当前 ToolRegistry 中所有工具(name + description)。
/// 前端 Settings / Skills 页展示可用工具列表用。
#[tauri::command]
pub async fn reflect_list_tools(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<ToolInfo>> {
    let tools = agent.tools();
    let mut out: Vec<ToolInfo> = tools
        .list()
        .into_iter()
        .filter_map(|name| {
            let t = tools.get(&name)?;
            Some(ToolInfo {
                name,
                description: t.description().to_string(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 单个工具的 name + description。
#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}
