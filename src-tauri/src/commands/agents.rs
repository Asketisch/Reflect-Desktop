//! Agent definition management commands — wraps `vendor/reflect-agent-def`.
//!
//! Phase 1 item 3: agent profile management UI backing. Each agent is a
//! Markdown file with YAML frontmatter at `~/.reflect/agents/<name>.md`,
//! shared with the TUI/CLI.
//!
//! ## 命令清单
//!
//! - `reflect_list_agent_defs` — list all definitions (sorted by name)
//! - `reflect_get_agent_def(name)` — single definition
//! - `reflect_save_agent_def(def)` — create / update (serializes to `.md`)
//! - `reflect_delete_agent_def(name)` — delete file
//! - `reflect_parse_agent_md(text)` — preview / validate a markdown string
//!
//! ## 序列化
//!
//! `reflect-agent-def` 只提供 parser(`parse_agent_str`),没有内置
//! to-markdown 序列化。`reflect_save_agent_def` 在命令层手动构造
//! frontmatter(YAML,跳过 `system_prompt`)+ body,保证 round-trip 通过
//! `parse_agent_str`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use reflect_agent_def::{load_agents_dir, parse_agent_str, AgentDefinition};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::error::{CommandError, CommandResult};
use crate::state::MinimalAgent;

/// `~/.reflect/agents/` 目录(存 agent 定义 markdown 文件)。
fn agents_dir() -> CommandResult<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| CommandError {
        msg: "no home dir".into(),
    })?;
    Ok(home.join(".reflect/agents"))
}

/// 把 `AgentDefinition` 序列化为 markdown(frontmatter + body)。
///
/// 手动实现而非 `serde_yaml::to_string` 整个结构,因为 `system_prompt` 字段
/// 必须作为 body(在 frontmatter 之后),不能出现在 YAML 块里。
fn serialize_agent_md(def: &AgentDefinition) -> CommandResult<String> {
    // 构造 frontmatter 的 YAML 视图(排除 system_prompt)。
    #[derive(Serialize)]
    struct Frontmatter {
        name: String,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        spawnable: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        readonly: Option<bool>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        tools: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        disallowed_tools: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_turns: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_result_chars: Option<usize>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        memory: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        mcp_collections: Vec<String>,
    }

    let memory_str: Vec<String> = def
        .memory
        .iter()
        .map(|s| s.to_string().to_lowercase())
        .collect();

    let fm = Frontmatter {
        name: def.name.clone(),
        description: def.description.clone(),
        spawnable: if def.spawnable { Some(true) } else { None },
        readonly: if def.readonly { Some(true) } else { None },
        tools: def.tools.clone(),
        disallowed_tools: def.disallowed_tools.clone(),
        model: def.model.clone(),
        max_turns: def.max_turns,
        // 协议升级:`AgentDefinition` 移除了 `max_result_chars` 字段。
        // 该 DTO 字段保留给前端(可选),此处用 None 占位。
        max_result_chars: None,
        memory: memory_str,
        mcp_collections: def.mcp_collections.clone(),
    };

    let yaml = serde_yaml::to_string(&fm).map_err(|e| CommandError {
        msg: format!("agent def yaml serialize: {e}"),
    })?;

    Ok(format!("---\n{yaml}---\n{}\n", def.system_prompt))
}

/// 计算目标文件路径:`<agents_dir>/<name>.md`。校验 name 防 path traversal。
fn path_for(name: &str, dir: &Path) -> CommandResult<PathBuf> {
    if name.is_empty() {
        return Err(CommandError {
            msg: "agent name cannot be empty".into(),
        });
    }
    // 防 `../` / 绝对路径 / 路径分隔符 / NUL。只拦截真的路径穿越,
    // 允许 `foo..bar` 这样的中段连续 dot(不会在 join 后逃出 dir)。
    if name.contains('/')
        || name.contains('\\')
        || name == ".."
        || name.starts_with("../")
        || name.starts_with("..\\")
        || name.contains('\0')
    {
        return Err(CommandError {
            msg: format!("agent name contains invalid characters: {name:?}"),
        });
    }
    Ok(dir.join(format!("{name}.md")))
}

/// List all agent definitions (sorted by name).
#[tauri::command]
pub async fn reflect_list_agent_defs(
    _agent: State<'_, MinimalAgent>,
) -> CommandResult<Vec<AgentDefinition>> {
    let dir = agents_dir()?;
    let map: HashMap<String, AgentDefinition> = load_agents_dir(&dir).map_err(CommandError::from)?;
    let mut v: Vec<AgentDefinition> = map.into_values().collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

/// Read a single agent definition by name. Throws on not-found.
#[tauri::command]
pub async fn reflect_get_agent_def(
    _agent: State<'_, MinimalAgent>,
    name: String,
) -> CommandResult<AgentDefinition> {
    let dir = agents_dir()?;
    let path = path_for(&name, &dir)?;
    let raw = std::fs::read_to_string(&path).map_err(|e| CommandError {
        msg: format!("agent def '{name}' not found: {e}"),
    })?;
    parse_agent_str(&raw).map_err(CommandError::from)
}

/// Create or update an agent definition. Serializes to `<agents_dir>/<name>.md`.
#[tauri::command]
pub async fn reflect_save_agent_def(
    _agent: State<'_, MinimalAgent>,
    def: AgentDefinition,
) -> CommandResult<AgentDefinition> {
    if def.name.is_empty() {
        return Err(CommandError {
            msg: "agent def name is required".into(),
        });
    }
    if def.description.is_empty() {
        return Err(CommandError {
            msg: "agent def description is required".into(),
        });
    }
    let dir = agents_dir()?;
    std::fs::create_dir_all(&dir).map_err(CommandError::from)?;
    let path = path_for(&def.name, &dir)?;
    let md = serialize_agent_md(&def)?;
    std::fs::write(&path, md).map_err(CommandError::from)?;
    // round-trip 校验:写回后重解析,确认可读。
    let raw = std::fs::read_to_string(&path).map_err(CommandError::from)?;
    let back = parse_agent_str(&raw).map_err(CommandError::from)?;
    Ok(back)
}

/// Delete an agent definition file. Returns `true` if deleted, `false` if not found.
#[tauri::command]
pub async fn reflect_delete_agent_def(
    _agent: State<'_, MinimalAgent>,
    name: String,
) -> CommandResult<bool> {
    let dir = agents_dir()?;
    let path = path_for(&name, &dir)?;
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path).map_err(CommandError::from)?;
    Ok(true)
}

/// Parse / validate a markdown string as an agent definition (no side effects).
/// Used by the editor for live preview / validation before save.
#[tauri::command]
pub async fn reflect_parse_agent_md(
    _agent: State<'_, MinimalAgent>,
    text: String,
) -> CommandResult<AgentDefinition> {
    parse_agent_str(&text).map_err(CommandError::from)
}

/// 顶层 config/patch 类型(供前端做 partial update;当前未直接用,留作扩展)。
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentDefPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<Option<String>>,
    pub tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub spawnable: Option<bool>,
    pub readonly: Option<bool>,
    pub max_turns: Option<Option<u32>>,
}

#[cfg(test)]
mod tests {
    //! 命令包装层测试。沿用 sessions.rs / tasks.rs / schedule.rs 策略:
    //! 不 spin Tauri runtime,直接测核心逻辑(序列化 / 路径校验 / round-trip)。

    use super::*;
    use reflect_agent_def::AgentDefinition;

    #[test]
    fn serialize_round_trips_through_parser() {
        let def = AgentDefinition {
            name: "code-reviewer".into(),
            description: "Reviews code changes".into(),
            spawnable: true,
            readonly: true,
            tools: vec!["read".into(), "grep".into()],
            disallowed_tools: vec!["bash".into()],
            model: Some("inherit".into()),
            max_turns: Some(30),
            max_result_chars: Some(3000),
            memory: vec![],
            mcp_collections: vec![],
            system_prompt: "# Code Reviewer\n\nYou are strict.".into(),
        };
        let md = serialize_agent_md(&def).unwrap();
        // 必须有 frontmatter 标记 + body。
        assert!(md.starts_with("---\n"));
        assert!(md.contains("name: code-reviewer"));
        assert!(md.contains("spawnable: true"));
        assert!(md.contains("# Code Reviewer"));

        let back = parse_agent_str(&md).unwrap();
        assert_eq!(back.name, "code-reviewer");
        assert!(back.spawnable);
        assert!(back.readonly);
        assert_eq!(back.tools, vec!["read", "grep"]);
        assert_eq!(back.model.as_deref(), Some("inherit"));
        assert!(back.system_prompt.contains("You are strict."));
    }

    #[test]
    fn serialize_minimal_omits_optional_fields() {
        let def = AgentDefinition {
            name: "minimal".into(),
            description: "just enough".into(),
            ..Default::default()
        };
        let md = serialize_agent_md(&def).unwrap();
        assert!(!md.contains("spawnable"));
        assert!(!md.contains("tools:"));
        assert!(!md.contains("model:"));
    }

    #[test]
    fn path_for_rejects_traversal() {
        let dir = Path::new("/tmp");
        assert!(path_for("../etc/passwd", dir).is_err());
        assert!(path_for("a/b", dir).is_err());
        assert!(path_for("a\\b", dir).is_err());
        assert!(path_for("", dir).is_err());
        assert!(path_for("good-name", dir).is_ok());
    }

    #[test]
    fn path_for_allows_inner_dots() {
        let dir = Path::new("/tmp");
        // `..` only mid-name does NOT escape the directory; the join stays
        // inside `dir`.
        assert!(path_for("foo..bar", dir).is_ok());
        assert!(path_for("..foo", dir).is_ok());
        assert!(path_for("foo..", dir).is_ok());
    }

    #[test]
    fn path_for_builds_correct_filename() {
        let dir = Path::new("/tmp/agents");
        let p = path_for("reviewer", dir).unwrap();
        assert_eq!(p, PathBuf::from("/tmp/agents/reviewer.md"));
    }
}
