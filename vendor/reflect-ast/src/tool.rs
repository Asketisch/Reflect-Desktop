//! `AstTool` — 单一 AST tool 暴露给 LLM。
//!
//! 整个 tree-sitter 集成对 LLM 呈现为一个 tool:`ast`。`action` 字段
//! 决定走哪条路径(`list_languages` / `search` / `replace` /
//! `rename_symbol`),`path` / `pattern` / `replacement` 等按 action 在
//! P1+ 增量补全。P0 仅 `list_languages` 真正实现,其他 action 返回
//! "not implemented" 错误,方便上层尽早发现 wiring 问题而不影响行为。
//!
//! ## 参数 schema
//!
//! ```json
//! {
//!   "type": "object",
//!   "properties": {
//!     "action": { "type": "string", "enum": ["list_languages", "search",
//!                                              "replace", "rename_symbol"] },
//!     "path":   { "type": "string" },                 // P1+: search / replace
//!     "pattern":{ "type": "string" },                 // P1+: search / replace
//!     "replacement": { "type": "string" },            // P3: replace
//!     "from":   { "type": "string" },                 // P3: rename_symbol
//!     "to":     { "type": "string" },                  // P3: rename_symbol
//!     "include":   { "type": "string" },              // P2: text 模式 include glob
//!     "max_results": { "type": "number" }              // P2: text 模式截断
//!   },
//!   "required": ["action"],
//!   "additionalProperties": true
//! }
//! ```
//!
//! ## 权限 (v1.0.0-rc1+)
//!
//! - `required_permission = Auto` (tool-level fallback)。
//! - `action_permission(&args)` per-action 路由:`replace` / `rename_symbol`
//!   返回 `Prompt` 走 `ApprovalGate`;`list_languages` / `search` 返回 `Auto`。
//! - `is_concurrency_safe = false` —— 保守值;read-only 工具可后续切 true。
//!
//! ## Search action (P1/P2)
//!
//! - `kind:<node>` / `regex:<re>` —— 走 `LanguageGrid::parse` + tree-sitter walk。
//!   `path` 必填(单文件),`pattern` 必填;`include` / `max_results` 仅对 `text:` 生效。
//! - `text:<literal>` —— 走 `ignore::WalkBuilder`,`path` 是搜索根目录(可省,
//!   默认 workspace 根)。返回 `path:line:text` 行格式(对齐 `grep` builtin)。
//!
//! ## Replace / RenameSymbol (P3)
//!
//! - `replace` / `rename_symbol` 是 mutation,经 `Tool::action_permission` 切
//!   `Prompt`(走 `ApprovalGate`)。当前 stub 返回 P3 placeholder。
//!
//! ## 错误映射
//!
//! - 未知 `action` → `ToolError::InvalidArgs`(serde 反序列化失败)
//! - `search` 路径上 `path` / `pattern` 缺失 → `ToolError::InvalidArgs`
//! - 后缀无 grammar / parse 失败 → `ToolError::InvalidArgs`(转 `AstError`)
//! - 读文件 IO 错 → `ToolError::Io`
//! - 写文件 IO 错(P3) → `ToolError::Io`

use std::fs;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use reflect_protocol::{ContentBlock, PermissionMode, RiskLevel, ToolError, ToolOutput};
use reflect_tools::Tool;
use serde_json::json;

use crate::action::AstAction;
use crate::edit::{self};
use crate::grid::{AstError, LanguageGrid};
use crate::matcher::{self, CompiledPattern};

/// AST tool 实例。
///
/// `bootstrap_ast`(`reflect-exec::bootstrap_ast` 类似 `bootstrap_lsp`)
/// 构造一个 `LanguageGrid::with_defaults()` 包成 `Arc`,挂到
/// `ToolRegistry::register(Builtin, ...)`(`reflect-tui` / `reflect-exec`
/// / `reflect` facade 三处注册)。`grid` 是 `Arc<LanguageGrid>`,
/// 进程内全局共享一份。
pub struct AstTool {
    pub grid: Arc<LanguageGrid>,
}

impl AstTool {
    /// 构造 tool,默认 grammar bundle。
    pub fn new() -> Self {
        Self {
            grid: LanguageGrid::with_defaults(),
        }
    }

    /// 自定义 grammar 注册表(测试 / 扩展 bundle 用)。
    pub fn with_grid(grid: Arc<LanguageGrid>) -> Self {
        Self { grid }
    }
}

impl Default for AstTool {
    fn default() -> Self {
        Self::new()
    }
}

/// `text:` 模式缺省截断上限(对齐 `grep` builtin 的 `DEFAULT_MAX_RESULTS`)。
const TEXT_DEFAULT_MAX_RESULTS: usize = 100;

#[async_trait]
impl Tool for AstTool {
    fn name(&self) -> &str {
        "ast"
    }

    fn description(&self) -> &str {
        "基于 tree-sitter 的结构化代码搜索 / 重写。\
         action ∈ {list_languages, search, replace, rename_symbol}。\
         list_languages 列出当前 build 支持的 grammar;\
         search 按 pattern 在 path 下做结构化匹配(只读);\
         replace 用 replacement 改写每个匹配节点(写);\
         rename_symbol 单文件内按 from→to 重命名(写)。\
         pattern 前缀 kind:<node> 走 tree-sitter 节点 kind 匹配;\
         regex:<re> 走 leaf 文本正则;text:<literal> 走纯文本行扫描(无需 grammar)。"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list_languages", "search", "replace", "rename_symbol"],
                    "description": "AST action to perform"
                },
                "path": {
                    "type": "string",
                    "description": "File path relative to workspace (search/replace/rename_symbol); for text: mode it's the search root (default: workspace)"
                },
                "pattern": {
                    "type": "string",
                    "description": "Search pattern with prefix (kind:<node> | regex:<re> | text:<literal>)"
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement text for replace action (P3)"
                },
                "from": {
                    "type": "string",
                    "description": "Original symbol name for rename_symbol (P3)"
                },
                "to": {
                    "type": "string",
                    "description": "New symbol name for rename_symbol (P3)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob filter for text: mode only (e.g. '*.rs')"
                },
                "max_results": {
                    "type": "number",
                    "description": "Max hits to return (text: mode only, default 100)"
                }
            },
            "required": ["action"],
            "additionalProperties": true
        })
    }

    fn is_concurrency_safe(&self) -> bool {
        // P0 保守:串行。P3 切 per-action 表(read action → true,
        // mutation → false)。
        false
    }

    fn required_permission(&self) -> PermissionMode {
        // Tool-level fallback:`search` / `list_languages` 走 Auto;
        // `replace` / `rename_symbol` 在 `action_permission` 里切到 Prompt。
        // 这里返回 Auto 是因为 `action_permission` 在 queue 路径上覆盖
        // 这个值,返回 Prompt 只会让 LLM 在 search 时也触发 approval modal。
        PermissionMode::Auto
    }

    /// Per-action 权限路由(v1.0.0-rc1+):`replace` / `rename_symbol` 切到
    /// `Prompt`,走 `ApprovalGate` 让用户在 TUI modal 显式确认;`list_languages`
    /// / `search` 保持 `Auto`(read-only)。`PlanModeGate` hook 也按此路由
    /// 拒绝 mutation,见 `crates/reflect-hooks/src/builtins/plan_mode_gate.rs`。
    fn action_permission(&self, args: &serde_json::Value) -> PermissionMode {
        let action = args.get("action").and_then(|v| v.as_str());
        match action {
            Some("replace") | Some("rename_symbol") => PermissionMode::Prompt,
            // 缺省走 required_permission()(默认 Auto)
            _ => PermissionMode::Auto,
        }
    }

    async fn execute(
        &self,
        ctx: reflect_tools::ToolContext,
        args: serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        // 1. 解析 action 字段(仅此字段,P1+ 再扩)。`args.get("action")`
        //    返回 `Option<&Value>`,再 `serde_json::from_value` 反序列化
        //    到 `AstAction`,这样 `path` / `pattern` 等多余字段不会触发
        //    "expected map with a single key" 的反序列化错误。
        let action: AstAction =
            serde_json::from_value(args.get("action").cloned().ok_or_else(|| {
                ToolError::InvalidArgs {
                    message: "ast: missing required field 'action'".to_string(),
                }
            })?)
            .map_err(|e: serde_json::Error| ToolError::InvalidArgs {
                message: format!("ast: invalid arguments: {e}"),
            })?;

        match action {
            AstAction::ListLanguages => self.exec_list_languages(),
            AstAction::Search => self.exec_search(&ctx, &args).await,
            AstAction::Replace => self.exec_replace(&ctx, &args).await,
            AstAction::RenameSymbol => self.exec_rename_symbol(&ctx, &args).await,
        }
    }
}

impl AstTool {
    /// `action = list_languages` —— 返回当前 build 启用的 grammar ID
    /// 列表(JSON 数组)。
    fn exec_list_languages(&self) -> Result<ToolOutput, ToolError> {
        let langs = self.grid.list();
        let body = serde_json::to_string_pretty(&langs).unwrap_or_else(|_| format!("{langs:?}"));
        Ok(ToolOutput {
            content: vec![ContentBlock::text(body)],
            is_error: false,
            metadata: json!({
                "count": langs.len(),
                "languages": langs,
            }),
            elapsed_ms: 0,
        })
    }

    /// `action = search` —— 走 `matcher::search_in_tree` (kind:/regex:) 或
    /// `matcher::search_text` (text:)。
    ///
    /// 路径解析:
    /// - `kind:` / `regex:`:`path` 必填(单文件),workspace 相对,经 sandbox
    ///   校验(sandbox 失败 = PathEscape)。
    /// - `text:`:`path` 可选,缺省走 workspace 根;`include` / `max_results` 可选。
    async fn exec_search(
        &self,
        ctx: &reflect_tools::ToolContext,
        args: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let pattern_raw = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "ast search: missing 'pattern'".to_string(),
            })?;
        let compiled = CompiledPattern::compile(pattern_raw).map_err(ast_error_to_tool_error)?;

        match &compiled {
            CompiledPattern::Text(needle) => {
                // text: 走文件系统,可省 path(默认 workspace),include/max_results 可选。
                let root = match args.get("path").and_then(|v| v.as_str()) {
                    Some(p) => reflect_tools::sandbox::resolve_sandbox_path(
                        &ctx.workspace_path(),
                        Path::new(p),
                    )?,
                    None => ctx.workspace_path(),
                };
                let include = args.get("include").and_then(|v| v.as_str());
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(TEXT_DEFAULT_MAX_RESULTS);
                let hits = matcher::search_text(&root, needle, include, max_results)
                    .map_err(ast_error_to_tool_error)?;
                Ok(self.hits_to_tool_output(hits, "text"))
            }
            CompiledPattern::Kind(_) | CompiledPattern::Regex(_) => {
                // kind:/regex: 必须有 path(单文件),走 sandbox + grid.parse。
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArgs {
                        message: format!(
                            "ast search: kind:/regex: 模式必须指定 'path'(单文件);pattern = {pattern_raw:?}"
                        ),
                    })?;
                let abs = reflect_tools::sandbox::resolve_sandbox_path(
                    &ctx.workspace_path(),
                    Path::new(path_str),
                )?;
                let source = fs::read_to_string(&abs)
                    .map_err(|e| ToolError::Io(format!("read {path_str}: {e}")))?;
                let lang = self
                    .grid
                    .for_ext(&abs)
                    .ok_or_else(|| ToolError::InvalidArgs {
                        message: format!(
                            "ast search: no grammar for extension of {path_str}\
                             (supported: {:?})",
                            self.grid.list()
                        ),
                    })?;
                let mut hits = matcher::search_in_tree(&self.grid, lang, &source, &compiled)
                    .map_err(ast_error_to_tool_error)?;
                // 写入 file 字段(walk 阶段 file 为空字符串占位)。
                for h in &mut hits {
                    h.file = path_str.to_string();
                }
                Ok(self.hits_to_tool_output(hits, "kind_or_regex"))
            }
        }
    }

    /// 把 `Vec<Hit>` 序列化成 JSON 文本返回。
    ///
    /// `mode` 标签("text" / "kind_or_regex")只是 metadata 备注,便于上层
    /// 区分 text 模式与 AST 模式(行为上有差异:text 模式 file 可能是绝对路径)。
    fn hits_to_tool_output(
        &self,
        hits: Vec<crate::matcher::Hit>,
        mode: &'static str,
    ) -> ToolOutput {
        let body = serde_json::to_string_pretty(&hits).unwrap_or_else(|_| format!("{hits:?}"));
        ToolOutput {
            content: vec![ContentBlock::text(body)],
            is_error: false,
            metadata: json!({
                "count": hits.len(),
                "mode": mode,
            }),
            elapsed_ms: 0,
        }
    }

    /// `action = replace` —— 用 `replacement` 改写每个匹配 `kind:<node>`
    /// 的节点,产生 unified diff 后写盘。
    ///
    /// 流程:
    /// 1. 解析 `path` / `pattern` / `replacement` —— `pattern` 必须是
    ///    `kind:<node>`(`regex:` / `text:` 在 `replace` 路径上无意义)。
    /// 2. `ctx.approval` 主动 ask —— Prompt permission,等待用户 modal 决策。
    /// 3. 读源文件 → `grid.parse` → `apply_replace` → 写回 → 返回 diff。
    ///
    /// 错误情形:
    /// - `path` / `pattern` / `replacement` 缺失 → `InvalidArgs`。
    /// - `pattern` 不是 `kind:` 前缀 → `InvalidArgs`。
    /// - extension 不在 grammar 表里 → `InvalidArgs`。
    /// - approval `Deny` → `ToolOutput { is_error: true, .. }`(不抛 `Err`)。
    /// - 读 / 写 IO 错 → `ToolError::Io`。
    async fn exec_replace(
        &self,
        ctx: &reflect_tools::ToolContext,
        args: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let path_str =
            args.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArgs {
                    message: "ast replace: missing 'path'".to_string(),
                })?;
        let pattern_raw = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "ast replace: missing 'pattern'".to_string(),
            })?;
        let replacement = args
            .get("replacement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "ast replace: missing 'replacement'".to_string(),
            })?;
        // pattern 必须是 `kind:<node>`(regex:/text: 在 replace 路径上语义
        // 不清,需要先 search 拿到 byte range 才能替换;留给 v1+ 单独路径)。
        let compiled = CompiledPattern::compile(pattern_raw).map_err(ast_error_to_tool_error)?;
        let target_kind = match &compiled {
            CompiledPattern::Kind(k) => k.clone(),
            _ => {
                return Err(ToolError::InvalidArgs {
                    message: format!(
                        "ast replace: pattern must be kind:<node>, got {pattern_raw:?}"
                    ),
                });
            }
        };

        // Approval gate:Prompt 模式 —— headless / 无 gate 时按 Auto
        // 走(等价于 M6 之前行为);有 gate 时调用 ask_tool 等待用户决策。
        if let Some(gate) = &ctx.approval {
            let mut preview_args = args.clone();
            if let Some(obj) = preview_args.as_object_mut() {
                obj.insert("action".into(), json!("replace"));
            }
            let decision = gate
                .ask_tool("ast", &preview_args, RiskLevel::High, &ctx.cancel)
                .await;
            if let reflect_protocol::ReviewDecision::Deny { reason } = decision {
                return Ok(ToolOutput {
                    content: vec![ContentBlock::text(format!(
                        "ast replace: approval denied: {reason}"
                    ))],
                    is_error: true,
                    metadata: json!({"approval": "denied", "action": "replace"}),
                    elapsed_ms: 0,
                });
            }
            // Approve / ApproveForSession 走下面写盘流程。
        }

        // 路径 sandbox + 读源 + parse + apply_replace + 写盘。
        let abs = reflect_tools::sandbox::resolve_sandbox_path(
            &ctx.workspace_path(),
            Path::new(path_str),
        )?;
        let source =
            fs::read_to_string(&abs).map_err(|e| ToolError::Io(format!("read {path_str}: {e}")))?;
        let lang = self
            .grid
            .for_ext(&abs)
            .ok_or_else(|| ToolError::InvalidArgs {
                message: format!("ast replace: no grammar for extension of {path_str}"),
            })?;
        let tree = self
            .grid
            .parse(lang, &source)
            .map_err(ast_error_to_tool_error)?;
        let (new_source, unified_diff) =
            edit::apply_replace(&source, &tree, &target_kind, replacement)
                .map_err(ast_error_to_tool_error)?;

        // diff 为空(没有命中 target_kind)时跳过写盘,但返回 is_error: false
        // + metadata `replacements: 0`,让 LLM 看到"无 op"而不是误以为
        // 出错。
        if new_source == source {
            return Ok(ToolOutput {
                content: vec![ContentBlock::text(format!(
                    "ast replace: no '{target_kind}' nodes found in {path_str}"
                ))],
                is_error: false,
                metadata: json!({
                    "path": path_str,
                    "target_kind": target_kind,
                    "replacements": 0,
                    "diff": "",
                }),
                elapsed_ms: 0,
            });
        }

        fs::write(&abs, &new_source)
            .map_err(|e| ToolError::Io(format!("write {path_str}: {e}")))?;
        // 统计实际替换次数(从 diff 行数估算;不严格,够 LLM 看就行)。
        let replacements = unified_diff.matches("+fn").count()
            + unified_diff.matches("+struct").count()
            + unified_diff.matches("+impl").count();
        Ok(ToolOutput {
            content: vec![ContentBlock::Diff {
                unified_diff: unified_diff.clone(),
            }],
            is_error: false,
            metadata: json!({
                "path": path_str,
                "target_kind": target_kind,
                "replacements": replacements,
                "diff": unified_diff,
            }),
            elapsed_ms: 0,
        })
    }

    /// `action = rename_symbol` —— 单文件 word-boundary 正则替换
    /// (`\b<from>\b` → `to`)。v1 不跨文件;AST-scoped 跨文件 rename 留 v1+。
    async fn exec_rename_symbol(
        &self,
        ctx: &reflect_tools::ToolContext,
        args: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        let path_str =
            args.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArgs {
                    message: "ast rename_symbol: missing 'path'".to_string(),
                })?;
        let from =
            args.get("from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArgs {
                    message: "ast rename_symbol: missing 'from'".to_string(),
                })?;
        let to = args
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs {
                message: "ast rename_symbol: missing 'to'".to_string(),
            })?;

        // Approval gate。
        if let Some(gate) = &ctx.approval {
            let mut preview_args = args.clone();
            if let Some(obj) = preview_args.as_object_mut() {
                obj.insert("action".into(), json!("rename_symbol"));
            }
            let decision = gate
                .ask_tool("ast", &preview_args, RiskLevel::High, &ctx.cancel)
                .await;
            if let reflect_protocol::ReviewDecision::Deny { reason } = decision {
                return Ok(ToolOutput {
                    content: vec![ContentBlock::text(format!(
                        "ast rename_symbol: approval denied: {reason}"
                    ))],
                    is_error: true,
                    metadata: json!({"approval": "denied", "action": "rename_symbol"}),
                    elapsed_ms: 0,
                });
            }
        }

        let abs = reflect_tools::sandbox::resolve_sandbox_path(
            &ctx.workspace_path(),
            Path::new(path_str),
        )?;
        let source =
            fs::read_to_string(&abs).map_err(|e| ToolError::Io(format!("read {path_str}: {e}")))?;
        let (new_source, unified_diff, count) =
            edit::apply_rename(&source, from, to).map_err(ast_error_to_tool_error)?;

        if count == 0 {
            return Ok(ToolOutput {
                content: vec![ContentBlock::text(format!(
                    "ast rename_symbol: no occurrences of {from:?} in {path_str}"
                ))],
                is_error: false,
                metadata: json!({
                    "path": path_str,
                    "from": from,
                    "to": to,
                    "replacements": 0,
                    "diff": "",
                }),
                elapsed_ms: 0,
            });
        }

        fs::write(&abs, &new_source)
            .map_err(|e| ToolError::Io(format!("write {path_str}: {e}")))?;
        Ok(ToolOutput {
            content: vec![ContentBlock::Diff {
                unified_diff: unified_diff.clone(),
            }],
            is_error: false,
            metadata: json!({
                "path": path_str,
                "from": from,
                "to": to,
                "replacements": count,
                "diff": unified_diff,
            }),
            elapsed_ms: 0,
        })
    }
}

/// `AstError` → `ToolError` 映射。`InvalidArgs` 涵盖 `UnsupportedLanguage` /
/// `BadPattern`,`Io` 走 `Io`,`Parse` 视为 `Execution` 错(实际几乎不触发)。
fn ast_error_to_tool_error(e: AstError) -> ToolError {
    match e {
        AstError::UnsupportedLanguage(m) | AstError::BadPattern(m) => {
            ToolError::InvalidArgs { message: m }
        }
        AstError::Parse(m) => ToolError::Execution(format!("ast parse: {m}")),
        AstError::Io(m) => ToolError::Io(m),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn tmp_workspace() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "reflect_ast_tool_test_{}_{}",
            std::process::id(),
            n
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).ok();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_ctx(workspace: &std::path::Path) -> reflect_tools::ToolContext {
        reflect_tools::ToolContext::for_workspace(workspace)
    }

    #[test]
    fn tool_name_and_description_static() {
        let tool = AstTool::new();
        assert_eq!(tool.name(), "ast");
        assert!(tool.description().contains("list_languages"));
        assert!(tool.description().contains("kind:"));
    }

    #[test]
    fn p0_required_permission_is_auto() {
        let tool = AstTool::new();
        assert_eq!(tool.required_permission(), PermissionMode::Auto);
        assert!(!tool.is_concurrency_safe());
    }

    #[test]
    fn parameters_schema_has_action_required() {
        let tool = AstTool::new();
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().expect("required array");
        let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            names.contains(&"action"),
            "schema should require 'action', got {names:?}"
        );
        let enums = schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action enum array");
        let values: Vec<&str> = enums.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(
            values,
            vec!["list_languages", "search", "replace", "rename_symbol"]
        );
    }

    #[tokio::test]
    async fn list_languages_returns_default_grammars() {
        let tool = AstTool::new();
        let out = tool
            .execute(
                reflect_tools::ToolContext::default(),
                json!({"action": "list_languages"}),
            )
            .await
            .expect("execute list_languages");
        assert!(!out.is_error);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("rust"), "missing rust: {text}");
                assert!(text.contains("python"), "missing python: {text}");
                assert!(text.contains("javascript"), "missing javascript: {text}");
            }
            other => panic!("expected Text block, got {other:?}"),
        }
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 6); // rust + ts + tsx + py + go + js
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = AstTool::new();
        let err = tool
            .execute(
                reflect_tools::ToolContext::default(),
                json!({"action": "frobnicate"}),
            )
            .await
            .expect_err("unknown action should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn search_kind_finds_function_items() {
        let ws = tmp_workspace();
        let src = "fn alpha() {}\nfn beta() {}\nstruct S;\n";
        std::fs::write(ws.join("a.rs"), src).unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.rs",
                    "pattern": "kind:function_item",
                }),
            )
            .await
            .expect("execute search kind:");
        assert!(!out.is_error, "got: {out:?}");
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 2);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("alpha"), "missing alpha: {text}");
                assert!(text.contains("beta"), "missing beta: {text}");
                assert!(text.contains("\"file\": \"a.rs\""));
            }
            other => panic!("expected Text block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn search_kind_unknown_returns_empty() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn main() {}").unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.rs",
                    "pattern": "kind:nonexistent_kind_xyz",
                }),
            )
            .await
            .expect("execute search kind: unknown");
        assert!(!out.is_error);
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 0);
    }

    #[tokio::test]
    async fn search_kind_rejects_missing_path() {
        let ws = tmp_workspace();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({"action": "search", "pattern": "kind:function_item"}),
            )
            .await
            .expect_err("missing path should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn search_kind_rejects_bad_pattern() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn main() {}").unwrap();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({"action": "search", "path": "a.rs", "pattern": "function_item"}),
            )
            .await
            .expect_err("missing prefix should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn search_kind_rejects_unsupported_extension() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.xyz"), "fn main() {}").unwrap();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.xyz",
                    "pattern": "kind:function_item",
                }),
            )
            .await
            .expect_err("unsupported extension should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn search_kind_python_function_definition() {
        let ws = tmp_workspace();
        std::fs::write(
            ws.join("a.py"),
            "def alpha():\n    pass\ndef beta():\n    pass\n",
        )
        .unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.py",
                    "pattern": "kind:function_definition",
                }),
            )
            .await
            .expect("execute search kind: function_definition");
        assert!(!out.is_error);
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 2);
    }

    #[tokio::test]
    async fn search_kind_tsx_finds_jsx_element() {
        let ws = tmp_workspace();
        std::fs::write(
            ws.join("a.tsx"),
            "const x = <div>hi</div>;\nfunction f() { return <span/>; }\n",
        )
        .unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.tsx",
                    "pattern": "kind:jsx_element",
                }),
            )
            .await
            .expect("execute search kind: jsx_element");
        assert!(!out.is_error);
        assert!(out.metadata["count"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn search_regex_matches_fn_token() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn foo() {}\nfn bar() {}\nstruct S;\n").unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.rs",
                    "pattern": "regex:^fn",
                }),
            )
            .await
            .expect("execute search regex:");
        assert!(!out.is_error);
        // 至少 2 个 `fn ` token 命中
        assert!(out.metadata["count"].as_u64().unwrap() >= 2);
    }

    #[tokio::test]
    async fn search_text_walks_workspace() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "alpha\nTODO: do thing\nbeta").unwrap();
        std::fs::write(ws.join("b.txt"), "TODO: also here").unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": ".",
                    "pattern": "text:TODO",
                }),
            )
            .await
            .expect("execute search text:");
        assert!(!out.is_error);
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 2);
    }

    #[tokio::test]
    async fn search_text_default_root_is_workspace() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "TODO: visible").unwrap();
        let tool = AstTool::new();
        // path 缺省 → 走 workspace 根
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({"action": "search", "pattern": "text:TODO"}),
            )
            .await
            .expect("execute search text: default root");
        assert!(!out.is_error);
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn search_text_respects_gitignore() {
        let ws = tmp_workspace();
        std::fs::create_dir_all(ws.join("target")).unwrap();
        std::fs::write(ws.join("target/ignored.rs"), "TODO: hidden").unwrap();
        std::fs::write(ws.join("a.rs"), "TODO: visible").unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({"action": "search", "path": ".", "pattern": "text:TODO"}),
            )
            .await
            .expect("execute search text: gitignore");
        assert!(!out.is_error);
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 1);
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("a.rs"), "should include a.rs");
                assert!(
                    !text.contains("target/ignored"),
                    "should exclude target/: {text}"
                );
            }
            other => panic!("expected Text block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn search_text_rejects_invalid_regex() {
        // text 模式 needle 不是 regex,不应被 regex 编译;这里仅 sanity check:
        // 包含 "[" 这种 regex 特殊字符也能字面命中。
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "let x = arr[0];\n").unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": ".",
                    "pattern": "text:arr[0]",
                }),
            )
            .await
            .expect("text: with brackets");
        assert!(!out.is_error);
        assert_eq!(out.metadata["count"].as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn search_regex_compile_error_returns_invalid_args() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn main() {}").unwrap();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "search",
                    "path": "a.rs",
                    "pattern": "regex:[",
                }),
            )
            .await
            .expect_err("invalid regex should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn action_permission_routes_mutation_to_prompt() {
        let tool = AstTool::new();
        // read actions: Auto
        for action in ["list_languages", "search"] {
            let p = tool.action_permission(&json!({"action": action}));
            assert_eq!(
                p,
                PermissionMode::Auto,
                "{action} should be Auto, got {p:?}"
            );
        }
        // mutation actions: Prompt
        for action in ["replace", "rename_symbol"] {
            let p = tool.action_permission(&json!({"action": action}));
            assert_eq!(
                p,
                PermissionMode::Prompt,
                "{action} should be Prompt, got {p:?}"
            );
        }
        // missing action 走 default(Auto)
        assert_eq!(tool.action_permission(&json!({})), PermissionMode::Auto);
    }

    #[tokio::test]
    async fn replace_action_writes_unified_diff() {
        let ws = tmp_workspace();
        let src = "fn foo() {}\nfn bar() {}\nstruct S;\n";
        std::fs::write(ws.join("a.rs"), src).unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "replace",
                    "path": "a.rs",
                    "pattern": "kind:function_item",
                    "replacement": "fn REPLACED() {}",
                }),
            )
            .await
            .expect("execute replace");
        assert!(!out.is_error, "got: {out:?}");
        // 文件被改写
        let new_src = std::fs::read_to_string(ws.join("a.rs")).unwrap();
        assert!(!new_src.contains("fn foo"));
        assert!(!new_src.contains("fn bar"));
        assert!(new_src.contains("fn REPLACED"));
        assert!(new_src.contains("struct S"));
        // ContentBlock::Diff 包含 unified diff
        assert!(matches!(
            &out.content[0],
            reflect_protocol::ContentBlock::Diff { .. }
        ));
        assert_eq!(out.metadata["target_kind"], "function_item");
    }

    #[tokio::test]
    async fn replace_action_no_match_skips_write() {
        let ws = tmp_workspace();
        let src = "fn foo() {}\n";
        std::fs::write(ws.join("a.rs"), src).unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "replace",
                    "path": "a.rs",
                    "pattern": "kind:nonexistent_kind_xyz",
                    "replacement": "X",
                }),
            )
            .await
            .expect("execute replace no match");
        assert!(!out.is_error);
        assert_eq!(out.metadata["replacements"].as_u64().unwrap(), 0);
        // 文件保持原样
        assert_eq!(std::fs::read_to_string(ws.join("a.rs")).unwrap(), src);
    }

    #[tokio::test]
    async fn replace_action_rejects_missing_replacement() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn foo() {}").unwrap();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "replace",
                    "path": "a.rs",
                    "pattern": "kind:function_item",
                }),
            )
            .await
            .expect_err("missing replacement should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn replace_action_rejects_non_kind_pattern() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn foo() {}").unwrap();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "replace",
                    "path": "a.rs",
                    "pattern": "regex:^fn",
                    "replacement": "X",
                }),
            )
            .await
            .expect_err("non-kind pattern should error");
        match err {
            ToolError::InvalidArgs { message } => {
                assert!(message.contains("kind:"), "got: {message}");
            }
            other => panic!("expected InvalidArgs, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rename_symbol_writes_word_boundary_diff() {
        let ws = tmp_workspace();
        let src = "let foo = 1;\nlet foobar = 2;\n";
        std::fs::write(ws.join("a.rs"), src).unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "rename_symbol",
                    "path": "a.rs",
                    "from": "foo",
                    "to": "bar",
                }),
            )
            .await
            .expect("execute rename_symbol");
        assert!(!out.is_error, "got: {out:?}");
        let new_src = std::fs::read_to_string(ws.join("a.rs")).unwrap();
        // word-boundary:foobar 不动
        assert!(new_src.contains("let bar = 1"));
        assert!(new_src.contains("let foobar = 2"));
        assert_eq!(out.metadata["from"], "foo");
        assert_eq!(out.metadata["to"], "bar");
        assert_eq!(out.metadata["replacements"].as_u64().unwrap(), 1);
        // ContentBlock::Diff
        assert!(matches!(
            &out.content[0],
            reflect_protocol::ContentBlock::Diff { .. }
        ));
    }

    #[tokio::test]
    async fn rename_symbol_no_match_skips_write() {
        let ws = tmp_workspace();
        let src = "let x = 1;\n";
        std::fs::write(ws.join("a.rs"), src).unwrap();
        let tool = AstTool::new();
        let out = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "rename_symbol",
                    "path": "a.rs",
                    "from": "missing_symbol",
                    "to": "renamed",
                }),
            )
            .await
            .expect("execute rename no match");
        assert!(!out.is_error);
        assert_eq!(out.metadata["replacements"].as_u64().unwrap(), 0);
        assert_eq!(std::fs::read_to_string(ws.join("a.rs")).unwrap(), src);
    }

    #[tokio::test]
    async fn rename_symbol_rejects_empty_from() {
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "let x = 1;").unwrap();
        let tool = AstTool::new();
        let err = tool
            .execute(
                make_ctx(&ws),
                json!({
                    "action": "rename_symbol",
                    "path": "a.rs",
                    "from": "",
                    "to": "renamed",
                }),
            )
            .await
            .expect_err("empty from should error");
        assert!(matches!(err, ToolError::InvalidArgs { .. }), "got: {err:?}");
    }

    #[tokio::test]
    async fn mutation_actions_return_error_when_approval_denied() {
        use std::sync::Arc;
        let ws = tmp_workspace();
        std::fs::write(ws.join("a.rs"), "fn foo() {}").unwrap();
        let tool = AstTool::new();
        // 构造一个 ApprovalGate + 一个后台 task 监听 ApprovalRequest 立刻 Deny。
        let (tx, mut rx) = tokio::sync::mpsc::channel::<reflect_protocol::Event>(4);
        let gate = Arc::new(reflect_tools::ApprovalGate::new(tx, "sub-test"));
        // 共享 gate 给 task —— 用 `with_session_allow` 把同一个 waiters 暴露出来。
        // 但更简单:clone gate 后 spawn,主路径用同一个 gate。
        let gate_for_task = gate.clone();
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                if let reflect_protocol::EventMsg::ApprovalRequest(req) = ev.msg {
                    let _ = gate_for_task.complete(
                        &req.request_id,
                        reflect_protocol::ReviewDecision::Deny {
                            reason: "user said no".into(),
                        },
                    );
                    break;
                }
            }
        });

        let mut ctx = make_ctx(&ws);
        ctx.approval = Some(gate);
        let out = tool
            .execute(
                ctx,
                json!({
                    "action": "replace",
                    "path": "a.rs",
                    "pattern": "kind:function_item",
                    "replacement": "fn REPLACED() {}",
                }),
            )
            .await
            .expect("execute replace with denied approval");
        assert!(out.is_error, "should error on deny, got: {out:?}");
        match &out.content[0] {
            reflect_protocol::ContentBlock::Text { text } => {
                assert!(text.contains("approval denied"), "got: {text}");
            }
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(out.metadata["approval"], "denied");
        // 文件未被改写
        let after = std::fs::read_to_string(ws.join("a.rs")).unwrap();
        assert!(after.contains("fn foo"));
        assert!(!after.contains("fn REPLACED"));
    }
}
