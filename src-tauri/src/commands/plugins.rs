//! 插件命令列表 —— composer 斜杠菜单的数据源。
//!
//! 只读快照:从挂载好的插件运行时(`state/plugins.rs`)取 slash 命令注册表。
//! 未挂载(占位线程 / 无 enabled 插件 / HOME 缺失)时返回空列表,前端
//! 弹层自然只显示内置命令。前端选中后手输/插入 `/<name> args`,展开仍由
//! 后端 submit 边界统一做(`expand_submission`)—— 这里不做任何文件 IO。

use serde::Serialize;
use tauri::State;

use crate::commands::error::CommandResult;
use crate::state::MinimalAgent;

/// 单个插件 slash 命令的展示信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginCommandInfo {
    /// 命令全名(`plugin:ns:name` 形式,如 `demo:hello`)。前端以
    /// `/name args` 形态插入输入框,后端按同名展开。
    pub name: String,
    /// 命令说明(md frontmatter `description`,可缺省)。
    pub description: Option<String>,
}

#[tauri::command]
pub async fn reflect_list_plugin_commands(
    agent: State<'_, MinimalAgent>,
) -> CommandResult<Vec<PluginCommandInfo>> {
    let runtime = agent.plugin_runtime();
    let guard = runtime.lock().await;
    let Some(loaded) = guard.as_ref() else {
        return Ok(Vec::new());
    };
    Ok(command_infos(loaded.commands().list()))
}

/// LoadedCommand → 展示信息(纯映射,单测覆盖)。
fn command_infos(commands: Vec<reflect_plugin::LoadedCommand>) -> Vec<PluginCommandInfo> {
    commands
        .into_iter()
        .map(|c| PluginCommandInfo {
            name: c.name,
            description: c.description,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 映射保持 name/description,字段一一对应。
    #[test]
    fn maps_loaded_commands_to_infos() {
        let infos = command_infos(vec![reflect_plugin::LoadedCommand {
            name: "demo:hello".into(),
            file_path: PathBuf::from("/tmp/hello.md"),
            description: Some("say hi".into()),
        }]);
        assert_eq!(
            infos,
            vec![PluginCommandInfo {
                name: "demo:hello".into(),
                description: Some("say hi".into()),
            }]
        );
    }

    #[test]
    fn maps_empty_registry_to_empty_vec() {
        assert!(command_infos(vec![]).is_empty());
    }
}
