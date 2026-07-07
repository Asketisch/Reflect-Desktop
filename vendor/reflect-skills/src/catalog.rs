//! `SkillsCatalog` — owns the parsed skill list + the activated set.
//!
//! Used by `pre_loop` to render the catalog and by `LoadSkillTool` to
//! mark a skill as activated (which exposes its declared `tools:` to
//! the LLM tool schema).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;

use crate::loader::parse_skill_file;
use crate::model::{SkillError, SkillMeta};
use crate::scanner::scan_skills_dirs;

/// Default "always-on" tool names. These tools are visible to the LLM
/// even when no skill is activated.
pub const ALWAYS_ON_TOOLS: &[&str] = &["bash", "read", "write", "edit", "grep", "glob"];

/// Thread-safe catalog of available skills + the set of currently
/// activated skills (mutated by `LoadSkillTool::execute`).
pub struct SkillsCatalog {
    /// 技能列表;`RwLock` 以便 plugin loader 在 `Arc` 共享下追加/移除。
    skills: RwLock<Vec<SkillMeta>>,
    activated: RwLock<HashSet<String>>,
    always_on: HashSet<String>,
}

impl std::fmt::Debug for SkillsCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillsCatalog")
            .field("skills", &self.skills.read().len())
            .field("activated", &self.activated.read())
            .finish()
    }
}

impl SkillsCatalog {
    /// New empty catalog with the default always-on set.
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(Vec::new()),
            activated: RwLock::new(HashSet::new()),
            always_on: ALWAYS_ON_TOOLS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// New catalog with a custom always-on set.
    pub fn with_always_on(always_on: impl IntoIterator<Item = String>) -> Self {
        Self {
            skills: RwLock::new(Vec::new()),
            activated: RwLock::new(HashSet::new()),
            always_on: always_on.into_iter().collect(),
        }
    }

    /// Scan the given directories and replace the current skill list.
    /// Returns the number of skills loaded.
    pub fn scan(&self, dirs: &[&Path]) -> usize {
        let mut skills = scan_skills_dirs(dirs);
        // Sort for deterministic output.
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        let n = skills.len();
        *self.skills.write() = skills;
        n
    }

    /// Insert a single skill (test / programmatic API).
    pub fn insert(&self, skill: SkillMeta) {
        self.skills.write().push(skill);
    }

    /// Load a single skill from a file path. Adds it to the catalog.
    pub fn load_file(&self, path: &Path) -> Result<(), SkillError> {
        let mut skill = parse_skill_file(path)?;
        if skill.path.as_os_str().is_empty() {
            skill.path = path.to_path_buf();
        }
        self.skills.write().push(skill);
        Ok(())
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<SkillMeta> {
        self.skills.read().iter().find(|s| s.name == name).cloned()
    }

    /// All skill names.
    pub fn names(&self) -> Vec<String> {
        self.skills.read().iter().map(|s| s.name.clone()).collect()
    }

    /// Mark a skill as activated. Idempotent.
    pub fn activate(&self, name: &str) {
        self.activated.write().insert(name.to_string());
    }

    /// Deactivate a skill.
    pub fn deactivate(&self, name: &str) {
        self.activated.write().remove(name);
    }

    /// True if a skill with this name is currently activated.
    pub fn is_activated(&self, name: &str) -> bool {
        self.activated.read().contains(name)
    }

    /// Return the set of tool names that should be visible to the LLM:
    /// always-on ∪ union of activated skills' `tools:` lists.
    pub fn active_tool_names(&self) -> HashSet<String> {
        let mut out = self.always_on.clone();
        for skill_name in self.activated.read().iter() {
            if let Some(skill) = self.get(skill_name) {
                for t in &skill.tools {
                    out.insert(t.clone());
                }
            }
        }
        out
    }

    /// Render the catalog as a Markdown-ish block suitable for
    /// injection into the system prompt.
    pub fn render_for_system_prompt(&self) -> String {
        render_catalog(&self.skills.read(), &self.activated.read())
    }

    // ── v1.0.0-rc2: plugin 命名空间 ─────────────────────────────────────

    /// 列出当前 catalog 中所有 plugin 提供的 skill 名空间(plugin_id)。
    pub fn plugin_ids(&self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();
        for s in self.skills.read().iter() {
            if let Some(pid) = &s.plugin_id {
                set.insert(pid.clone());
            }
        }
        set.into_iter().collect()
    }

    /// 注册一组 plugin 提供的 skill。`plugin_id` 记到每个 `SkillMeta`
    /// 上,方便 `remove_plugin_skills` 反注册。
    ///
    /// `LoadedSkill` 是 reflect-plugin 的中间形态(只有 name / skill_md /
    /// description);这里转成完整 `SkillMeta`,body 用空串(Phase B 后续
    /// 让 plugin 加载器把 markdown 全文也读进来)。
    ///
    /// 命名:plugin 内 skill name 不加 namespace(与 claude-code 一致,
    /// 同一 plugin 内不同 skills 同名会冲突,作者负责)。
    pub fn add_plugin_skills(&self, plugin_id: &str, loaded: &[crate::model::SkillMeta]) {
        let mut skills = self.skills.write();
        for mut s in loaded.iter().cloned() {
            s.plugin_id = Some(plugin_id.to_string());
            skills.push(s);
        }
        // 重排保持 name 字典序。
        skills.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// 移除某个 plugin 的所有 skill。返回实际移除数量。
    pub fn remove_plugin_skills(&self, plugin_id: &str) -> usize {
        let mut skills = self.skills.write();
        let before = skills.len();
        skills.retain(|s| s.plugin_id.as_deref() != Some(plugin_id));
        before - skills.len()
    }
}

impl Default for SkillsCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a catalog block. Used by [`SkillsCatalog::render_for_system_prompt`]
/// and by the `pre_loop` tests.
pub fn render_catalog(skills: &[SkillMeta], activated: &HashSet<String>) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for s in skills {
        let suffix = if !s.tools.is_empty() {
            format!(" [+tools: {}]", s.tools.join(", "))
        } else {
            String::new()
        };
        let marker = if activated.contains(&s.name) {
            " (active)"
        } else {
            ""
        };
        out.push_str(&format!(
            "- **{}**: {}{}{}\n",
            s.name, s.description, suffix, marker
        ));
    }
    out
}

/// Compute the default `SkillsCatalog` from a list of skill directories.
/// Convenience for the `reflect-exec` bootstrap.
pub fn load_default(skill_dirs: &[PathBuf]) -> Arc<SkillsCatalog> {
    let dir_refs: Vec<&Path> = skill_dirs.iter().map(|p| p.as_path()).collect();
    let cat = SkillsCatalog::new();
    cat.scan(&dir_refs);
    Arc::new(cat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn skill(name: &str, tools: Vec<&str>) -> SkillMeta {
        SkillMeta {
            name: name.into(),
            description: format!("{name} does things"),
            triggers: vec![],
            tools: tools.into_iter().map(String::from).collect(),
            mcp_collections: vec![],
            path: PathBuf::new(),
            body: "body".into(),
            plugin_id: None,
            when_paths: vec![],
        }
    }

    #[test]
    fn render_empty_returns_empty() {
        let out = render_catalog(&[], &HashSet::new());
        assert_eq!(out, "");
    }

    #[test]
    fn render_with_tools_appends_marker() {
        let skills = vec![skill("a", vec!["read", "grep"])];
        let out = render_catalog(&skills, &HashSet::new());
        assert!(out.contains("**a**"));
        assert!(out.contains("[+tools: read, grep]"));
    }

    #[test]
    fn render_without_tools_omits_marker() {
        let skills = vec![skill("a", vec![])];
        let out = render_catalog(&skills, &HashSet::new());
        assert!(!out.contains("[+tools:"));
    }

    #[test]
    fn render_marks_active_skills() {
        let skills = vec![skill("a", vec![])];
        let mut activated = HashSet::new();
        activated.insert("a".into());
        let out = render_catalog(&skills, &activated);
        assert!(out.contains("(active)"));
    }

    #[test]
    fn active_tool_names_includes_always_on_and_activated() {
        let cat = SkillsCatalog::new();
        cat.insert(skill("a", vec!["read", "grep"]));
        cat.insert(skill("b", vec!["write"]));
        // Nothing activated yet.
        let active = cat.active_tool_names();
        assert!(active.contains("bash"));
        assert!(active.contains("read"));
        // Activate a; should add grep.
        cat.activate("a");
        let active = cat.active_tool_names();
        assert!(active.contains("grep"));
        // Activate b; should still have write (already always-on).
        cat.activate("b");
        let active = cat.active_tool_names();
        assert!(active.contains("write"));
    }

    #[test]
    fn activate_is_idempotent() {
        let cat = SkillsCatalog::new();
        cat.insert(skill("a", vec![]));
        cat.activate("a");
        cat.activate("a");
        assert_eq!(cat.activated.read().len(), 1);
    }

    #[test]
    fn get_returns_skill_by_name() {
        let cat = SkillsCatalog::new();
        cat.insert(skill("foo", vec![]));
        assert!(cat.get("foo").is_some());
        assert!(cat.get("bar").is_none());
    }

    #[test]
    fn scan_loads_skill_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "---\nname: x\ndescription: d\n---\nbody\n",
        )
        .unwrap();
        let cat = SkillsCatalog::new();
        let n = cat.scan(&[dir.path()]);
        assert_eq!(n, 1);
        assert!(cat.get("x").is_some());
    }

    #[test]
    fn render_for_system_prompt_uses_internal_state() {
        let cat = SkillsCatalog::new();
        cat.insert(skill("x", vec!["read"]));
        cat.activate("x");
        let out = cat.render_for_system_prompt();
        assert!(out.contains("**x**"));
        assert!(out.contains("(active)"));
    }

    // ── v1.0.0-rc2: plugin 命名空间 ─────────────────────────────────────

    #[test]
    fn add_plugin_skills_records_plugin_id() {
        let cat = SkillsCatalog::new();
        let loaded = vec![skill("lint", vec![]), skill("format", vec![])];
        cat.add_plugin_skills("plugin-a", &loaded);
        assert_eq!(cat.plugin_ids(), vec!["plugin-a".to_string()]);
        let lint = cat.get("lint").unwrap();
        assert_eq!(lint.plugin_id.as_deref(), Some("plugin-a"));
    }

    #[test]
    fn remove_plugin_skills_drops_only_that_plugin() {
        let cat = SkillsCatalog::new();
        cat.add_plugin_skills("plugin-a", &[skill("lint", vec![])]);
        cat.add_plugin_skills("plugin-b", &[skill("format", vec![])]);

        let removed = cat.remove_plugin_skills("plugin-a");
        assert_eq!(removed, 1);
        assert!(cat.get("lint").is_none());
        assert!(cat.get("format").is_some());
        assert_eq!(cat.plugin_ids(), vec!["plugin-b".to_string()]);
    }

    #[test]
    fn remove_unknown_plugin_returns_zero() {
        let cat = SkillsCatalog::new();
        assert_eq!(cat.remove_plugin_skills("ghost"), 0);
    }

    #[test]
    fn builtin_skills_have_no_plugin_id() {
        let cat = SkillsCatalog::new();
        cat.insert(skill("global-skill", vec![]));
        assert_eq!(cat.plugin_ids(), Vec::<String>::new());
    }
}
