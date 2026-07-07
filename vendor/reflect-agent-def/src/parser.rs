//! YAML frontmatter parser + dir scanner.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::model::{AgentDefError, AgentDefinition};

/// Parse an agent definition from a Markdown string. The frontmatter is
/// the YAML block delimited by `---` lines at the top of the file.
pub fn parse_agent_str(input: &str) -> Result<AgentDefinition, AgentDefError> {
    let (front, body) = split_frontmatter(input).ok_or(AgentDefError::NoFrontmatter)?;
    let mut def: AgentDefinition = if front.trim().is_empty() {
        AgentDefinition::default()
    } else {
        serde_yaml::from_str(front)?
    };
    def.system_prompt = body.trim().to_string();
    validate(&def)?;
    Ok(def)
}

/// Parse a single agent definition from a file.
pub fn parse_agent_md(path: &Path) -> Result<AgentDefinition, AgentDefError> {
    let raw = fs::read_to_string(path)?;
    parse_agent_str(&raw)
}

/// Walk a directory and parse every `*.md` file as an agent definition.
/// Returned map is keyed by `def.name`. Files that fail to parse are
/// logged via `tracing::warn!` and skipped (rather than aborting the
/// whole load).
pub fn load_agents_dir(dir: &Path) -> Result<HashMap<String, AgentDefinition>, AgentDefError> {
    let mut out = HashMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        match parse_agent_md(&path) {
            Ok(def) => {
                out.insert(def.name.clone(), def);
            }
            Err(e) => {
                tracing::warn!(?path, ?e, "failed to load agent definition; skipping");
            }
        }
    }
    Ok(out)
}

/// Split the input into `(frontmatter, body)`. Frontmatter is the YAML
/// block between the first and second `---` line markers.
fn split_frontmatter(input: &str) -> Option<(&str, &str)> {
    // Must start with `---`. After stripping, the rest is `after_open`.
    let after_open = input.strip_prefix("---")?;
    // Walk lines until we find a line that starts with `---`.
    let mut rest_start: Option<usize> = None;
    let mut offset = 0usize;
    for line in after_open.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        if line.trim_start().starts_with("---") {
            rest_start = Some(line_start);
            break;
        }
    }
    let line_start = rest_start?;
    // Frontmatter is the YAML between the first and second `---`.
    let front = &after_open[..line_start];
    // Body starts after the closing `---` line (and its trailing \n).
    let body = &after_open[offset..];
    Some((front, body))
}

fn validate(def: &AgentDefinition) -> Result<(), AgentDefError> {
    if def.name.is_empty() {
        return Err(AgentDefError::MissingField("name"));
    }
    if def.description.is_empty() {
        return Err(AgentDefError::MissingField("description"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str =
        "---\nname: test\ndescription: A test agent\n---\nYou are a test agent.\n";

    #[test]
    fn parse_minimal_frontmatter() {
        let def = parse_agent_str(MINIMAL).unwrap();
        assert_eq!(def.name, "test");
        assert_eq!(def.description, "A test agent");
        assert_eq!(def.system_prompt, "You are a test agent.");
    }

    #[test]
    fn parse_all_optional_fields() {
        let input = r#"---
name: full
description: full agent
spawnable: true
readonly: true
tools: [read, grep]
disallowed_tools: [bash]
model: inherit
max_turns: 30
max_result_chars: 3000
memory: [project, user]
mcp_collections: [$default]
---
# System prompt body
with **markdown**
"#;
        let def = parse_agent_str(input).unwrap();
        assert_eq!(def.name, "full");
        assert!(def.spawnable);
        assert!(def.readonly);
        assert_eq!(def.tools, vec!["read", "grep"]);
        assert_eq!(def.disallowed_tools, vec!["bash"]);
        assert_eq!(def.model.as_deref(), Some("inherit"));
        assert_eq!(def.max_turns, Some(30));
        assert_eq!(def.max_result_chars, Some(3000));
        assert_eq!(def.memory.len(), 2);
        assert!(def.system_prompt.contains("**markdown**"));
    }

    #[test]
    fn missing_name_errors() {
        let input = "---\ndescription: x\n---\nbody";
        let err = parse_agent_str(input).unwrap_err();
        assert!(matches!(err, AgentDefError::MissingField("name")));
    }

    #[test]
    fn missing_description_errors() {
        let input = "---\nname: x\n---\nbody";
        let err = parse_agent_str(input).unwrap_err();
        assert!(matches!(err, AgentDefError::MissingField("description")));
    }

    #[test]
    fn no_frontmatter_errors() {
        let err = parse_agent_str("just body text").unwrap_err();
        assert!(matches!(err, AgentDefError::NoFrontmatter));
    }

    #[test]
    fn body_with_frontmatter_in_body_preserved() {
        let input = "---\nname: t\ndescription: d\n---\n# Body\n\n---\n\nthis looks like a frontmatter but it's body\n";
        let def = parse_agent_str(input).unwrap();
        assert!(def.system_prompt.contains("this looks like a frontmatter"));
    }

    #[test]
    fn load_agents_dir_missing_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("nonexistent");
        let map = load_agents_dir(&sub).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn load_agents_dir_parses_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.md"),
            "---\nname: a\ndescription: aa\n---\nbody a\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("b.md"),
            "---\nname: b\ndescription: bb\n---\nbody b\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("ignore.txt"), "not md").unwrap();
        let map = load_agents_dir(dir.path()).unwrap();
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("a"));
        assert!(map.contains_key("b"));
    }

    #[test]
    fn load_agents_dir_skips_invalid_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.md"), "no frontmatter").unwrap();
        let map = load_agents_dir(dir.path()).unwrap();
        assert!(map.is_empty());
    }
}
