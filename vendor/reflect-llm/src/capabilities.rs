//! Provider/model capability flags.

/// Static capabilities advertised by a model/provider combination.
///
/// Used by `reflect-core` to decide whether to inject cache breakpoints,
/// enable extended thinking, send system blocks, etc.
#[derive(Debug, Clone, Copy, Default)]
pub struct Capabilities {
    /// Supports tool/function calling.
    pub tool_use: bool,
    /// Supports prompt caching (Anthropic `cache_control`).
    pub prompt_caching: bool,
    /// Supports extended thinking (Anthropic thinking blocks).
    pub extended_thinking: bool,
    /// Accepts image input.
    pub vision: bool,
    /// Supports strict JSON-mode output.
    pub json_mode: bool,
    /// Accepts multi-block system prompt.
    pub system_blocks: bool,
}

/// 强类型 provider 标识 —— `ModelClient::provider_kind()` 的返回类型。
///
/// v1.0.0-rc2 引入:替代 `name() -> &str` 的弱字符串比较,让 plugin /
/// TUI / tracing 可在不 unwrap 字符串的前提下做 provider 级分支。
///
/// `Custom` 是默认值 —— plugin / test stub / 未来扩展走这个变体;
/// 3 个内置 client 各自 override 为 `Anthropic` / `OpenAI` / `Ollama`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    Anthropic,
    OpenAI,
    Ollama,
    /// 默认值 —— plugin / test stub / 未来扩展走这个。
    #[default]
    Custom,
}
