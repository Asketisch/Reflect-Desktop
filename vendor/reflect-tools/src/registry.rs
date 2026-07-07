//! `ToolRegistry` — name → `Arc<dyn Tool>` lookup with 3-source tagging.
//!
//! M3 supports `Builtin | Runtime | Plugin` source tags. Plugin
//! registration is a stub (M5).

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::spec::ToolSpec;
use crate::tool::Tool;

/// Where a tool came from. v1.0.0-rc2 起三个 source 全部正常工作 ——
///
/// `Builtin` 在 binary 启动期注册,`Runtime` 由用户代码或 MCP server 注入,
/// `Plugin` 由 `PluginManager` 在 plugin install / load 时挂载。
/// `unregister_source(ToolSource::Plugin)` 用于 unload 时一次性反注册
/// 某 plugin 的所有 tool。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolSource {
    /// Shipped with the binary (e.g. `bash`, `read`, `grep`).
    Builtin,
    /// Registered at runtime by user code (e.g. plugins, custom tools).
    Runtime,
    /// Loaded from a plugin via `PluginManager`.
    Plugin,
}

impl Default for ToolSource {
    fn default() -> Self {
        ToolSource::Builtin
    }
}

/// Thread-safe registry of available tools.
#[derive(Default)]
#[allow(clippy::type_complexity)] // pre-M5: simple type, complexity is acceptable
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, (ToolSource, Arc<dyn Tool>)>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool as `Builtin` (the default for built-in tools).
    pub fn register(&self, tool: Arc<dyn Tool>) {
        self.register_with_source(ToolSource::Builtin, tool);
    }

    /// Register a tool with an explicit source.
    ///
    /// v1.0.0-rc2 起 `ToolSource::Plugin` 不再 warn + fallthrough,而是
    /// 原样保留 source 标签 —— `PluginManager` 在 install / load 时通过
    /// 此 API 挂载 plugin 提供的能力,反注册走 `unregister(name)`(由
    /// `PluginManager::unload` 调用)。其他 source 行为保持原状。
    pub fn register_with_source(&self, source: ToolSource, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().insert(name, (source, tool));
    }

    /// 注册一个 plugin 提供的 tool —— `PluginSource::Plugin` source 的便捷方法。
    ///
    /// 等价于 `register_with_source(ToolSource::Plugin, tool)`。
    /// 通常 `PluginManager` 通过此 API 把 plugin manifest 里声明的能力
    /// 挂到主程序,卸载时用 `unregister(name)` 反注册。
    pub fn register_plugin_tool(&self, tool: Arc<dyn Tool>) {
        self.register_with_source(ToolSource::Plugin, tool);
    }

    /// 列出当前所有 tool 的 (name, source) 对。
    ///
    /// 主要给 `PluginManager` 用 —— 它需要知道哪些 tool 来自 plugin,
    /// 才能在 unload 时按 source tag 反注册(而非依赖 name 推测)。
    /// 顺序按 name 字典序,与 `list()` 一致。
    pub fn list_with_source(&self) -> Vec<(String, ToolSource)> {
        let mut out: Vec<_> = self
            .tools
            .read()
            .iter()
            .map(|(name, (source, _))| (name.clone(), *source))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// 反注册某个 source 下的所有 tool,返回实际移除的数量。
    ///
    /// `PluginManager::unload` 调用此方法一次清空该 plugin 的所有 tool;
    /// 也可用于测试清理。
    pub fn unregister_source(&self, source: ToolSource) -> usize {
        let mut map = self.tools.write();
        let before = map.len();
        map.retain(|_, (s, _)| *s != source);
        before - map.len()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().get(name).map(|(_, t)| t.clone())
    }

    /// Remove a tool by name. Returns `true` if a tool was actually
    /// removed. Used by v0.3 MCP integration to unregister tools when a
    /// server shuts down or restarts (`McpConnectionManager::reload`
    /// `on_remove` callback).
    ///
    /// 内部锁不向外暴露 —— caller 不需要关心三个 source 的存储细节。
    pub fn unregister(&self, name: &str) -> bool {
        self.tools.write().remove(name).is_some()
    }

    /// Register only if no tool with the same name is already registered.
    /// Returns `true` on successful insert, `false` if a tool with the
    /// given name is already present (caller should `warn!` and skip).
    ///
    /// v0.3 MCP 加载走此路径:`mcp__<server>__<tool>` 前缀保证与 7 个
    /// Reflect 内置 tool 不重名,但如果用户配置出 `mcp__fs__bash` 与
    /// builtin `bash` 仍然能并存(MCP 走 `mcp__` 前缀隔离)。如出现
    /// 真冲突(同名 tool 重复 register),返回 `false` 提示 caller warn。
    pub fn register_if_absent(&self, source: ToolSource, tool: Arc<dyn Tool>) -> bool {
        let mut map = self.tools.write();
        let name = tool.name().to_string();
        if map.contains_key(&name) {
            return false;
        }
        map.insert(name, (source, tool));
        true
    }

    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.read().keys().cloned().collect();
        names.sort();
        names
    }

    /// v1.1.0 Phase 4:批量注册工具,排除 `excluded` 列表里的工具名。
    ///
    /// 用法:`reflect_task::coordinator::build_worker_tool_registry`
    /// 把父 registry 的 builtin + runtime 工具复制到 worker registry,
    /// 同时排除 `INTERNAL_WORKER_TOOLS`(`TeamCreate` / `TeamDelete`
    /// / `SyntheticOutput` / `send_message`)。返回成功注册的工具数。
    ///
    /// 排除优先级高于同名覆盖:即使 caller 在 `tools` 里传了一个
    /// `TeamCreate`,worker registry 也不会拿到它(coordinator 拥有
    /// team 生命周期)。
    pub fn register_except(
        &self,
        source: ToolSource,
        tools: Vec<Arc<dyn Tool>>,
        excluded: &[&str],
    ) -> usize {
        let mut count = 0usize;
        for tool in tools {
            let name = tool.name();
            if excluded.contains(&name) {
                tracing::debug!(tool = %name, "register_except: 跳过 excluded tool");
                continue;
            }
            self.register_with_source(source, tool);
            count += 1;
        }
        count
    }

    /// Build a `ToolSpec` for each registered tool (M2 feeds this to
    /// `ChatRequest.tools`).
    pub fn list_specs(&self) -> Vec<ToolSpec> {
        self.tools
            .read()
            .values()
            .map(|(_, t)| ToolSpec::Function {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
                required_permission: t.required_permission(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Tool, ToolContext, ToolError};
    use async_trait::async_trait;
    use reflect_protocol::ToolOutput as Out;

    struct T;
    #[async_trait]
    impl Tool for T {
        fn name(&self) -> &str {
            "t"
        }
        fn description(&self) -> &str {
            "d"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type":"object"})
        }
        fn is_concurrency_safe(&self) -> bool {
            true
        }
        async fn execute(&self, _: ToolContext, _: serde_json::Value) -> Result<Out, ToolError> {
            unreachable!()
        }
    }

    #[test]
    fn register_get_list_specs() {
        let r = ToolRegistry::default();
        r.register(Arc::new(T));
        assert!(r.get("t").is_some());
        assert!(r.get("missing").is_none());
        assert_eq!(r.list(), vec!["t".to_string()]);
        let specs = r.list_specs();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name(), "t");
    }

    /// v0.3: MCP server 关闭时反注册 tool。
    #[test]
    fn unregister_removes_tool_by_name() {
        let r = ToolRegistry::default();
        r.register(Arc::new(T));
        assert!(r.get("t").is_some());
        assert!(r.unregister("t"));
        assert!(r.get("t").is_none());
        // 第二次 unregister 返回 false,语义幂等。
        assert!(!r.unregister("t"));
        assert!(!r.unregister("never_existed"));
    }

    /// v0.3: MCP 注册工具时同名冲突不覆盖,返回 false。
    #[test]
    fn register_if_absent_skips_on_collision() {
        let r = ToolRegistry::default();
        // 第一次注册 builtin 't' 成功。
        assert!(r.register_if_absent(ToolSource::Builtin, Arc::new(T)));
        // 第二次同源注册同名字 → 跳过。
        assert!(!r.register_if_absent(ToolSource::Runtime, Arc::new(T)));
        // builtin 't' 仍在。
        let t = r.get("t").expect("first register survives collision");
        assert_eq!(t.name(), "t");
    }

    /// v1.0.0-rc2: `ToolSource::Plugin` 不再 warn,保留 source tag。
    /// 这是 plugin 系统接入点 —— `PluginManager` 通过
    /// `register_plugin_tool` 挂载 plugin 能力。
    #[test]
    fn plugin_source_is_preserved_not_warned() {
        let r = ToolRegistry::default();
        r.register_plugin_tool(Arc::new(T));
        let pairs = r.list_with_source();
        assert_eq!(pairs, vec![("t".to_string(), ToolSource::Plugin)]);
    }

    /// `register_with_source(Plugin, _)` 与 `register_plugin_tool` 等价。
    #[test]
    fn register_with_source_plugin_equals_helper() {
        let r = ToolRegistry::default();
        r.register_with_source(ToolSource::Plugin, Arc::new(T));
        let (_, src) = r
            .list_with_source()
            .into_iter()
            .find(|(n, _)| n == "t")
            .unwrap();
        assert_eq!(src, ToolSource::Plugin);
    }

    /// `unregister_source` 只移除指定 source 的 tool,其他 source 不动。
    #[test]
    fn unregister_source_filters_by_tag() {
        let r = ToolRegistry::default();
        r.register_with_source(ToolSource::Builtin, Arc::new(T));
        struct U;
        #[async_trait]
        impl Tool for U {
            fn name(&self) -> &str {
                "u"
            }
            fn description(&self) -> &str {
                "d"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type":"object"})
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _: ToolContext,
                _: serde_json::Value,
            ) -> Result<Out, ToolError> {
                unreachable!()
            }
        }
        r.register_plugin_tool(Arc::new(U));
        let removed = r.unregister_source(ToolSource::Plugin);
        assert_eq!(removed, 1);
        // builtin 't' 保留,plugin 'u' 消失。
        assert!(r.get("t").is_some());
        assert!(r.get("u").is_none());
    }

    // ── v1.1.0 Phase 4: register_except ───────────────────────────

    /// 批量注册,排除名单外的工具成功;名单内跳过 + 计数不变。
    #[test]
    fn register_except_skips_excluded() {
        struct A;
        #[async_trait]
        impl Tool for A {
            fn name(&self) -> &str {
                "alpha"
            }
            fn description(&self) -> &str {
                "d"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type":"object"})
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _: ToolContext,
                _: serde_json::Value,
            ) -> Result<Out, ToolError> {
                unreachable!()
            }
        }
        struct B;
        #[async_trait]
        impl Tool for B {
            fn name(&self) -> &str {
                "beta"
            }
            fn description(&self) -> &str {
                "d"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type":"object"})
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _: ToolContext,
                _: serde_json::Value,
            ) -> Result<Out, ToolError> {
                unreachable!()
            }
        }
        let r = ToolRegistry::default();
        let n = r.register_except(
            ToolSource::Builtin,
            vec![Arc::new(A), Arc::new(B)],
            &["beta"],
        );
        assert_eq!(n, 1, "beta 应被排除");
        assert!(r.get("alpha").is_some());
        assert!(r.get("beta").is_none(), "beta 不应注册");
    }

    /// 空 excluded 列表 = 全部注册(等价于遍历 register_with_source)。
    #[test]
    fn register_except_with_empty_excluded_registers_all() {
        struct A;
        #[async_trait]
        impl Tool for A {
            fn name(&self) -> &str {
                "alpha"
            }
            fn description(&self) -> &str {
                "d"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type":"object"})
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _: ToolContext,
                _: serde_json::Value,
            ) -> Result<Out, ToolError> {
                unreachable!()
            }
        }
        struct B;
        #[async_trait]
        impl Tool for B {
            fn name(&self) -> &str {
                "beta"
            }
            fn description(&self) -> &str {
                "d"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type":"object"})
            }
            fn is_concurrency_safe(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _: ToolContext,
                _: serde_json::Value,
            ) -> Result<Out, ToolError> {
                unreachable!()
            }
        }
        let r = ToolRegistry::default();
        let n = r.register_except(ToolSource::Runtime, vec![Arc::new(A), Arc::new(B)], &[]);
        assert_eq!(n, 2);
    }
}
