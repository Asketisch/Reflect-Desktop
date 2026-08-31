//! Memory 存储 —— 将 memory entry 持久化到 `MEMORY.md` 文件。
//!
//! 支持两个 scope:
//! - `project` —— 存储在当前工作区根目录 `.reflect/agent-memory/reflect/MEMORY.md`
//!   （由调用方传入活动工作区,与 shell/git/files 同源 —— 打包成 .app 后
//!   进程 cwd 不可写,也不再跟随不了 `reflect_set_workspace`）;
//! - `user` —— 存储在用户主目录 `~/.reflect/agent-memory/reflect/MEMORY.md`。
//!
//! 每条 entry 以 `## <key>` 标题块形式追加到文件中。

use std::path::{Path, PathBuf};

use crate::commands::MemoryEntry;

#[derive(Default)]
pub(crate) struct MemoryStore;

impl MemoryStore {
    /// 列出所有 scope 下的 memory 条目。
    pub(crate) fn list(&self, project_root: &Path) -> anyhow::Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();
        for (scope, path) in self.scopes(project_root)? {
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
    pub(crate) fn add(
        &self,
        scope: &str,
        key: &str,
        value: &str,
        project_root: &Path,
    ) -> anyhow::Result<()> {
        let path = self.path(scope, project_root)?;
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

    pub(crate) fn remove(&self, scope: &str, key: &str, project_root: &Path) -> anyhow::Result<()> {
        let path = self.path(scope, project_root)?;
        if !path.exists() {
            return Ok(());
        }
        let current = std::fs::read_to_string(&path)?;
        // 整行精确匹配 `## <key>`:子串前缀匹配会让 key "a" 命中
        // "## about" 的块起点,误删别的条目。
        let heading = format!("## {key}");
        if let Some((start, line_end)) = find_exact_heading_line(&current, &heading) {
            let end = current[line_end..]
                .find("\n## ")
                .map(|index| line_end + index)
                .unwrap_or(current.len());
            let mut updated = String::with_capacity(current.len());
            updated.push_str(&current[..start]);
            updated.push_str(&current[end..]);
            std::fs::write(path, updated)?;
        }
        Ok(())
    }

    fn scopes(&self, project_root: &Path) -> anyhow::Result<[(&'static str, PathBuf); 2]> {
        Ok([
            ("project", self.path("project", project_root)?),
            ("user", self.path("user", project_root)?),
        ])
    }

    fn path(&self, scope: &str, project_root: &Path) -> anyhow::Result<PathBuf> {
        let root = if scope == "user" {
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?
        } else {
            project_root.to_path_buf()
        };
        Ok(root.join(".reflect/agent-memory/reflect/MEMORY.md"))
    }
}

/// 在文本中查找整行恰为 `heading` 的行,返回 (行起始字节, 行尾字节,不含换行)。
fn find_exact_heading_line(text: &str, heading: &str) -> Option<(usize, usize)> {
    let mut from = 0;
    while let Some(rel) = text[from..].find(heading) {
        let start = from + rel;
        let at_line_start = start == 0 || text.as_bytes()[start - 1] == b'\n';
        let line_end = text[start..]
            .find('\n')
            .map(|i| start + i)
            .unwrap_or(text.len());
        if at_line_start && text[start..line_end].trim_end() == heading {
            return Some((start, line_end));
        }
        from = start + heading.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::find_exact_heading_line;

    #[test]
    fn exact_line_match_rejects_prefix_and_extension() {
        let text = "## about\nabout body\n## a\na body\n";
        // key "a" 不能命中 "## about" 的前缀。
        assert_eq!(find_exact_heading_line(text, "## a"), Some((20, 24)));
        assert_eq!(find_exact_heading_line(text, "## about"), Some((0, 8)));
        assert_eq!(find_exact_heading_line(text, "## missing"), None);
    }
}
