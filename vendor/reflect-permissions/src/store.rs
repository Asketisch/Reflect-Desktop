//! `store` —— PermissionRule 持久化(TOML 文件 + 内存实现)。
//!
//! 格式: `[[rule]]` 数组,每条 `{ tool, action }`。
//!
//! ```toml
//! # ~/.reflect/permissions.toml
//! [[rule]]
//! tool = "Bash"
//! action = "allow"
//!
//! [[rule]]
//! tool = "Write"
//! action = "deny"
//! ```
//!
//! v1.x 简化:无 lock / 并发写。如果并发 TUI + CLI 同时 add,最后写者
//! 覆盖(同 `reflect-rollout::path` 风格)。`add` 内部 read-modify-write
//! 保证已有 rules 不丢。

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rules::PermissionRule;

#[cfg(test)]
use crate::rules::PermissionAction;

/// 持久化层错误。区分 IO / parse / env(无 HOME),让 caller 决定怎么
/// 反馈给用户。
#[derive(Debug, Error)]
pub enum PermissionsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse toml: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize toml: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("HOME env var not set")]
    NoHome,
}

/// File format wrapper —— `[[rule]]` 数组的根对象。
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PermissionFile {
    #[serde(default, rename = "rule")]
    rules: Vec<PermissionRule>,
}

/// Permission store trait。`async` 简化未来扩展(网络同步、数据库),v1.x
/// 两个 impl 都是 sync 但 wrap 成 async。
#[async_trait::async_trait]
pub trait PermissionStore: Send + Sync {
    /// 列出全部规则。**不**排序 —— caller 决定展示顺序。
    async fn list(&self) -> Result<Vec<PermissionRule>, PermissionsError>;
    /// 添加一条规则(append 到末尾)。已存在的同名 tool **不**替换 —
    /// 保留"first match wins"语义,`evaluate` 文档要求。
    async fn add(&self, rule: PermissionRule) -> Result<(), PermissionsError>;
    /// 按 tool 名删一条。找不到 → `Ok(())`(幂等,add/remove 配对更友好)。
    async fn remove(&self, tool: &str) -> Result<(), PermissionsError>;
}

// ── InMemoryPermissionStore ──────────────────────────────────────────────

/// 内存实现,测试用,无 IO。
#[derive(Debug, Default)]
pub struct InMemoryPermissionStore {
    rules: Mutex<Vec<PermissionRule>>,
}

impl InMemoryPermissionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl PermissionStore for InMemoryPermissionStore {
    async fn list(&self) -> Result<Vec<PermissionRule>, PermissionsError> {
        Ok(self.rules.lock().clone())
    }

    async fn add(&self, rule: PermissionRule) -> Result<(), PermissionsError> {
        self.rules.lock().push(rule);
        Ok(())
    }

    async fn remove(&self, tool: &str) -> Result<(), PermissionsError> {
        let mut g = self.rules.lock();
        g.retain(|r| r.tool != tool);
        Ok(())
    }
}

// ── FilePermissionStore ─────────────────────────────────────────────────

/// TOML 文件持久化,默认路径 `~/.reflect/permissions.toml`。
#[derive(Debug)]
pub struct FilePermissionStore {
    path: PathBuf,
}

impl FilePermissionStore {
    /// 用 `~/.reflect/permissions.toml`(HOME 不存在 → `Err(NoHome)`)。
    pub fn with_default_home() -> Result<Self, PermissionsError> {
        let home = std::env::var_os("HOME").ok_or(PermissionsError::NoHome)?;
        Ok(Self::with_path(
            PathBuf::from(home).join(".reflect/permissions.toml"),
        ))
    }

    /// 测试用 —— 直接给路径,跳过 HOME env 依赖。
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// 暴露路径,给 Pill 输出提示用(让用户知道 rules 存在哪)。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 同步读 + 解析。文件不存在 → `Ok(default())`(让 `list` 起点是空,
    /// 不是 `Err`)—— 与 `reflect-rollout` 的"缺失 base 返回空"风格一致。
    fn read(&self) -> Result<PermissionFile, PermissionsError> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => {
                if s.trim().is_empty() {
                    Ok(PermissionFile::default())
                } else {
                    Ok(toml::from_str(&s)?)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PermissionFile::default()),
            Err(e) => Err(PermissionsError::Io(e)),
        }
    }

    /// 同步写:保证 parent dir 存在(避免 PermissionDenied on first write),
    /// 然后 toml::to_string_pretty + write。
    fn write(&self, file: &PermissionFile) -> Result<(), PermissionsError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(file)?;
        std::fs::write(&self.path, body)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PermissionStore for FilePermissionStore {
    async fn list(&self) -> Result<Vec<PermissionRule>, PermissionsError> {
        Ok(self.read()?.rules)
    }

    async fn add(&self, rule: PermissionRule) -> Result<(), PermissionsError> {
        let mut file = self.read()?;
        // "first match wins" + 用户 add 顺序隐式定 precedence → 不去重
        // 同名(tool 一致 + action 不同也算合法,后写的在尾部不影响 evaluate)。
        // 但完全相同(tool + action)就跳过,避免无意义重复。
        if !file.rules.iter().any(|r| r == &rule) {
            file.rules.push(rule);
            self.write(&file)?;
        }
        Ok(())
    }

    async fn remove(&self, tool: &str) -> Result<(), PermissionsError> {
        let mut file = self.read()?;
        let before = file.rules.len();
        file.rules.retain(|r| r.tool != tool);
        if file.rules.len() != before {
            self.write(&file)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn rule(tool: &str, action: PermissionAction) -> PermissionRule {
        PermissionRule {
            tool: tool.into(),
            action,
            tool_glob: None,
            shell_pattern: None,
        }
    }

    #[tokio::test]
    async fn in_memory_add_list_remove_round_trip() {
        let s = InMemoryPermissionStore::new();
        assert!(s.list().await.unwrap().is_empty());
        s.add(rule("Bash", PermissionAction::Allow)).await.unwrap();
        s.add(rule("Write", PermissionAction::Deny)).await.unwrap();
        let rules = s.list().await.unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0], rule("Bash", PermissionAction::Allow));
        assert_eq!(rules[1], rule("Write", PermissionAction::Deny));
        s.remove("Bash").await.unwrap();
        let after = s.list().await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].tool, "Write");
        // 不存在的 tool → 幂等,Ok(())。
        s.remove("Nope").await.unwrap();
    }

    #[tokio::test]
    async fn file_store_missing_file_returns_empty() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("perms.toml");
        let s = FilePermissionStore::with_path(p);
        assert!(s.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn file_store_add_persists_round_trip() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("perms.toml");
        let s = FilePermissionStore::with_path(p.clone());
        s.add(rule("Bash", PermissionAction::Allow)).await.unwrap();
        s.add(rule("Read", PermissionAction::Ask)).await.unwrap();
        // 新建同路径 store,验证磁盘持久化。
        let s2 = FilePermissionStore::with_path(p);
        let rules = s2.list().await.unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0], rule("Bash", PermissionAction::Allow));
        assert_eq!(rules[1], rule("Read", PermissionAction::Ask));
    }

    #[tokio::test]
    async fn file_store_creates_parent_dir() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("nested").join("perms.toml");
        let s = FilePermissionStore::with_path(p);
        s.add(rule("Bash", PermissionAction::Allow)).await.unwrap();
        // 父目录被自动创建,文件存在。
        assert!(s.path().exists());
    }

    #[tokio::test]
    async fn file_store_add_duplicate_skips() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("perms.toml");
        let s = FilePermissionStore::with_path(p);
        s.add(rule("Bash", PermissionAction::Allow)).await.unwrap();
        s.add(rule("Bash", PermissionAction::Allow)).await.unwrap();
        // 完全重复(tool + action)不写,避免无意义重复。
        assert_eq!(s.list().await.unwrap().len(), 1);
        // 但不同 action 算合法 —— first match wins,后写的在尾不影响 evaluate。
        s.add(rule("Bash", PermissionAction::Deny)).await.unwrap();
        assert_eq!(s.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn file_store_remove_only_affects_matching_tool() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("perms.toml");
        let s = FilePermissionStore::with_path(p);
        s.add(rule("Bash", PermissionAction::Allow)).await.unwrap();
        s.add(rule("Write", PermissionAction::Deny)).await.unwrap();
        s.remove("Bash").await.unwrap();
        let after = s.list().await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].tool, "Write");
    }

    #[tokio::test]
    async fn file_store_handles_corrupt_toml() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("perms.toml");
        std::fs::write(&p, "not = valid toml :::").unwrap();
        let s = FilePermissionStore::with_path(p);
        // 解析失败 → Err(Parse),让上层决定 fallback(当前 resolver 把
        // 错误降级为 NoMatch)。
        let r = s.list().await;
        assert!(r.is_err());
    }
}
