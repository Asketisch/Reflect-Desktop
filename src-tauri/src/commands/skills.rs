//! Skills 列表 + 轻量 YAML frontmatter 解析。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::commands::error::CommandResult;

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: String,
    pub tools: Vec<String>,
    pub triggers: Vec<String>,
}

#[tauri::command]
pub async fn reflect_list_skills() -> CommandResult<Vec<SkillInfo>> {
    // 扫描 `~/.reflect/skills/**/SKILL.md` 与 `<cwd>/.reflect/skills/**/SKILL.md`。
    let mut out = Vec::new();
    let search_dirs: Vec<PathBuf> = [
        dirs::home_dir().map(|h| h.join(".reflect/skills")),
        std::env::current_dir()
            .ok()
            .map(|c| c.join(".reflect/skills")),
    ]
    .into_iter()
    .flatten()
    .collect();
    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }
        collect_skills_in(&dir, &mut out)?;
    }
    Ok(out)
}

fn collect_skills_in(dir: &PathBuf, out: &mut Vec<SkillInfo>) -> CommandResult<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // file_type() 基于 symlink_metadata,不跟随链接:目录符号链接
        // 可能成环(或指回祖先目录),is_dir() 会跟随导致无限递归 abort。
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            collect_skills_in(&path, out)?;
        } else if path.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(info) = parse_skill_frontmatter(&content, &path) {
                    out.push(info);
                }
            }
        }
    }
    Ok(())
}

fn parse_skill_frontmatter(content: &str, path: &Path) -> Option<SkillInfo> {
    // 极简 YAML frontmatter 解析器:抽取首个 `---\n...\n---` 块。
    let stripped = content.strip_prefix("---")?;
    let rest = stripped.trim_start_matches('\n');
    let end = rest.find("\n---")?;
    let yaml = &rest[..end];
    let body = rest[end + 4..].trim();

    let name = extract_yaml_field(yaml, "name").unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    });
    let description = extract_yaml_field(yaml, "description").unwrap_or_default();
    let tools = extract_yaml_list(yaml, "tools");
    let triggers = extract_yaml_list(yamml_safe(yaml), "triggers");

    let _ = body; // body 在 summary 中暂未使用,留给未来 `/skills/<name>` 详情页。

    Some(SkillInfo {
        name,
        description,
        path: path.display().to_string(),
        tools,
        triggers,
    })
}

fn yamml_safe(s: &str) -> &str {
    s
}

fn extract_yaml_field(yaml: &str, key: &str) -> Option<String> {
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn extract_yaml_list(yaml: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_list = false;
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}:")) {
            // 行内列表 `[a, b, c]`
            if rest.trim_start().starts_with('[') {
                let inside = rest
                    .trim_start()
                    .trim_start_matches('[')
                    .trim_end_matches(']');
                for item in inside.split(',') {
                    let s = item.trim().trim_matches('"').trim_matches('\'');
                    if !s.is_empty() {
                        out.push(s.to_string());
                    }
                }
                in_list = false;
            } else if rest.trim().is_empty() {
                in_list = true;
            } else {
                return out; // 单值,不算列表
            }
            continue;
        }
        if in_list {
            if let Some(item) = trimmed.strip_prefix("- ") {
                let s = item.trim().trim_matches('"').trim_matches('\'');
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            } else if !trimmed.is_empty() {
                in_list = false;
            }
        }
    }
    out
}
