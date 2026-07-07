//! `reflect-config` — 统一 TOML 配置加载 + 热重载。
//!
//! M7 起取代 `reflect-hooks::config::HooksConfig` 的单一职责，
//! 把 provider / compact / hooks 配置集中到 `~/.reflect/config.toml`。
//!
//! ## 模块
//! - [`schema`] —— 数据结构 (`ReflectConfig` 及各 section)
//! - [`load`] —— 单次同步加载
//! - [`watch`] —— 基于 `notify` 的热重载 (debounce 250ms)
//! - [`builder`] —— `ReflectConfig → ModelRegistry`
//!
//! ## 配置示例
//! ```toml
//! [active]
//! provider = "anthropic"
//!
//! [anthropic]
//! api_key = "sk-ant-..."
//! base_url = "https://api.anthropic.com"   # 可选
//! model = "claude-3-5-sonnet-latest"       # 可选
//!
//! [openai]
//! api_key = "sk-..."
//!
//! [compact]
//! trigger_tokens = 10000
//!
//! [hooks]
//! enabled = ["search_budget", "verification"]
//!
//! [hooks.search_budget]
//! max_calls = 20
//! ```

#![allow(clippy::derivable_impls)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::io_other_error)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::manual_div_ceil)]

pub mod analytics;
pub mod builder;
pub mod error;
pub mod feature_flags;
pub mod load;
pub mod schema;
pub mod watch;

pub use analytics::{
    ExporterGuard, Health, default_service_name, init_exporter, is_enabled as analytics_enabled,
    status_line as analytics_status_line,
};
pub use builder::{LspServerConfigShape, McpServerConfigShape};
pub use error::ConfigError;
pub use feature_flags::{is_feature_enabled, is_feature_enabled_with_compile_time};
pub use load::{default_config_path, load_default, load_from_file, load_from_str};
pub use schema::{
    ACP_DEFAULT_BIND, AcpSection, ActiveSection, AnalyticsSection, AnthropicSection,
    AskUserQuestionSection, BridgeSection, CompactSection, CoordinatorSection, DapSection,
    FeatureFlagsSection, HooksSection, LangfuseSection, LspFilePattern, LspServerEntry,
    LspServersSection, McpServerEntry, McpServersSection, McpTransport, ModelSection,
    NotificationsSection, OllamaSection, OpenAISection, PlanCompletionSection,
    PostgresSessionSection,
    ReadBeforeEditSection, ReflectConfig, ResolvedAskUserQuestion, SandboxSection, SanitizeSection,
    SearchBudgetSection, SseRedisSection, TestRunnerSection, TokenBudgetSection, VerificationSection,
    VoiceSection,
};
pub use watch::{ConfigWatcher, WatcherHandle};

/// 配置目录的默认相对名（位于 `$HOME` 下）。
pub const DEFAULT_CONFIG_DIR: &str = ".reflect";
/// 默认配置文件名。
pub const DEFAULT_CONFIG_FILE: &str = "config.toml";
