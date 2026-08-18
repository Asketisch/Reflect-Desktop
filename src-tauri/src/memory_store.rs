//! Memory 存储 —— 将 memory entry 持久化到 `MEMORY.md` 文件。
//!
//! 支持两个 scope:
//! - `project` —— 存储在当前工作区根目录 `.reflect/agent-memory/reflect/MEMORY.md`;
//! - `user` —— 存储在用户主目录 `~/.reflect/agent-memory/reflect/MEMORY.md`。
//!
//! 每条 entry 以 `## <key>` 标题块形式追加到文件中。

use std::path::PathBuf;

use crate::commands::MemoryEntry;

#[derive(Default)]
pub(crate) struct MemoryStore;

impl MemoryStore {
    /// 列出所有 scope 下的 memory 条目。
    pub(crate) fn list(&self) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();
        for (scope, path) in self.scopes()? {
            if let Ok(content) = std::fs::read_to_string(path) {
                if !content.trim().is_empty() {
                    entries.push(MemoryEntry {
                        scope: scope.to_string(),
                        key: "(all)".to_string(),
                        value: content,
                    });
                }
            }
        }
        Ok(entries)
    }

    /// 新增 memory entry。
    pub(crate) fn add(&self, scope: &str, key: &str, value: &str) -> anyhow::Result<()> {
        let path = self.path(scope)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut current = std::fs::read_to_string(&path).unwrap_or_default();
        if !current.is_empty() && !current.ends_with('\n') {
            current.push('\n');
        }
        current.push_str(&format!("\n## {key}\n{value}\n"));
        std::fs::write(path, current)?;
        Ok(())
    }

    pub(crate) fn remove(&self, scope: &str, key: &str) -> anyhow::Result<()> {
        let path = self.path(scope)?;
        if !path.exists() {
            return Ok(());
        }
        let current = std::fs::read_to_string(&path)?;
        let needle = format!("## {key}");
        if let Some(start) = current.find(&needle) {
            let after = start + needle.len();
            let end = current[after..]
                .find("\n## ")
                .map(|index| after + index)
                .unwrap_or(current.len());
            let mut updated = String::with_capacity(current.len());
            updated.push_str(&current[..start]);
            updated.push_str(&current[end..]);
            std::fs::write(path, updated)?;
        }
        Ok(())
    }

    fn scopes(&self) -> anyhow::Result<[(&'static str, PathBuf); 2]> {
        Ok([
            ("project", self.path("project")?),
            ("user", self.path("user")?),
        ])
    }

    fn path(&self, scope: &str) -> anyhow::Result<PathBuf> {
        let root = if scope == "user" {
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?
        } else {
            std::env::current_dir().map_err(|error| anyhow::anyhow!("cwd: {error}"))?
        };
        Ok(root.join(".reflect/agent-memory/reflect/MEMORY.md"))
    }
}
