//! Merge Markdown-parsed agent def with a TOML defaults layer.
//!
//! Mirrors reflect `agent_definitions.py:merge_definitions`. The
//! Markdown values WIN for any field that is set (non-`None` /
/// non-empty / non-default); TOML fills the `None` slots.
use toml::Value;

use crate::model::AgentDefinition;

/// Merge the Markdown-parsed `md` with a TOML defaults table. The
/// `toml` value is a `toml::value::Table` (i.e. a `toml::Value::Table`).
pub fn merge_with_toml(mut md: AgentDefinition, toml: Option<Value>) -> AgentDefinition {
    let Some(Value::Table(t)) = toml else {
        return md;
    };
    // name: TOML only if MD's is empty.
    if md.name.is_empty() {
        if let Some(Value::String(s)) = t.get("name") {
            md.name = s.clone();
        }
    }
    if md.description.is_empty() {
        if let Some(Value::String(s)) = t.get("description") {
            md.description = s.clone();
        }
    }
    if !md.spawnable {
        if let Some(Value::Boolean(b)) = t.get("spawnable") {
            md.spawnable = *b;
        }
    }
    if !md.readonly {
        if let Some(Value::Boolean(b)) = t.get("readonly") {
            md.readonly = *b;
        }
    }
    if md.tools.is_empty() {
        if let Some(arr) = t.get("tools").and_then(|v| v.as_array()) {
            md.tools = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    if md.disallowed_tools.is_empty() {
        if let Some(arr) = t.get("disallowed_tools").and_then(|v| v.as_array()) {
            md.disallowed_tools = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    if md.model.is_none() {
        if let Some(Value::String(s)) = t.get("model") {
            md.model = Some(s.clone());
        }
    }
    if md.max_turns.is_none() {
        if let Some(Value::Integer(n)) = t.get("max_turns") {
            md.max_turns = Some(*n as u32);
        }
    }
    if md.max_result_chars.is_none() {
        if let Some(Value::Integer(n)) = t.get("max_result_chars") {
            md.max_result_chars = Some(*n as usize);
        }
    }
    // system_prompt is always MD-only (no TOML equivalent).
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_agent_str;

    const MD: &str = "---\nname: from-md\ndescription: desc\n---\nMD body\n";

    #[test]
    fn merge_with_no_toml_returns_md_unchanged() {
        let md = parse_agent_str(MD).unwrap();
        let merged = merge_with_toml(md.clone(), None);
        assert_eq!(merged.name, "from-md");
        assert_eq!(merged.description, "desc");
        assert_eq!(merged.system_prompt, "MD body");
    }

    #[test]
    fn merge_toml_fills_none_slots() {
        let md = parse_agent_str(MD).unwrap();
        let toml: Value = toml::toml! {
            spawnable = true
            readonly = true
            max_turns = 50
            tools = ["read", "write"]
        }
        .into();
        let merged = merge_with_toml(md, Some(toml));
        // name/description still from MD.
        assert_eq!(merged.name, "from-md");
        assert_eq!(merged.description, "desc");
        // spawnable, readonly, max_turns, tools from TOML.
        assert!(merged.spawnable);
        assert!(merged.readonly);
        assert_eq!(merged.max_turns, Some(50));
        assert_eq!(merged.tools, vec!["read", "write"]);
        // system_prompt always MD.
        assert_eq!(merged.system_prompt, "MD body");
    }

    #[test]
    fn merge_md_wins_on_conflict() {
        let md = parse_agent_str(
            "---\nname: x\ndescription: d\nmax_turns: 100\nspawnable: true\n---\nbody",
        )
        .unwrap();
        let toml: Value = toml::toml! {
            max_turns = 10
            spawnable = false
        }
        .into();
        let merged = merge_with_toml(md, Some(toml));
        assert_eq!(merged.max_turns, Some(100));
        assert!(merged.spawnable);
    }

    #[test]
    fn merge_with_non_table_value_ignored() {
        let md = parse_agent_str(MD).unwrap();
        let merged = merge_with_toml(md.clone(), Some(Value::String("hi".into())));
        // Should be unchanged.
        assert_eq!(merged.name, md.name);
        assert_eq!(merged.system_prompt, md.system_prompt);
    }
}
