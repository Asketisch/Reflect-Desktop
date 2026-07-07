//! `resolver` —— `PermissionResolver` trait + 内存 + store-backed 实现。
//!
//! resolver 是 ApprovalGate 的依赖:gate 在 `ask_tool` 短路 Allow / Deny,
//! 通过 resolver 查"这个 tool 当前有没有显式规则"。

use std::sync::Arc;

use crate::rules::{RuleMatch, evaluate};
use crate::store::PermissionStore;

/// Resolver trait。`async` 简化未来扩展(network / db 后端)。
///
/// 设计取舍:`async fn` 而不是 `fn(&self) -> RuleMatch` —— 即使当前
/// 两个 impl(File + InMemory)都是 sync,async 让 store 端可以走
/// `tokio::fs` 而不阻塞 reactor。代价是 caller 多一层 `.await`,但
/// `ask_tool` 本来就是 async 函数,无影响。
#[async_trait::async_trait]
pub trait PermissionResolver: Send + Sync {
    async fn resolve(&self, tool_name: &str) -> RuleMatch;
}

// ── StorePermissionResolver ─────────────────────────────────────────────

/// 把 `PermissionStore` 包成 resolver。**错误降级**:list 失败
/// (corrupt toml / IO 错误)→ `NoMatch`,让上层按 `PermissionMode`
/// 默认行为走,而不是 deny 用户工作流。
///
/// "降级而非 fail-closed"的理由:用户误编辑 toml 不该让所有 tool 调用
/// 失败。`tracing::warn!` 留排查痕迹。
pub struct StorePermissionResolver {
    store: Arc<dyn PermissionStore>,
}

// 手写 Debug:`Arc<dyn PermissionStore>` 没有 derive Debug。
impl std::fmt::Debug for StorePermissionResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorePermissionResolver")
            .field("store", &"<dyn PermissionStore>")
            .finish()
    }
}

impl StorePermissionResolver {
    pub fn new(store: Arc<dyn PermissionStore>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &Arc<dyn PermissionStore> {
        &self.store
    }
}

#[async_trait::async_trait]
impl PermissionResolver for StorePermissionResolver {
    async fn resolve(&self, tool_name: &str) -> RuleMatch {
        match self.store.list().await {
            Ok(rules) => evaluate(&rules, tool_name),
            Err(e) => {
                tracing::warn!(error = %e, tool = %tool_name, "permission store list failed; falling back to NoMatch");
                RuleMatch::NoMatch
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{PermissionAction, PermissionRule};
    use crate::store::InMemoryPermissionStore;

    #[tokio::test]
    async fn resolver_returns_allow_when_rule_present() {
        let s = Arc::new(InMemoryPermissionStore::new());
        s.add(PermissionRule {
            tool: "Bash".into(),
            action: PermissionAction::Allow,
            tool_glob: None,
            shell_pattern: None,
        })
        .await
        .unwrap();
        let r = StorePermissionResolver::new(s);
        assert_eq!(r.resolve("Bash").await, RuleMatch::Allow);
    }

    #[tokio::test]
    async fn resolver_returns_deny_when_rule_present() {
        let s = Arc::new(InMemoryPermissionStore::new());
        s.add(PermissionRule {
            tool: "Write".into(),
            action: PermissionAction::Deny,
            tool_glob: None,
            shell_pattern: None,
        })
        .await
        .unwrap();
        let r = StorePermissionResolver::new(s);
        assert_eq!(r.resolve("Write").await, RuleMatch::Deny);
    }

    #[tokio::test]
    async fn resolver_returns_no_match_for_unknown_tool() {
        let s = Arc::new(InMemoryPermissionStore::new());
        let r = StorePermissionResolver::new(s);
        assert_eq!(r.resolve("Bash").await, RuleMatch::NoMatch);
    }
}
