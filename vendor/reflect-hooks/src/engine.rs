//! `HookEngine` — owns the registered hooks, dispatches events, merges decisions.
//!
//! See `docs/tools-and-hooks.md §4.4` for the merge priority.
//!
//! Implementation note: hooks are stored as `Arc<dyn Hook>`. Dispatch
//! snapshots the relevant hooks under a read lock, drops the guard, and
//! awaits each hook's `handle` outside the lock to avoid holding a
//! `Send`-incompatible guard across an `.await`.

use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

use crate::abort::HookAbortSignal;
use crate::decision::{HookDecision, SystemMessage};
use crate::event::HookEvent;
use crate::hook::Hook;

/// Thread-safe registry + dispatcher for `Hook`s。
pub struct HookEngine {
    hooks: RwLock<Vec<Arc<dyn Hook>>>,
    /// 被 `disable(name)` 标记但未 `unregister` 的 hook —— 仍在 vec
    /// 中,但 `dispatch` 时跳过。`PluginManager::disable` 走此路径而
    /// 不 `unregister`,保证可以原地 `enable` 复活。
    disabled: RwLock<HashSet<String>>,
}

impl Default for HookEngine {
    fn default() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
            disabled: RwLock::new(HashSet::new()),
        }
    }
}

impl HookEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook. Order matters for `ModifyArgs` (last write wins)
    /// and `InjectMessage` (concatenation in registration order).
    pub fn register<H: Hook + 'static>(&self, hook: H) {
        let name = hook.name().to_string();
        self.hooks.write().push(Arc::new(hook));
        // 新注册 hook 默认 enabled;如果它之前在 disabled set 中,
        // (例如 reload 后重新注册)保持之前的 disabled 状态 —— 调用方
        // 需要显式 `enable(name)` 复活。这是 hook hot reload 时的
        // 关键不变量:清掉 + 重建后,disabled 集合保留,避免热重载期间
        // 被禁用的 hook 短暂地"复活"。
        if self.disabled.read().contains(&name) {
            tracing::debug!(name = %name, "hook re-registered while disabled; remains disabled");
        }
    }

    /// 反注册指定名字的 hook。返回 `true` 表示真删了,`false` 表示
    /// 没找到。重复调用幂等。
    ///
    /// v1.0.0-rc2 起为 plugin 卸载提供 API。**注意:`disabled` set
    /// 中的条目不会被清理** —— 这是热重载关键不变量:`unregister + register`
    /// 之后,hook 的 disabled 状态保持不变(若用户曾 `disable("foo")`,
    /// reload 后 `foo` 仍 disabled)。悬挂条目无害,等 hook 真正永久消失
    /// 时(进程退出)随 `HookEngine` 一起 GC。
    pub fn unregister(&self, name: &str) -> bool {
        let mut hooks = self.hooks.write();
        let before = hooks.len();
        hooks.retain(|h| h.name() != name);
        before != hooks.len()
    }

    /// 标记指定名字的 hook 为 disabled —— dispatch 时跳过,但 hook
    /// 仍在 vec 中。返回 `true` 表示之前是 enabled,`false` 表示已是
    /// disabled 或不存在。
    pub fn disable(&self, name: &str) -> bool {
        // 先确认 hook 真的存在;否则视为 no-op。
        if !self.hooks.read().iter().any(|h| h.name() == name) {
            return false;
        }
        self.disabled.write().insert(name.to_string())
    }

    /// 把 disabled 的 hook 复活。返回 `true` 表示之前是 disabled,`false`
    /// 表示已是 enabled 或不存在。
    pub fn enable(&self, name: &str) -> bool {
        if !self.hooks.read().iter().any(|h| h.name() == name) {
            return false;
        }
        self.disabled.write().remove(name)
    }

    /// hook 当前是否被禁用。
    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.read().contains(name)
    }

    /// 列出所有已注册 hook 的名字 —— 用于 UI / 测试。
    /// 顺序按注册序,与 `register` 调用次序一致。
    pub fn hook_names(&self) -> Vec<String> {
        self.hooks
            .read()
            .iter()
            .map(|h| h.name().to_string())
            .collect()
    }

    /// 列出 `(name, description)` 元组 —— 供 `/hooks ls` TUI pill 渲染。
    /// 顺序按注册序。`description` 来自 `Hook::description()`(默认空串)。
    /// disabled 状态保留(显示时由调用方按需加 `[disabled]` 标记)。
    pub fn hook_summaries(&self) -> Vec<(String, String)> {
        self.hooks
            .read()
            .iter()
            .map(|h| (h.name().to_string(), h.description().to_string()))
            .collect()
    }

    /// Number of registered hooks (test-only).
    #[cfg(test)]
    #[allow(clippy::len_without_is_empty)] // pre-M5: test-only helper
    pub fn len(&self) -> usize {
        self.hooks.read().len()
    }

    /// Dispatch `event` to every relevant hook and merge the results.
    /// Does not consult the abort signal — use
    /// [`dispatch_with_abort`](Self::dispatch_with_abort) for cancellable
    /// loops.
    pub async fn dispatch(&self, event: &HookEvent) -> HookDecision {
        let kind = event.kind();
        // Snapshot under the lock, then drop the guard before awaiting.
        // disabled set 也在同一 read guard 内读,避免 TOCTOU。
        let snapshot: Vec<Arc<dyn Hook>> = {
            let hooks = self.hooks.read();
            let disabled = self.disabled.read();
            hooks
                .iter()
                .filter(|h| h.events().contains(&kind) && !disabled.contains(h.name()))
                .cloned()
                .collect()
        };
        let mut decisions: Vec<HookDecision> = Vec::new();
        for hook in snapshot {
            match hook.handle(event).await {
                Ok(d) => decisions.push(d),
                Err(e) => {
                    tracing::error!(hook = hook.name(), error = %e, "hook failed (fail-closed)");
                    decisions.push(HookDecision::Deny {
                        reason: format!("hook '{}' failed: {e}", hook.name()),
                    });
                }
            }
        }
        Self::merge(decisions)
    }

    /// Like [`dispatch`](Self::dispatch) but returns a `Deny { "cancelled" }`
    /// if the abort signal was triggered.
    pub async fn dispatch_with_abort(
        &self,
        event: &HookEvent,
        abort: &HookAbortSignal,
    ) -> HookDecision {
        if abort.is_triggered() {
            return HookDecision::Deny {
                reason: "cancelled by user".into(),
            };
        }
        self.dispatch(event).await
    }

    /// Merge a list of hook decisions into one. Priority
    /// `Deny > Ask > ModifyArgs > InjectMessage > Allow`. Failures (errors)
    /// are turned into `Deny` at the call site, not here.
    pub fn merge(decisions: Vec<HookDecision>) -> HookDecision {
        if decisions.is_empty() {
            return HookDecision::Allow;
        }
        // Flatten one level of Combined so a single hook can return multiple
        // decisions.
        let flat: Vec<HookDecision> = decisions
            .into_iter()
            .flat_map(|d| match d {
                HookDecision::Combined(inner) => inner,
                other => vec![other],
            })
            .collect();

        let mut deny_reason: Option<String> = None;
        let mut ask_reason: Option<String> = None;
        let mut modified: Option<serde_json::Value> = None;
        let mut injected: Vec<SystemMessage> = Vec::new();

        for d in flat {
            match d {
                HookDecision::Deny { reason } => {
                    if deny_reason.is_none() {
                        deny_reason = Some(reason);
                    }
                }
                HookDecision::Ask { reason } => {
                    if ask_reason.is_none() {
                        ask_reason = Some(reason);
                    }
                }
                HookDecision::ModifyArgs(v) => {
                    modified = Some(v);
                }
                HookDecision::InjectMessage(m) => {
                    injected.push(m);
                }
                // PermissionOverride is collected separately by callers
                // that need it; for M3 it does not influence merge.
                HookDecision::PermissionOverride(_) => {}
                HookDecision::Allow | HookDecision::Combined(_) => {}
            }
        }

        // Priority 1: Deny wins outright.
        if let Some(reason) = deny_reason {
            let mut parts: Vec<HookDecision> = vec![HookDecision::Deny { reason }];
            if let Some(v) = modified {
                parts.push(HookDecision::ModifyArgs(v));
            }
            for m in injected {
                parts.push(HookDecision::InjectMessage(m));
            }
            return if parts.len() == 1 {
                parts.remove(0)
            } else {
                HookDecision::Combined(parts)
            };
        }

        // Priority 2: Ask wins over side-effects but loses to Deny.
        if let Some(reason) = ask_reason {
            let mut parts: Vec<HookDecision> = vec![HookDecision::Ask { reason }];
            if let Some(v) = modified {
                parts.push(HookDecision::ModifyArgs(v));
            }
            for m in injected {
                parts.push(HookDecision::InjectMessage(m));
            }
            return if parts.len() == 1 {
                parts.remove(0)
            } else {
                HookDecision::Combined(parts)
            };
        }

        // Priority 3: combine side-effects in deterministic order.
        let mut parts: Vec<HookDecision> = Vec::new();
        if let Some(v) = modified {
            parts.push(HookDecision::ModifyArgs(v));
        }
        for m in injected {
            parts.push(HookDecision::InjectMessage(m));
        }
        if parts.is_empty() {
            HookDecision::Allow
        } else if parts.len() == 1 {
            parts.remove(0)
        } else {
            HookDecision::Combined(parts)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{HookContext, HookEventKind, StopReason};
    use crate::hook::{Hook, HookError};
    use async_trait::async_trait;
    use reflect_protocol::{PermissionMode, ThreadId, TurnId};
    use std::path::PathBuf;

    struct AllowHook;
    #[async_trait]
    impl Hook for AllowHook {
        fn name(&self) -> &str {
            "allow"
        }
        fn events(&self) -> &[HookEventKind] {
            &[HookEventKind::PreToolUse]
        }
    }

    struct DenyHook(&'static str);
    #[async_trait]
    impl Hook for DenyHook {
        fn name(&self) -> &str {
            "deny"
        }
        fn events(&self) -> &[HookEventKind] {
            &[HookEventKind::PreToolUse]
        }
        async fn handle(&self, _: &HookEvent) -> Result<HookDecision, HookError> {
            Ok(HookDecision::Deny {
                reason: self.0.into(),
            })
        }
    }

    fn make_session_start() -> HookEvent {
        HookEvent::SessionStart {
            session_id: ThreadId::new(),
            config: serde_json::json!({}),
        }
    }

    fn make_pre_tool_use() -> HookEvent {
        HookEvent::PreToolUse {
            tool: "bash".into(),
            args: serde_json::json!({}),
            ctx: HookContext {
                session_id: ThreadId::new(),
                turn_id: TurnId::new(),
                workspace: PathBuf::from("/"),
                permission_mode: PermissionMode::Auto,
            },
        }
    }

    #[test]
    fn merge_empty_is_allow() {
        assert_eq!(HookEngine::merge(vec![]), HookDecision::Allow);
    }

    #[test]
    fn merge_single_allow() {
        assert_eq!(
            HookEngine::merge(vec![HookDecision::Allow]),
            HookDecision::Allow
        );
    }

    #[test]
    fn merge_deny_wins() {
        let merged = HookEngine::merge(vec![
            HookDecision::Allow,
            HookDecision::Deny { reason: "x".into() },
        ]);
        match merged {
            HookDecision::Deny { reason } => assert_eq!(reason, "x"),
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn merge_modify_args_takes_last() {
        let merged = HookEngine::merge(vec![
            HookDecision::ModifyArgs(serde_json::json!({"a": 1})),
            HookDecision::ModifyArgs(serde_json::json!({"b": 2})),
        ]);
        match merged {
            HookDecision::ModifyArgs(v) => assert_eq!(v, serde_json::json!({"b": 2})),
            other => panic!("expected modify, got {other:?}"),
        }
    }

    #[test]
    fn merge_inject_message_combines() {
        let merged = HookEngine::merge(vec![
            HookDecision::InjectMessage(SystemMessage::new("a")),
            HookDecision::InjectMessage(SystemMessage::new("b")),
        ]);
        match merged {
            HookDecision::Combined(parts) => assert_eq!(parts.len(), 2),
            other => panic!("expected combined, got {other:?}"),
        }
    }

    #[test]
    fn merge_deny_plus_modify_keeps_both() {
        let merged = HookEngine::merge(vec![
            HookDecision::ModifyArgs(serde_json::json!({"x": 1})),
            HookDecision::Deny {
                reason: "no".into(),
            },
        ]);
        match merged {
            HookDecision::Combined(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(parts[0], HookDecision::Deny { .. }));
                assert!(matches!(parts[1], HookDecision::ModifyArgs(_)));
            }
            other => panic!("expected combined, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_with_no_hooks_allows() {
        let engine = HookEngine::new();
        assert_eq!(
            engine.dispatch(&make_session_start()).await,
            HookDecision::Allow
        );
    }

    #[tokio::test]
    async fn dispatch_with_allow_hook_allows() {
        let engine = HookEngine::new();
        engine.register(AllowHook);
        assert_eq!(
            engine.dispatch(&make_pre_tool_use()).await,
            HookDecision::Allow
        );
    }

    #[tokio::test]
    async fn dispatch_with_deny_hook_denies() {
        let engine = HookEngine::new();
        engine.register(DenyHook("blocked"));
        // DenyHook is registered for PreToolUse, so dispatching a PreToolUse
        // event should produce Deny.
        let d = engine.dispatch(&make_pre_tool_use()).await;
        match d {
            HookDecision::Deny { reason } => assert_eq!(reason, "blocked"),
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_filters_unrelated_hooks() {
        let engine = HookEngine::new();
        engine.register(DenyHook("blocked")); // PreToolUse only
        // Dispatch a Stop event — DenyHook should be filtered out.
        let d = engine
            .dispatch(&HookEvent::Stop {
                reason: StopReason::AgentDecision,
                attempt: 0,
            })
            .await;
        assert_eq!(d, HookDecision::Allow);
    }

    #[tokio::test]
    async fn abort_signal_dispatch_returns_cancel_deny() {
        let engine = HookEngine::new();
        engine.register(AllowHook);
        let abort = HookAbortSignal::new();
        abort.trigger();
        let d = engine
            .dispatch_with_abort(&make_session_start(), &abort)
            .await;
        match d {
            HookDecision::Deny { reason } => assert!(reason.contains("cancel")),
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hook_error_fails_closed() {
        struct BadHook;
        #[async_trait]
        impl Hook for BadHook {
            fn name(&self) -> &str {
                "bad"
            }
            fn events(&self) -> &[HookEventKind] {
                &[HookEventKind::SessionStart]
            }
            async fn handle(&self, _: &HookEvent) -> Result<HookDecision, HookError> {
                Err(HookError::Other("boom".into()))
            }
        }
        let engine = HookEngine::new();
        engine.register(BadHook);
        let d = engine.dispatch(&make_session_start()).await;
        match d {
            HookDecision::Deny { reason } => assert!(reason.contains("boom")),
            other => panic!("expected fail-closed deny, got {other:?}"),
        }
    }

    // ── v1.0.0-rc2 lifecycle API 测试 ────────────────────────────────

    /// `unregister(name)` 移除 hook;不存在返回 false;幂等。
    #[test]
    fn unregister_removes_named_hook() {
        let engine = HookEngine::new();
        engine.register(AllowHook); // name = "allow"
        assert!(engine.unregister("allow"));
        assert!(!engine.unregister("allow"));
        assert!(!engine.unregister("never_existed"));
        assert_eq!(engine.len(), 0);
    }

    /// `disable` 跳过 dispatch,但 hook 仍在 vec 中;`enable` 复活。
    #[tokio::test]
    async fn disable_then_enable_round_trip() {
        let engine = HookEngine::new();
        engine.register(DenyHook("blocked")); // name = "deny"
        assert!(engine.disable("deny"));
        // disabled 时 dispatch 不命中 deny hook。
        assert_eq!(
            engine.dispatch(&make_pre_tool_use()).await,
            HookDecision::Allow
        );
        // 二次 disable 返回 false。
        assert!(!engine.disable("deny"));
        // enable 后复活。
        assert!(engine.enable("deny"));
        let d = engine.dispatch(&make_pre_tool_use()).await;
        assert!(matches!(d, HookDecision::Deny { .. }));
    }

    /// `disable` 不存在的 hook 返回 false(no-op,不报错)。
    #[test]
    fn disable_missing_is_noop() {
        let engine = HookEngine::new();
        assert!(!engine.disable("ghost"));
    }

    /// 重新 register 已 disabled 的 hook 仍然 disabled —— 热重载关键不变量。
    #[tokio::test]
    async fn reregister_preserves_disabled_state() {
        let engine = HookEngine::new();
        engine.register(DenyHook("v1"));
        engine.disable("deny");
        // unregister 后重新注册(模拟 reload 流程)。
        assert!(engine.unregister("deny"));
        engine.register(DenyHook("v2"));
        // 重新 register 后仍是 disabled —— 热重载期间不会被短暂复活。
        assert!(engine.is_disabled("deny"));
        assert_eq!(
            engine.dispatch(&make_pre_tool_use()).await,
            HookDecision::Allow
        );
    }

    /// `unregister` **不**清 disabled set —— 重新 register 同名 hook
    /// 后仍是 disabled(热重载关键不变量)。但如果不再 register,
    /// disabled set 里的悬挂条目无害,等 `HookEngine` 析构时一起 GC。
    #[test]
    fn unregister_preserves_disabled_for_later_reregister() {
        let engine = HookEngine::new();
        engine.register(AllowHook);
        engine.disable("allow");
        assert!(engine.is_disabled("allow"));
        engine.unregister("allow");
        // 悬挂条目无害保留 —— 这样如果用户 reload 同名 hook,会直接
        // 命中 disabled 状态而不被短暂复活。
        assert!(engine.is_disabled("allow"));
    }

    /// `hook_names()` 按注册序返回。
    #[test]
    fn hook_names_in_registration_order() {
        let engine = HookEngine::new();
        engine.register(AllowHook);
        engine.register(DenyHook("x"));
        assert_eq!(engine.hook_names(), vec!["allow", "deny"]);
    }

    /// `hook_summaries()` 返回 `(name, description)` 元组,按注册序。
    #[test]
    fn hook_summaries_registration_order() {
        let engine = HookEngine::new();
        engine.register(AllowHook); // description 默认 ""
        engine.register(DenyHook("blocked"));
        let sums = engine.hook_summaries();
        assert_eq!(sums.len(), 2);
        assert_eq!(sums[0].0, "allow");
        assert_eq!(sums[0].1, "");
        assert_eq!(sums[1].0, "deny");
        assert_eq!(sums[1].1, "");
    }

    /// `description()` 可由 hook 实现覆盖 —— `DenyHook` 测试替身。
    #[test]
    fn hook_summaries_returns_custom_description() {
        struct DescribedHook;
        #[async_trait]
        impl Hook for DescribedHook {
            fn name(&self) -> &str {
                "audit-logger"
            }
            fn description(&self) -> &str {
                "writes every PreToolUse to ~/.reflect/audit.jsonl"
            }
            fn events(&self) -> &[HookEventKind] {
                &[HookEventKind::PreToolUse]
            }
        }
        let engine = HookEngine::new();
        engine.register(DescribedHook);
        let sums = engine.hook_summaries();
        assert_eq!(sums.len(), 1);
        assert_eq!(sums[0].0, "audit-logger");
        assert!(sums[0].1.contains("PreToolUse"));
    }

    /// 空 engine → 空 summaries。
    #[test]
    fn hook_summaries_empty() {
        let engine = HookEngine::new();
        assert!(engine.hook_summaries().is_empty());
    }
}
