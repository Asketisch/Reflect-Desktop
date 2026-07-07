//! `bundled` —— 内置技能包(P2 `bundled-skills`)。
//!
//! 编译期嵌入常用技能元数据;启动时可 merge 进 `SkillsCatalog`。

use std::path::PathBuf;

use crate::model::SkillMeta;

/// 内置技能 catalog(stub:3 个代表性技能,v2 扩至 15+)。
pub fn bundled_skills() -> Vec<SkillMeta> {
    vec![
        SkillMeta {
            name: "code-review".into(),
            description: "Review code changes for bugs and style".into(),
            triggers: vec!["review".into(), "pr".into()],
            tools: vec!["read".into(), "grep".into()],
            mcp_collections: vec![],
            path: PathBuf::from("bundled/code-review/SKILL.md"),
            body: include_str!("bundled/code-review.md").to_string(),
            plugin_id: None,
            when_paths: vec!["**/*.rs".into(), "**/*.ts".into()],
        },
        SkillMeta {
            name: "commit-helper".into(),
            description: "Draft conventional commit messages".into(),
            triggers: vec!["commit".into()],
            tools: vec!["bash".into(), "read".into()],
            mcp_collections: vec![],
            path: PathBuf::from("bundled/commit-helper/SKILL.md"),
            body: include_str!("bundled/commit-helper.md").to_string(),
            plugin_id: None,
            when_paths: vec![],
        },
        SkillMeta {
            name: "test-runner".into(),
            description: "Run and interpret test output".into(),
            triggers: vec!["test".into(), "cargo test".into()],
            tools: vec!["bash".into()],
            mcp_collections: vec![],
            path: PathBuf::from("bundled/test-runner/SKILL.md"),
            body: include_str!("bundled/test-runner.md").to_string(),
            plugin_id: None,
            when_paths: vec!["**/Cargo.toml".into()],
        },
    ]
}

/// 把 bundled skills 插入 catalog(跳过已存在的同名技能)。
pub fn merge_bundled(catalog: &crate::catalog::SkillsCatalog) -> usize {
    let mut added = 0;
    for s in bundled_skills() {
        if catalog.get(&s.name).is_none() {
            catalog.insert(s);
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_has_at_least_three() {
        assert!(bundled_skills().len() >= 3);
    }

    #[test]
    fn merge_bundled_is_idempotent() {
        let cat = crate::catalog::SkillsCatalog::new();
        let n1 = merge_bundled(&cat);
        let n2 = merge_bundled(&cat);
        assert!(n1 >= 3);
        assert_eq!(n2, 0);
    }
}
