//! 配置数据结构。所有字段 `Option<>` / `Default`，未知 TOML 键不报错。
//!
//! 所有 section 派生 `PartialEq, Eq` 以便 `reflect-exec::handle_reload` 的
//! `diff_sections` 比较新旧配置，输出精确的 `ConfigReloaded.sections_changed`
//! 列表（M8 v0 用的是 `vec!["all"]` 占位）。

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 顶层配置 —— 对应 `~/.reflect/config.toml` 的根表。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReflectConfig {
    #[serde(default)]
    pub active: ActiveSection,
    #[serde(default)]
    pub anthropic: Option<AnthropicSection>,
    #[serde(default)]
    pub openai: Option<OpenAISection>,
    /// v0.3.1 新增: Ollama 本地 provider 配置。
    #[serde(default)]
    pub ollama: Option<OllamaSection>,
    #[serde(default)]
    pub compact: CompactSection,
    /// v1.2 P1-12:会话级 token 预算上限。`None` = 不设上限(仅靠
    /// `max_iterations`);`Some` = 累计 token 达 `session_total_tokens`
    /// 后终止当前 turn(`TurnStatus::TokenBudgetExceeded`)。
    #[serde(default)]
    pub token_budget: Option<TokenBudgetSection>,
    /// v1.2 P0-1:OS 沙箱(`[sandbox]` 段)。控制 bash 是否在真实 OS 沙箱内跑。
    #[serde(default)]
    pub sandbox: SandboxSection,
    #[serde(default)]
    pub hooks: HooksSection,
    /// v0.3 新增: MCP server 配置表 `[mcp_servers.<name>]`。
    #[serde(default)]
    pub mcp_servers: McpServersSection,
    /// v0.5 新增: LSP server 配置表 `[lsp_servers.<name>]`。每个
    /// entry 对应一个 stdio language server,`LspConnectionManager`
    /// 启动期并发拉起。
    #[serde(default)]
    pub lsp_servers: LspServersSection,
    /// v1.0.0-rc2 新增: 插件系统配置。`[plugins]` 段描述已启用的
    /// plugin id 与已知的 marketplace 源。
    #[serde(default)]
    pub plugins: PluginsSection,
    /// v1.0 多 Provider 路由:per-role 路由策略段(main / compact /
    /// subagent)。缺省时 builder 用 `RoutingPolicy::default()`,
    /// 所有 slot 退化为仅 primary。
    #[serde(default)]
    pub routing: Option<RoutingSection>,
    /// P3: Subagent 独立 provider 配置(`[subagent_providers]`)。
    /// 可选功能 —— 不配置时 subagent 与主 agent 共享同一 `ModelRegistry`。
    #[serde(default)]
    pub subagent_providers: Option<SubagentProvidersSection>,
    /// v1.1.0 Phase 4:Coordinator 模式配置。`None` = 未启用,
    /// `Some(_)` = 启用,字段控制 system prompt 覆盖与 worker 数上限。
    #[serde(default)]
    pub coordinator: Option<CoordinatorSection>,
    /// v1.0.0-rc2:工具输出密钥脱敏(`[sanitize]` 段)。与
    /// `[hooks]` / `[compact]` 平级,因为这是 content filter 而非
    /// hook —— 用户 / 维护者一眼能在根表里看到。详见
    /// `docs/sanitize.md`。
    #[serde(default)]
    pub sanitize: Option<SanitizeSection>,
    /// v1.1.0 P1 #14:`ask_user_question` 工具配置。控制最大问题数 /
    /// 最大选项数 / 默认超时(秒)。
    #[serde(default)]
    pub ask_user_question: Option<AskUserQuestionSection>,
    /// 模型元数据兜底(`[model]` 段)。当运行时模型不在 `reflect-llm`
    /// 的静态 `CONTEXT_WINDOW_TABLE` / `PRICING_TABLE` 时(典型场景:私
    /// 有 OpenAI 兼容端点暴露的别名,如 `qwen36-1m`),用这里的值兜
    /// 底,让 TUI 的上下文用量条 / 成本段不再「凭空消失」。字段全
    /// `Option`,缺省 `None` = 零行为变化。
    #[serde(default)]
    pub model: Option<ModelSection>,
    /// P3: Feature Flag 运行时开关表 `[feature_flags.flags]`。
    #[serde(default)]
    pub feature_flags: Option<FeatureFlagsSection>,
    /// P3: 分析遥测 stub(`[analytics]`)。
    #[serde(default)]
    pub analytics: Option<AnalyticsSection>,
    /// P2: 通知 webhook(`[notifications]`)。
    #[serde(default)]
    pub notifications: Option<NotificationsSection>,
    /// P2: PostgreSQL 会话持久化(`[postgres_session]`)。
    #[serde(default)]
    pub postgres_session: Option<PostgresSessionSection>,
    /// P2: SSE Redis 回放 stub(`[sse_redis]`)。
    #[serde(default)]
    pub sse_redis: Option<SseRedisSection>,
    /// P3: Bridge 远程控制 stub(`[bridge]`)。
    #[serde(default)]
    pub bridge: Option<BridgeSection>,
    /// P3: 语音服务 stub(`[voice]`)。
    #[serde(default)]
    pub voice: Option<VoiceSection>,
    /// P2: DAP 调试器 stub(`[dap]`)。
    #[serde(default)]
    pub dap: Option<DapSection>,
    /// P2: ACP stub server(`[acp]`)。
    #[serde(default)]
    pub acp: Option<AcpSection>,
    /// Phase 4 收尾(多 Provider 路由):配置 schema 版本号。
    /// `#[serde(default = 1)]` 让旧 TOML(无该字段)自动填 1,
    /// 完全向后兼容。真正的 schema breaking 出现时 bump + 加
    /// `migrate_vN_to_vM` 函数。
    #[serde(default = "default_config_version")]
    pub config_version: u32,
}

/// 当前 schema 版本。Phase 4 引入时为 1;后续 schema breaking
/// bump 此值 + 在 `load_from_str_with_migration` 加新 migration 步骤。
pub const CURRENT_CONFIG_VERSION: u32 = 1;

fn default_config_version() -> u32 {
    1
}

impl Default for ReflectConfig {
    fn default() -> Self {
        Self {
            active: ActiveSection::default(),
            anthropic: None,
            openai: None,
            ollama: None,
            compact: CompactSection::default(),
            token_budget: None,
            sandbox: SandboxSection::default(),
            hooks: HooksSection::default(),
            mcp_servers: McpServersSection::default(),
            lsp_servers: LspServersSection::default(),
            plugins: PluginsSection::default(),
            routing: None,
            subagent_providers: None,
            coordinator: None,
            sanitize: None,
            ask_user_question: None,
            model: None,
            feature_flags: None,
            analytics: None,
            notifications: None,
            postgres_session: None,
            sse_redis: None,
            bridge: None,
            voice: None,
            dap: None,
            acp: None,
            config_version: CURRENT_CONFIG_VERSION,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ActiveSection {
    /// 当前激活的 provider 名(`"anthropic"` / `"openai"`); 缺省时由 env 或第一个非空 provider 决定。
    pub provider: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnthropicSection {
    /// 单值 API key(向后兼容);缺省或 `[[anthropic.credentials]]` 数组
    /// 非空时为 `None`,builder 走 env `ANTHROPIC_API_KEY` 兜底。
    /// v1.0 多 Provider 路由后,本字段语义退化为 "默认 credential 的
    /// 快捷写法";`credentials` 数组与本字段互斥,数组优先。
    #[serde(default)]
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub timeout_secs: Option<u64>,
    /// v1.0 多 Provider 路由:多 credential 列表。`[[anthropic.credentials]]`
    /// 形态 TOML,builder 阶段展开为 `CredentialPool`。
    /// 缺省时 builder 把单值 `api_key` wrap 为 `label = "default"`。
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OpenAISection {
    #[serde(default)]
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

/// Ollama 本地 provider 配置 —— `[ollama]` 段。
///
/// 多数字段可选;若整个段缺失,builder 通过 env `OLLAMA_HOST` /
/// `OLLAMA_API_KEY` 兜底注册。`api_key` 主要用于 Ollama Cloud 或带
/// 认证的反向代理,本地 `ollama serve` 不需要。
///
/// TOML 形态:
/// ```toml
/// [ollama]
/// base_url = "http://192.168.1.5:11434"
/// api_key = "..."
/// model = "qwen2.5:7b"
/// keep_alive_secs = 300
/// num_ctx = 8192
/// num_gpu = 99
/// timeout_secs = 120
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OllamaSection {
    /// Ollama server URL,默认 `http://127.0.0.1:11434`。
    pub base_url: Option<String>,
    /// 可选;Ollama Cloud / 反向代理时填。
    pub api_key: Option<String>,
    /// 默认模型,builder 兜底 `"llama3.2"`。
    pub model: Option<String>,
    /// 模型保留时长(秒)。`Some(0)` 立即卸载、`Some(-1)` 永久保留;
    /// `None` 走 server 默认 5 分钟。序列化为 Ollama `keep_alive`
    /// 字符串(`"300"` / `"-1"` / `"5m"` 等)。
    pub keep_alive_secs: Option<i64>,
    /// 上下文窗口大小(传给 Ollama `options.num_ctx`)。
    pub num_ctx: Option<u32>,
    /// GPU 层数(传给 Ollama `options.num_gpu`)。
    pub num_gpu: Option<u32>,
    /// 单次 HTTP 请求超时;默认 120s(本地首次模型加载可能较慢)。
    pub timeout_secs: Option<u64>,
    /// v1.0 多 Provider 路由:Ollama credential 列表(本地 server
    /// 通常用不到,但 Ollama Cloud 配多账号时支持)。`credentials` 空
    /// 时 builder 把单值 `api_key` wrap 为单条 default credential。
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

/// v1.0 多 Provider 路由:`[[<provider>.credentials]]` 数组里单条
/// 凭证的 schema 镜像。`reflect_llm::Credential` 1:1 字段对应,只多
/// `#[serde(default)]` 字段以便 TOML 缺省时仍能解析。
///
/// 序列化字段:label 必填(供 tracing / TUI 诊断用),其余都可省。
///
/// TOML 形态:
/// ```toml
/// [[anthropic.credentials]]
/// label = "work"
/// api_key = "sk-work-..."
/// weight = 2
/// cooldown_override_secs = 90
///
/// [[anthropic.credentials]]
/// label = "personal"
/// api_key = "sk-personal-..."
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CredentialConfig {
    /// 必填(用户诊断标识);缺省时 builder 兜底 `"env-N"` 或 `"default"`。
    pub label: String,
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// 覆盖 `RoutingPolicy` 全局 cooldown 默认值,单位秒。
    #[serde(default)]
    pub cooldown_override_secs: Option<u64>,
}

fn default_weight() -> u32 {
    1
}

impl CredentialConfig {
    /// 转为 `reflect_llm::Credential`。
    pub fn to_credential(&self) -> reflect_llm::Credential {
        use std::time::Duration;
        reflect_llm::Credential {
            label: self.label.clone(),
            api_key: self.api_key.clone(),
            base_url: self.base_url.clone(),
            weight: if self.weight == 0 { 1 } else { self.weight },
            cooldown_override: self.cooldown_override_secs.map(Duration::from_secs),
        }
    }
}

// ── v1.0 多 Provider 路由:per-role 路由策略段 ─────────────────

/// v1.0 多 Provider 路由:`[routing]` 段。三个 slot(main / compact /
/// subagent)各自独立定义 primary + fallbacks + weights。
///
/// TOML 形态:
/// ```toml
/// [routing]
///
/// [routing.main]
/// primary = "anthropic/claude-3-5-sonnet-latest"
/// fallbacks = ["openai/gpt-4o"]
/// weights = [1]
///
/// [routing.compact]
/// primary = "openai/gpt-4o-mini"
/// fallbacks = ["anthropic/claude-3-haiku"]
///
/// [routing.subagent]
/// primary = "ollama/llama3.2"
/// ```
///
/// 缺省规则:整个 `[routing]` 段缺省 → builder 用
/// `RoutingPolicy::default()`(仅 main.primary = 当前 active spec,
/// 其余 slot 为空);子 slot 缺省(只有部分 slot 写) → 缺失的 slot
/// 也用 `SpecSlotConfig::default()`(仅 primary = active spec)。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RoutingSection {
    #[serde(default)]
    pub main: SpecSlotConfig,
    #[serde(default)]
    pub compact: SpecSlotConfig,
    #[serde(default)]
    pub subagent: SpecSlotConfig,
}

/// 单角色 slot 配置:primary 必填,fallbacks / weights 可选。
/// `weights` 与 `fallbacks` 等长,serde 在 builder 阶段校验。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SpecSlotConfig {
    /// `"provider/model"` 形式 spec,如 `"anthropic/claude-3-5-sonnet-latest"`。
    /// 缺省时 builder 用 ReflectConfig 解析出的 active spec 兜底。
    #[serde(default)]
    pub primary: Option<String>,
    /// fallback spec 列表。`None` = 无 fallback。
    #[serde(default)]
    pub fallbacks: Vec<String>,
    /// 与 `fallbacks` 等长的权重列表。`None` 或长度不匹配时用全 1。
    #[serde(default)]
    pub weights: Vec<u32>,
}

// ── Subagent Providers:子代理独立凭证配置段 ──────────────────────

/// `[subagent_providers]` 段 —— 为 subagent 提供独立的 provider 凭证(base_url + api_key)。
///
/// **可选功能**:当整个 section 缺省时,subagent 与主 agent 共享同一 `ModelRegistry`(默认行为)。
/// 当配置了任意子 provider 时,系统会构建一个独立的 child registry 供 subagent 使用。
///
/// TOML 形态:
/// ```toml
/// [subagent_providers.anthropic]
/// api_key = "sk-subagent-..."
/// base_url = "https://subagent-proxy.example.com"
/// model = "claude-3-haiku"
/// timeout_secs = 60
///
/// [[subagent_providers.anthropic.credentials]]
/// label = "sub-work"
/// api_key = "sk-sub-work-..."
/// weight = 1
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SubagentProvidersSection {
    #[serde(default)]
    pub anthropic: Option<AnthropicSubagentSection>,
    #[serde(default)]
    pub openai: Option<OpenAISubagentSection>,
    #[serde(default)]
    pub ollama: Option<OllamaSubagentSection>,
}

/// Subagent 专用的 Anthropic provider 配置 —— `[subagent_providers.anthropic]`。
///
/// 字段语义与主 agent 的 `AnthropicSection` 一致,但额外支持 `model` 字段
/// (独立于主 agent 的 model)。credentials 数组优先于单值 api_key。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnthropicSubagentSection {
    #[serde(default)]
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    /// Subagent 专用 model(独立于主 agent)。
    /// 若缺省则回退到 `routing.subagent.primary` 中的模型。
    pub model: Option<String>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

/// Subagent 专用的 OpenAI provider 配置 —— `[subagent_providers.openai]`。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OpenAISubagentSection {
    #[serde(default)]
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    /// Subagent 专用 model(独立于主 agent)。
    pub model: Option<String>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

/// Subagent 专用的 Ollama provider 配置 —— `[subagent_providers.ollama]`。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OllamaSubagentSection {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    /// Subagent 专用 model(独立于主 agent)。
    pub model: Option<String>,
    pub keep_alive_secs: Option<i64>,
    pub num_ctx: Option<u32>,
    pub num_gpu: Option<u32>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub credentials: Vec<CredentialConfig>,
}

// ── v1.1.0 Phase 4:Coordinator 模式配置段 ──────────────────────

/// v1.1.0 Phase 4:`[coordinator]` 段镜像 —— 镜像 Claude Code 的
/// `coordinatorMode` 行为契约。
///
/// TOML 形态:
/// ```toml
/// [coordinator]
/// enabled = true                          # 启用(显式);不写 → 走 env var
/// system_prompt_path = "/etc/coordinator.md"   # 覆盖默认 prompt
/// max_workers = 8                          # 覆盖默认 4
/// ```
///
/// `enabled = None` 时回退 `REFLECT_COORDINATOR_MODE` env var;
/// `enabled = Some(false)` 显式关闭,无视 env var。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoordinatorSection {
    /// 显式启用 / 关闭。`None` → 走 env var;`Some(true/false)` → 直接生效。
    #[serde(default)]
    pub enabled: Option<bool>,
    /// 自定义 system prompt 文件路径(`None` 用二进制内嵌默认)。
    /// 文件不存在或为空时 `reflect_task::CoordinatorConfig::from_env_or_config`
    /// `warn` 并回退默认,不让 config 错值让 coordinator 完全瘫痪。
    #[serde(default)]
    pub system_prompt_path: Option<PathBuf>,
    /// 最大并行 worker 数。`None` → 走 env var `REFLECT_COORDINATOR_MAX_WORKERS`
    /// 或默认 4。
    #[serde(default)]
    pub max_workers: Option<u8>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct CompactSection {
    pub trigger_tokens: Option<u32>,
}

/// v1.2 P1-12:Token 预算上限(`[token_budget]` 段)。
///
/// 与 `[compact].trigger_tokens` 不同 —— 后者触发**压缩**(保留上下文、
/// 削减 token),本段触发**终止**(会话 token 累计达上限直接结束当前 turn,
/// `TurnStatus::TokenBudgetExceeded`)。两者正交:压缩是软回收,预算是硬停。
///
/// 优先级:env `REFLECT_TOKEN_BUDGET` > TOML `session_total_tokens` > 无上限。
/// `per_turn_input_tokens`(可选)给单轮输入设上限,留作 follow-up 接入点。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TokenBudgetSection {
    /// 会话级累计 token 硬上限(input + output 累加)。达到即终止后续 model_call。
    /// `None` = 不设上限(仅靠 `max_iterations`)。
    #[serde(default)]
    pub session_total_tokens: Option<u64>,
    /// (可选)单轮输入 token 上限,留作 follow-up 接入;当前不强制。
    #[serde(default)]
    pub per_turn_input_tokens: Option<u32>,
}

/// v1.2 P0-1:OS 沙箱配置(`[sandbox]` 段)。控制 `bash` 工具是否在真实
/// OS 沙箱(macOS Seatbelt / Linux Landlock)内执行。默认关闭(向后兼容;
/// 透传所有命令)。env `REFLECT_SANDBOX_OS_LEVEL` 优先于 `os_level` 字段。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SandboxSection {
    /// 启用 OS 级沙箱。`true` 时 bash 命令在受限 fs 沙箱内跑(workspace +
    /// 临时目录可写,系统目录写一律拒,`rm -rf /` 被内核阻止)。
    #[serde(default)]
    pub os_level: bool,
    /// workspace 之外额外允许写的目录(绝对路径)。
    #[serde(default)]
    pub writable_dirs: Vec<std::path::PathBuf>,
    /// 允许出站网络(默认 false;`cargo` / `git fetch` 等需要时开)。
    #[serde(default)]
    pub allow_network: bool,
}

/// v1.1.0 P1 #14:`ask_user_question` 工具配置。
///
/// `ask_user_question` 让 LLM 主动向用户发起 1-4 道结构化问题(每题
/// 2-4 选项,可选 `multi_select`,可填 "Other" 自定义文本)。本段控制
/// **服务端校验边界**:即便 LLM 试图突破,`ApprovalGate::ask_question`
/// 也会用这些字段校验。TUI 渲染也读 `max_questions` 决定 modal 行数。
///
/// TOML 形态:
/// ```toml
/// [ask_user_question]
/// enabled = true                          # 默认 true;false → LLM 调用此工具报 InvalidArgs
/// max_questions = 4                       # 1-4(超出时 protocol 层硬限制为 4)
/// max_options = 4                         # 1-4(超出时 protocol 层硬限制为 4)
/// default_timeout_secs = 900              # 15 min;0 = 永不超时
/// ```
///
/// 所有字段都是 `Option<_>` —— 缺省值与 `reflect_protocol::question` 模块
/// 的常量(`MAX_QUESTIONS=4` / `MIN/MAX_OPTIONS=2/4` / `MAX_HEADER_CHARS=12`)
/// 对齐。TUI / headless 启动时把本段读出来构造 `AskUserQuestionTool`。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AskUserQuestionSection {
    /// 显式开关。`None` = 默认 `true`(打开)。
    #[serde(default)]
    pub enabled: Option<bool>,
    /// 单次调用最大问题数(1-4)。`None` 走 protocol 硬限制 4。
    #[serde(default)]
    pub max_questions: Option<u8>,
    /// 单题最大选项数(1-4)。`None` 走 protocol 硬限制 4。
    #[serde(default)]
    pub max_options: Option<u8>,
    /// 默认超时(秒)。`0` = 永不超时(等用户主动回执或 cancel)。
    /// `None` 走默认 900 秒(15 分钟,与 Claude Code 对齐)。
    #[serde(default)]
    pub default_timeout_secs: Option<u64>,
}

impl AskUserQuestionSection {
    /// 解析后归一化:`max_questions` / `max_options` 钳到 [1, 4] 范围;
    /// 关闭时返回 `None`(其他字段都忽略)。
    pub fn resolved(&self) -> Option<ResolvedAskUserQuestion> {
        if !self.enabled.unwrap_or(true) {
            return None;
        }
        Some(ResolvedAskUserQuestion {
            max_questions: self.max_questions.unwrap_or(4).clamp(1, 4),
            max_options: self.max_options.unwrap_or(4).clamp(1, 4),
            default_timeout_secs: self.default_timeout_secs.unwrap_or(900),
        })
    }
}

/// 模型元数据兜底段 `[model]`。
///
/// 背景:`reflect-llm` 用两张静态表(`CONTEXT_WINDOW_TABLE` /
/// `PRICING_TABLE`)给 TUI 提供上下文窗口与计价。当运行时模型字符串
/// (经私有 OpenAI 兼容端点的别名,如 `qwen36-1m`、自定义反代名)不在
/// 表内时,上下文用量条与成本段会「凭空消失」(它们各自在 `None` 时返回
/// 零宽 Span)。本段让用户为未知模型自报一个窗口 / 计价,使 metrics 仍
/// 能渲染。
///
/// 全部字段 `Option`,`None` = 沿用静态表 / 优雅省略,零行为变化。
///
/// 计价以 **micro-USD / 1M tokens**(整数)表示:`1` micro-USD = `1e-6`
/// USD。例:`$3.00/Mtok` → `input_price_micro_usd_per_mtok = 3_000_000`。
/// 用整数而非 `f64` 是为了让本段(进而 `ReflectConfig`)能派生 `Eq`,
/// 与其它 `*Section` 保持一致;精度 1e-6 USD 远低于实际计费颗粒度。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ModelSection {
    /// 未知模型兜底的上下文窗口(token)。当引擎报告的 `context_window_size`
    /// 与 `context_window_for` 静态表都返回 `None` 时,TUI 用此值做上下文
    /// 用量条的分母。典型:1M 上下文模型填 `1000000`。
    #[serde(default)]
    pub context_window: Option<u32>,
    /// 输入 token 计价(micro-USD / 1M tokens)。`$3.00/Mtok` → `3_000_000`。
    /// 当 `pricing::price` 对当前模型返回 `None` 时,TUI 用此值 + 输出价
    /// 自行估算每轮成本并累加。`None` = 不自行计价(成本段显示 `$—`)。
    #[serde(default)]
    pub input_price_micro_usd_per_mtok: Option<u64>,
    /// 输出 token 计价(micro-USD / 1M tokens)。`$15.00/Mtok` → `15_000_000`。
    /// 与 `input_price_micro_usd_per_mtok` 配对。`None` 退化为 0(只计输入)。
    #[serde(default)]
    pub output_price_micro_usd_per_mtok: Option<u64>,
}

impl ModelSection {
    /// 把 micro-USD/1Mtok 的整数控件换算成 USD/1Mtok 的 `f64`,供 TUI 与
    /// `pricing` 表的 `f64` 路径对接。`None` → `None`(不自行计价)。
    pub fn input_price_usd_per_mtok(&self) -> Option<f64> {
        self.input_price_micro_usd_per_mtok
            .map(|v| v as f64 / 1_000_000.0)
    }
    /// 输出价同理;`None`(输入价配了但输出价留空)退化为 `0.0`。
    pub fn output_price_usd_per_mtok(&self) -> Option<f64> {
        Some(
            self.output_price_micro_usd_per_mtok
                .unwrap_or(0) as f64
                / 1_000_000.0,
        )
    }
}

/// `AskUserQuestionSection::resolved` 的归一化结果,业务侧直接读字段
/// 不用再处理 `Option<Option<_>>`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedAskUserQuestion {
    pub max_questions: u8,
    pub max_options: u8,
    pub default_timeout_secs: u64,
}

/// P3: Feature Flag 段 —— 运行时 bool 表,缺省视为 false。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FeatureFlagsSection {
    #[serde(default)]
    pub flags: HashMap<String, bool>,
}

impl FeatureFlagsSection {
    /// 查询 flag;未配置 → false。
    pub fn is_enabled(&self, flag: &str) -> bool {
        self.flags.get(flag).copied().unwrap_or(false)
    }
}

/// v1.3: 分析遥测(OTLP)配置。
///
/// `enabled=false` 或 `None` 时,`init_exporter` 返回 `None`,遥测
/// 完全关闭;`enabled=true` 时按 `endpoint` / `service_name` / `headers`
/// / `flush_timeout_ms` 启动 OTLP/HTTP exporter。
///
/// 故意不带 `sample_ratio:f64`,保持 `Eq` 派生以兼容 `ReflectConfig`
/// 的 `Option<AnalyticsSection>` 等值比较。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AnalyticsSection {
    /// 是否启用遥测。默认 `None` 等价 `false`。
    #[serde(default)]
    pub enabled: Option<bool>,
    /// OTLP/HTTP 端点(如 `http://localhost:4318/v1/traces`)。
    #[serde(default)]
    pub endpoint: Option<String>,
    /// OTLP 后端看到的 `service.name`。默认 `"reflect-agent"`。
    #[serde(default)]
    pub service_name: Option<String>,
    /// OTLP header(如认证 token)。
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
    /// 退出时 flush 等待毫秒。默认 `2000`。
    #[serde(default)]
    pub flush_timeout_ms: Option<u64>,
}

/// P2: 通知集成配置。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct NotificationsSection {
    #[serde(default)]
    pub webhook_url: Option<String>,
}

/// P2: PostgreSQL 会话配置(与 `reflect-stream` trait 对齐)。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PostgresSessionSection {
    pub database_url: Option<String>,
    #[serde(default = "default_pg_prefix")]
    pub table_prefix: String,
}

fn default_pg_prefix() -> String {
    "reflect_".into()
}

/// P2: SSE Redis stub 配置。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SseRedisSection {
    pub redis_url: Option<String>,
    #[serde(default = "default_sse_prefix")]
    pub key_prefix: String,
}

fn default_sse_prefix() -> String {
    "reflect:sse:".into()
}

/// P3: Bridge 远程 stub。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BridgeSection {
    pub endpoint: Option<String>,
}

/// P3: 语音服务 stub。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VoiceSection {
    #[serde(default)]
    pub enabled: Option<bool>,
    pub provider: Option<String>,
}

/// P2: DAP 调试器 stub。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DapSection {
    pub adapter: Option<String>,
}

/// ACP stub server 默认监听地址(`:0` 表示由 OS 真实 listen 时随机分配)。
///
/// 单一来源:`reflect-integration::acp` 与 `reflect-tui` 的 `/ide` slash
/// 都引用此处常量,避免默认值漂移。
pub const ACP_DEFAULT_BIND: &str = "127.0.0.1:0";

/// P2: ACP stub server 配置。
///
/// `Default` 与 serde default 都委托到 [`ACP_DEFAULT_BIND`],
/// 保持单一来源,避免 `AcpSection::default()` 给出空 bind 而破坏 `resolve_bind`。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AcpSection {
    #[serde(default = "default_acp_bind")]
    pub bind: String,
}

impl Default for AcpSection {
    fn default() -> Self {
        Self {
            bind: default_acp_bind(),
        }
    }
}

fn default_acp_bind() -> String {
    ACP_DEFAULT_BIND.into()
}

/// v1.0.0-rc2:工具输出密钥脱敏(`[sanitize]` 段)schema 镜像。
///
/// 与 `reflect_tools::sanitize::SanitizeConfig` 是两个独立 struct
/// (字段一一对应),转换在消费者(`reflect-exec`)内联完成 —— 避免
/// `reflect-config → reflect-tools → reflect-hooks → reflect-config`
/// 依赖环。
///
/// TOML 形态:
/// ```toml
/// [sanitize]
/// enabled = true                          # 默认 true
/// marker = "[REDACTED]"                   # 默认 "[REDACTED]"
/// disable_default_patterns = false        # 逃生口,留空 → 跑 10 个默认
/// extra_patterns = [                      # 用户追加,会与默认 OR'd
///   "(?i)\\bmy_custom_token\\s*=\\s*\\S+",
/// ]
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SanitizeSection {
    /// 显式开关。`None` → 默认 `true`(打开)。`Some(false)` 显式关闭。
    pub enabled: Option<bool>,
    /// 覆盖默认 `[REDACTED]` marker。typed pattern(如 AWS)会基于此
    /// 拼成 `[<marker>:aws_key]`。
    pub marker: Option<String>,
    /// `true` 时不加载 10 个默认 pattern,只跑 `extra_patterns`。
    pub disable_default_patterns: Option<bool>,
    /// 用户补充的额外 regex pattern。整段匹配替换为 marker。
    pub extra_patterns: Option<Vec<String>>,
}

/// Hooks 配置 —— 字段是 `reflect-hooks::config::HooksConfig` 的扁平镜像。
/// Phase C 会接入 `HooksConfig::from_reflect_section` 做转换。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct HooksSection {
    /// 启用的 hook 名白名单;`None` = 全部启用。
    pub enabled: Option<Vec<String>>,
    #[serde(default)]
    pub search_budget: Option<SearchBudgetSection>,
    #[serde(default)]
    pub test_runner: Option<TestRunnerSection>,
    #[serde(default)]
    pub plan_completion: Option<PlanCompletionSection>,
    #[serde(default)]
    pub verification: Option<VerificationSection>,
    #[serde(default)]
    pub langfuse_tracker: Option<LangfuseSection>,
    /// `read_before_edit` PreToolUse hook 配置(v1.0.0-rc3+):
    /// 拦截 `write`/`edit`,要求本 session 内 agent 先 `read` 过文件。
    /// 详见 `docs/sanitize.md` 与 `docs/featuresList/04-permission/secret-sanitize.md`（id: `secret-sanitize`）。
    #[serde(default)]
    pub read_before_edit: Option<ReadBeforeEditSection>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SearchBudgetSection {
    pub max_calls: Option<u32>,
    pub search_tools: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TestRunnerSection {
    pub enabled: Option<bool>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PlanCompletionSection {
    pub strict: Option<bool>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VerificationSection {
    pub run_on_stop: Option<bool>,
    pub test_command: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LangfuseSection {
    pub enabled: Option<bool>,
    /// Langfuse HTTP ingestion host,例 `https://cloud.langfuse.com`。
    pub endpoint: Option<String>,
    /// Langfuse public key(`pk-lf-...`)。
    pub public_key: Option<String>,
    /// Langfuse secret key(`sk-lf-...`)。
    pub secret_key: Option<String>,
    /// 导出模式:`tracing`(默认) / `http` / `otlp`(OTLP stub,仅日志)。
    pub export_mode: Option<String>,
}

/// `read_before_edit` hook 配置块。
///
/// TOML 形态:
/// ```toml
/// [hooks.read_before_edit]
/// enabled = true
/// mtime_drift_tolerance_ms = 500
/// ```
///
/// - `enabled` 默认 `true`(master switch);
/// - `mtime_drift_tolerance_ms` 默认 `500` —— 兼容 macOS HFS+ / FAT32
///   2s mtime 粒度(plan R4)。**`0` 表示禁用漂移校验**(只验证文件
///   存在过,不比较 mtime 差值),运维在严格监控文件系统事件或调试
///   hook 时可临时置 0。详见
///   `reflect_hooks::FileReadStateTracker::check_write_safe`。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReadBeforeEditSection {
    /// `None` / `Some(true)` = hook 生效;`Some(false)` = hook 注册但
    /// 永远 Allow(用于 A/B 对比与运维临时关闭)。
    pub enabled: Option<bool>,
    /// mtime 漂移容差,毫秒。默认 `500`;`0` 表示禁用漂移校验(见上)。
    /// 参见 `reflect_hooks::FileReadStateTracker::check_write_safe`。
    pub mtime_drift_tolerance_ms: Option<u64>,
}

// ── MCP (v0.3) ────────────────────────────────────────────────────────────

/// `[mcp_servers.<name>]` 表镜像,key 是 server 名。
///
/// TOML 形态:
/// ```toml
/// [mcp_servers.filesystem]
/// type = "stdio"
/// command = "npx"
/// args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
/// timeout_ms = 30_000
///
/// [mcp_servers.github]
/// type = "streamable-http"
/// url = "https://mcp.example.com/github"
/// headers = { Authorization = "Bearer ghp_xxx" }
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct McpServersSection {
    /// server 名 → entry;空表意味着未配置任何 MCP server。
    #[serde(flatten)]
    pub servers: HashMap<String, McpServerEntry>,
}

/// 单个 MCP server 原始配置(未校验)。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct McpServerEntry {
    /// transport 类型。`"stdio"`(默认)或 `"http"` / `"streamable-http"`。
    #[serde(rename = "type", default)]
    pub transport: McpTransport,
    /// stdio 模式必填。
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    /// stdio 模式:除 `HOME`/`PATH` 之外注入到子进程的 env。
    pub env: Option<HashMap<String, String>>,
    /// http 模式必填。
    pub url: Option<String>,
    /// http 模式自定义 HTTP header;`Authorization` 自动剥离到 `auth_header`。
    pub headers: Option<HashMap<String, String>>,
    /// 单次 tool call 超时(毫秒),默认 30 000 = 30s。
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// v0.3.1 预告字段:是否始终加载到 system prompt。当前 manager 忽略。
    #[serde(default)]
    pub always_load: Option<bool>,
}

/// MCP transport 枚举。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    #[default]
    Stdio,
    /// 别名 `streamable-http` 在 `FromStr`/serde alias 中允许。
    #[serde(alias = "streamable-http")]
    Http,
    /// Legacy MCP HTTP+SSE(GET events + POST messages)。v1+ 与 streamable-http 并存。
    #[serde(alias = "http-sse")]
    Sse,
}

// ── LSP (v0.5) ──────────────────────────────────────────────────────

/// `[lsp_servers.<name>]` 表镜像,key 是 server 名。
///
/// TOML 形态:
/// ```toml
/// [lsp_servers.rust]
/// command = "rust-analyzer"
/// file_patterns = [{ glob = "**/*.rs", language_id = "rust" }]
///
/// [lsp_servers.typescript]
/// command = "typescript-language-server"
/// args = ["--stdio"]
/// file_patterns = [
///     { glob = "**/*.ts",  language_id = "typescript" },
///     { glob = "**/*.tsx", language_id = "typescriptreact" },
/// ]
/// timeout_ms = 30_000
/// root_uri = "file:///workspace"
/// initialization_options = { ... }
/// ```
///
/// 设计要点:每个 entry 持有 glob 模式 + language id,`reflect_lsp::matching`
/// 阶段用预编译 `GlobSet` 决定哪个 server 接管给定文件。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LspServersSection {
    /// server 名 → entry;空表意味着未配置任何 LSP server。
    #[serde(flatten)]
    pub servers: HashMap<String, LspServerEntry>,
}

/// 单个 LSP server 原始配置(未校验)。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LspServerEntry {
    /// stdio 子进程命令(LSP 全部走 stdio,无 HTTP)。
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// 除 `HOME`/`PATH` 之外注入到子进程的 env。
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 文件 glob → LSP `languageId` 映射,决定 server 接管哪些文件。
    /// 至少 1 个,否则 `lsp_server_configs` 校验失败。
    #[serde(default)]
    pub file_patterns: Vec<LspFilePattern>,
    /// 覆盖 LSP `initialize` 的 `rootUri`(默认用 ctx.workspace)。
    #[serde(default)]
    pub root_uri: Option<String>,
    /// 透传给 `initialize` 的 `initializationOptions` JSON 值。
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,
    /// 单次 request 超时(毫秒),默认 30 000 = 30s。
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// 单条文件 glob → languageId 映射。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LspFilePattern {
    /// glob 表达式,相对 workspace root。例如 `**/*.rs`。
    pub glob: String,
    /// LSP `TextDocumentItem.languageId`,例如 `"rust"` / `"typescript"`。
    pub language_id: String,
}

// ── Plugins (v1.0.0-rc2) ───────────────────────────────────────────

/// 单个 plugin 在 `[plugins.marketplaces.<name>]` 下的源定义。
///
/// 对齐 `reflect_plugin::manifest::MarketplaceSource` 的子集 —— 这里
/// 只放用户能在 `~/.reflect/config.toml` 里手写的形态(URL / GitHub /
/// 本地 path),`Npm` / `Git` 留 v1.1 因为需要单独的 `git2` 依赖。
///
/// TOML 形态:
/// ```toml
/// [plugins.marketplaces.official]
/// type = "github"
/// repo = "anthropics/claude-plugins-official"
/// auto_update = true
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PluginMarketplaceConfig {
    /// 源类型 —— `"github"` / `"url"` / `"directory"` / `"file"`。
    #[serde(rename = "type", default)]
    pub kind: PluginMarketplaceKind,
    /// GitHub shorthand:`owner/repo`(`type = "github"` 时用)。
    #[serde(default)]
    pub repo: Option<String>,
    /// HTTP URL(`type = "url"` 时用)指向 marketplace.json。
    #[serde(default)]
    pub url: Option<String>,
    /// 本地路径(`type = "directory"` / `"file"` 时用)。
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// HTTP 拉取时附加 headers(`type = "url"` 时用)。
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// git ref(`type = "github"` 时用,branch / tag / sha)。
    #[serde(default)]
    pub r#ref: Option<String>,
    /// 启动时后台刷新。
    #[serde(default)]
    pub auto_update: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginMarketplaceKind {
    #[default]
    Directory,
    File,
    Github,
    Url,
}

/// `[plugins]` 段 —— 描述已启用的 plugin 与已知的 marketplace 源。
///
/// 设计要点:
/// - **`enabled_plugins` 只记录 plugin id 字符串**(与
///   `~/.reflect/plugins/installed_plugins.json` 互补 —— 后者描述
///   "装了什么",前者描述 "用什么")。`PluginManager` 在 boot / reload
///   时以 `installed_plugins.json` 为准查 `enabled_plugins` 是否启用。
/// - **marketplaces 字段是用户可见的配置入口**;安装记录
///   (`known_marketplaces.json`) 由 `PluginManager` 维护,不在
///   `ReflectConfig` 内(避免双向同步)。
///
/// TOML 形态:
/// ```toml
/// [plugins]
/// enabled_plugins = ["code-formatter@anthropic-tools", "local-plugin@inline"]
///
/// [plugins.marketplaces.official]
/// type = "github"
/// repo = "anthropics/claude-plugins-official"
/// auto_update = true
///
/// [plugins.marketplaces.local-dev]
/// type = "directory"
/// path = "/Users/me/dev/my-marketplace"
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PluginsSection {
    /// 已启用的 plugin id 列表。
    #[serde(default)]
    pub enabled_plugins: Vec<String>,
    /// 用户配置的 marketplace 源。
    #[serde(default)]
    pub marketplaces: HashMap<String, PluginMarketplaceConfig>,
}

impl PluginsSection {
    /// 检查某个 plugin id 是否启用。
    pub fn is_enabled(&self, plugin_id: &str) -> bool {
        self.enabled_plugins.iter().any(|p| p == plugin_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 解析基础 ──────────────────────────────────────────────────────────

    /// Phase 2:`[hooks]` 块空 → `read_before_edit = None`(默认行为)。
    #[test]
    fn hooks_section_without_read_before_edit_is_none() {
        let cfg: ReflectConfig = toml::from_str("[hooks]\nenabled = []\n").unwrap();
        assert!(cfg.hooks.read_before_edit.is_none());
    }

    /// Phase 2:`[hooks.read_before_edit]` 显式块解析为完整 section。
    #[test]
    fn hooks_section_parses_read_before_edit_with_values() {
        let cfg: ReflectConfig = toml::from_str(
            r#"
            [hooks.read_before_edit]
            enabled = true
            mtime_drift_tolerance_ms = 1000
            "#,
        )
        .unwrap();
        let rbe = cfg
            .hooks
            .read_before_edit
            .expect("read_before_edit section parsed");
        assert_eq!(rbe.enabled, Some(true));
        assert_eq!(rbe.mtime_drift_tolerance_ms, Some(1000));
    }

    /// Phase 2:`ReadBeforeEditSection` 独立构造 + serde round-trip。
    #[test]
    fn read_before_edit_section_serde_roundtrip() {
        let s = ReadBeforeEditSection {
            enabled: Some(false),
            mtime_drift_tolerance_ms: Some(250),
        };
        let j = serde_json::to_string(&s).unwrap();
        let back: ReadBeforeEditSection = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn parses_stdio_mcp_server_entry() {
        let toml = r#"
            [mcp_servers.filesystem]
            type = "stdio"
            command = "npx"
            args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
            timeout_ms = 30000
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let entry = cfg.mcp_servers.servers.get("filesystem").expect("entry");
        assert_eq!(entry.transport, McpTransport::Stdio);
        assert_eq!(entry.command.as_deref(), Some("npx"));
        assert_eq!(
            entry.args.as_deref(),
            Some(
                [
                    "-y".to_string(),
                    "@modelcontextprotocol/server-filesystem".to_string(),
                    "/tmp".to_string(),
                ]
                .as_slice()
            )
        );
        assert_eq!(entry.timeout_ms, Some(30_000));
    }

    #[test]
    fn parses_http_mcp_server_entry_with_alias() {
        let toml = r#"
            [mcp_servers.github]
            type = "streamable-http"
            url = "https://mcp.example.com/github"
            headers = { Authorization = "Bearer xyz" }
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let entry = cfg.mcp_servers.servers.get("github").expect("entry");
        assert_eq!(entry.transport, McpTransport::Http);
        assert_eq!(entry.url.as_deref(), Some("https://mcp.example.com/github"));
        assert_eq!(
            entry
                .headers
                .as_ref()
                .and_then(|h| h.get("Authorization"))
                .map(String::as_str),
            Some("Bearer xyz")
        );
    }

    #[test]
    fn transport_defaults_to_stdio_when_omitted() {
        let toml = r#"
            [mcp_servers.minimal]
            command = "echo"
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let entry = cfg.mcp_servers.servers.get("minimal").unwrap();
        assert_eq!(entry.transport, McpTransport::Stdio);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        // Reflect 的容忍策略:未知字段不报错,留 v0.4 加 strict 模式。
        let toml = r#"
            [mcp_servers.x]
            command = "echo"
            future_field = "ignored"
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        assert!(cfg.mcp_servers.servers.contains_key("x"));
    }

    // ── PartialEq 比较能力 (给 diff_sections 用) ────────────────────────

    #[test]
    fn partial_eq_detects_added_server() {
        let old = ReflectConfig::default();
        let new: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.github]
            type = "http"
            url = "https://x"
        "#,
        )
        .unwrap();
        assert_ne!(old.mcp_servers, new.mcp_servers);
    }

    #[test]
    fn partial_eq_detects_removed_server() {
        let old: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.x]
            command = "echo"
        "#,
        )
        .unwrap();
        let new = ReflectConfig::default();
        assert_ne!(old.mcp_servers, new.mcp_servers);
    }

    #[test]
    fn partial_eq_detects_command_field_change() {
        let a: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.x]
            command = "echo"
        "#,
        )
        .unwrap();
        let b: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.x]
            command = "cat"
        "#,
        )
        .unwrap();
        assert_ne!(a.mcp_servers, b.mcp_servers);
    }

    #[test]
    fn partial_eq_detects_timeout_change() {
        // timeout 变化本身不触发 restart,但仍属 config change (reload 路径会
        // diff_sections 列出 "mcp_servers")。此测试锁定 partial_eq 语义。
        let a: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.x]
            command = "echo"
            timeout_ms = 30000
        "#,
        )
        .unwrap();
        let b: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.x]
            command = "echo"
            timeout_ms = 60000
        "#,
        )
        .unwrap();
        assert_ne!(a.mcp_servers, b.mcp_servers);
    }

    // ── Ollama (v0.3.1) ─────────────────────────────────────────────────

    #[test]
    fn parses_full_ollama_section() {
        let toml = r#"
            [ollama]
            base_url = "http://192.168.1.5:11434"
            api_key = "sk-local"
            model = "qwen2.5:7b"
            keep_alive_secs = 300
            num_ctx = 8192
            num_gpu = 99
            timeout_secs = 120
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let o = cfg.ollama.as_ref().expect("ollama section");
        assert_eq!(o.base_url.as_deref(), Some("http://192.168.1.5:11434"));
        assert_eq!(o.api_key.as_deref(), Some("sk-local"));
        assert_eq!(o.model.as_deref(), Some("qwen2.5:7b"));
        assert_eq!(o.keep_alive_secs, Some(300));
        assert_eq!(o.num_ctx, Some(8192));
        assert_eq!(o.num_gpu, Some(99));
        assert_eq!(o.timeout_secs, Some(120));
    }

    #[test]
    fn parses_minimal_ollama_section_with_all_none_fields() {
        // 仅声明 `[ollama]`,所有字段都缺省 → 全 None,builder 走兜底。
        let toml = r#"
            [ollama]
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let o = cfg.ollama.as_ref().expect("ollama section");
        assert!(o.base_url.is_none());
        assert!(o.api_key.is_none());
        assert!(o.model.is_none());
        assert!(o.keep_alive_secs.is_none());
        assert!(o.num_ctx.is_none());
        assert!(o.num_gpu.is_none());
        assert!(o.timeout_secs.is_none());
    }

    #[test]
    fn partial_eq_detects_ollama_section_change() {
        // 改 keep_alive_secs → partial_eq 不等 → diff_sections 报 "ollama"。
        let a: ReflectConfig = toml::from_str(
            r#"
            [ollama]
            model = "llama3.2"
            keep_alive_secs = 300
            "#,
        )
        .unwrap();
        let mut b = a.clone();
        b.ollama.as_mut().unwrap().keep_alive_secs = Some(600);
        assert_ne!(a.ollama, b.ollama);
    }

    #[test]
    fn unknown_ollama_keys_ignored() {
        // 容忍策略:未知字段不报错(v0.4 strict 模式留待)。
        let toml = r#"
            [ollama]
            model = "llama3.2"
            future_field = "ignored"
            "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        assert!(cfg.ollama.is_some());
    }

    #[test]
    fn partial_eq_unchanged_when_only_other_section_changes() {
        let a: ReflectConfig = toml::from_str(
            r#"
            [mcp_servers.x]
            command = "echo"
            [compact]
            trigger_tokens = 10000
        "#,
        )
        .unwrap();
        let mut b = a.clone();
        b.compact.trigger_tokens = Some(20000);
        assert_eq!(a.mcp_servers, b.mcp_servers);
    }

    // ── v1.2 P1-12: token_budget section ──────────────────────────────

    #[test]
    fn token_budget_section_parses_session_total() {
        let cfg: ReflectConfig = toml::from_str(
            r#"
            [token_budget]
            session_total_tokens = 500000
        "#,
        )
        .unwrap();
        assert_eq!(
            cfg.token_budget.as_ref().unwrap().session_total_tokens,
            Some(500_000)
        );
        // per_turn_input_tokens 未给 → None(可选)。
        assert_eq!(cfg.token_budget.as_ref().unwrap().per_turn_input_tokens, None);
    }

    #[test]
    fn token_budget_section_defaults_to_none_when_absent() {
        let cfg: ReflectConfig = toml::from_str("").unwrap();
        assert!(cfg.token_budget.is_none(), "absent section → None");
    }

    #[test]
    fn token_budget_partial_eq_detects_change() {
        let a: ReflectConfig = toml::from_str(
            r#"
            [token_budget]
            session_total_tokens = 100000
        "#,
        )
        .unwrap();
        let mut b = a.clone();
        assert_eq!(a.token_budget, b.token_budget);
        b.token_budget.as_mut().unwrap().session_total_tokens = Some(200_000);
        assert_ne!(a.token_budget, b.token_budget, "change must be detected");
    }

    // ── v1.0.0-rc2: plugins section ──────────────────────────────────

    #[test]
    fn plugins_section_defaults_to_empty() {
        let cfg = ReflectConfig::default();
        assert!(cfg.plugins.enabled_plugins.is_empty());
        assert!(cfg.plugins.marketplaces.is_empty());
    }

    #[test]
    fn plugins_section_parses_enabled_and_marketplaces() {
        let toml = r#"
            [plugins]
            enabled_plugins = ["code-formatter@anthropic-tools", "local@inline"]

            [plugins.marketplaces.official]
            type = "github"
            repo = "anthropics/claude-plugins-official"
            auto_update = true

            [plugins.marketplaces.local-dev]
            type = "directory"
            path = "/Users/me/dev/marketplace"
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.plugins.enabled_plugins.len(), 2);
        assert!(cfg.plugins.is_enabled("code-formatter@anthropic-tools"));
        assert!(cfg.plugins.is_enabled("local@inline"));
        assert!(!cfg.plugins.is_enabled("ghost@nowhere"));
        let official = cfg.plugins.marketplaces.get("official").unwrap();
        assert_eq!(official.kind, PluginMarketplaceKind::Github);
        assert_eq!(
            official.repo.as_deref(),
            Some("anthropics/claude-plugins-official")
        );
        assert!(official.auto_update);
        let local = cfg.plugins.marketplaces.get("local-dev").unwrap();
        assert_eq!(local.kind, PluginMarketplaceKind::Directory);
        assert_eq!(
            local.path.as_deref(),
            Some(std::path::Path::new("/Users/me/dev/marketplace"))
        );
    }

    #[test]
    fn plugins_partial_eq_detects_changes() {
        let mut a = ReflectConfig::default();
        a.plugins.enabled_plugins.push("foo@bar".into());
        let mut b = ReflectConfig::default();
        b.plugins.enabled_plugins.push("baz@qux".into());
        assert_ne!(a.plugins, b.plugins);
    }

    #[test]
    fn plugins_partial_eq_unchanged_when_only_other_section_changes() {
        let mut a = ReflectConfig::default();
        a.plugins.enabled_plugins.push("foo@bar".into());
        let mut b = a.clone();
        b.compact.trigger_tokens = Some(20000);
        assert_eq!(a.plugins, b.plugins);
    }

    #[test]
    fn plugins_marketplace_kind_defaults_to_directory() {
        let toml = r#"
            [plugins.marketplaces.defaults]
            path = "/tmp/m"
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let m = cfg.plugins.marketplaces.get("defaults").unwrap();
        assert_eq!(m.kind, PluginMarketplaceKind::Directory);
        assert_eq!(m.path.as_deref(), Some(std::path::Path::new("/tmp/m")));
    }

    // ── v1.1.0 Phase 4: coordinator section ─────────────────────────

    /// `[coordinator]` 段默认 None,可通过 TOML 启用。
    #[test]
    fn coordinator_section_defaults_to_none() {
        let cfg = ReflectConfig::default();
        assert!(cfg.coordinator.is_none());
    }

    /// 解析 `[coordinator]` 段到 `Some(CoordinatorSection)`。
    #[test]
    fn parses_coordinator_section() {
        let toml = r#"
            [coordinator]
            enabled = true
            system_prompt_path = "/etc/coordinator.md"
            max_workers = 8
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let c = cfg.coordinator.expect("coordinator section");
        assert_eq!(c.enabled, Some(true));
        assert_eq!(
            c.system_prompt_path.as_deref(),
            Some(std::path::Path::new("/etc/coordinator.md"))
        );
        assert_eq!(c.max_workers, Some(8));
    }

    /// 旧 TOML 无 `[coordinator]` → None,保留向后兼容。
    #[test]
    fn old_toml_without_coordinator_loads() {
        let cfg: ReflectConfig = toml::from_str(
            r#"
            [active]
            provider = "anthropic"
        "#,
        )
        .unwrap();
        assert!(cfg.coordinator.is_none());
    }

    /// partial_eq 检测 `[coordinator]` 段变化。
    #[test]
    fn coordinator_partial_eq_detects_change() {
        let a = ReflectConfig::default();
        let b = ReflectConfig {
            coordinator: Some(CoordinatorSection {
                enabled: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_ne!(a.coordinator, b.coordinator);
    }

    // ── v1.0.0-rc2: sanitize section ─────────────────────────────

    /// 缺省时 `sanitize` 为 `None`,与 `enabled = true`(运行时默认)一致。
    #[test]
    fn sanitize_section_defaults_to_none() {
        let cfg = ReflectConfig::default();
        assert!(cfg.sanitize.is_none());
    }

    /// 解析完整 `[sanitize]` 段,所有字段都正确读取。
    #[test]
    fn parses_full_sanitize_section() {
        let toml = r#"
            [sanitize]
            enabled = true
            marker = "[HIDDEN]"
            disable_default_patterns = false
            extra_patterns = [
                "(?i)\\bmy_token\\s*=\\s*\\S+",
                "(?i)\\binternal_key\\b",
            ]
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let s = cfg.sanitize.expect("sanitize section");
        assert_eq!(s.enabled, Some(true));
        assert_eq!(s.marker.as_deref(), Some("[HIDDEN]"));
        assert_eq!(s.disable_default_patterns, Some(false));
        let extras = s.extra_patterns.expect("extra_patterns");
        assert_eq!(extras.len(), 2);
        assert!(extras[0].contains("my_token"));
        assert!(extras[1].contains("internal_key"));
    }

    /// 仅声明 `[sanitize]` 空表,字段全部为 `None`。
    #[test]
    fn parses_minimal_sanitize_section_with_all_none_fields() {
        let toml = r#"
            [sanitize]
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let s = cfg.sanitize.expect("sanitize section");
        assert!(s.enabled.is_none());
        assert!(s.marker.is_none());
        assert!(s.disable_default_patterns.is_none());
        assert!(s.extra_patterns.is_none());
    }

    /// 旧 TOML(无 `[sanitize]` 段)依然能解析,向后兼容。
    #[test]
    fn old_toml_without_sanitize_loads() {
        let cfg: ReflectConfig = toml::from_str(
            r#"
            [active]
            provider = "anthropic"
        "#,
        )
        .unwrap();
        assert!(cfg.sanitize.is_none());
    }

    /// TOML round-trip:序列化后再反序列化,字段保持一致。
    #[test]
    fn sanitize_section_round_trip() {
        let toml = r#"
            [sanitize]
            enabled = false
            marker = "[X]"
            extra_patterns = ["(?i)foo"]
        "#;
        let cfg: ReflectConfig = toml::from_str(toml).unwrap();
        let serialized = toml::to_string(&cfg).unwrap();
        let cfg2: ReflectConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(cfg.sanitize, cfg2.sanitize);
    }

    /// partial_eq 检测 `SanitizeSection` 字段变化。
    #[test]
    fn sanitize_partial_eq_detects_change() {
        let a = SanitizeSection {
            enabled: Some(true),
            ..Default::default()
        };
        let b = SanitizeSection {
            enabled: Some(false),
            ..Default::default()
        };
        assert_ne!(a, b);
    }

    /// partial_eq 在 marker 变更时也检测得到。
    #[test]
    fn sanitize_partial_eq_detects_marker_change() {
        let a = SanitizeSection {
            marker: Some("[A]".into()),
            ..Default::default()
        };
        let b = SanitizeSection {
            marker: Some("[B]".into()),
            ..Default::default()
        };
        assert_ne!(a, b);
    }

    /// ReflectConfig 顶层 partial_eq 在 sanitize 变化时也检测得到。
    #[test]
    fn reflect_config_partial_eq_detects_sanitize_change() {
        let a = ReflectConfig::default();
        let mut b = a.clone();
        b.sanitize = Some(SanitizeSection {
            enabled: Some(false),
            ..Default::default()
        });
        assert_ne!(a, b);
    }

    // ── [model] 段(未知模型 metrics 兜底) ───────────────────────────────

    /// `[model]` 段缺省 → `None`(零行为变化)。
    #[test]
    fn model_section_absent_by_default() {
        let cfg: ReflectConfig = toml::from_str("[active]\nprovider = \"anthropic\"\n").unwrap();
        assert!(cfg.model.is_none());
    }

    /// `[model]` 段解析 context_window + micro-USD 计价。
    #[test]
    fn model_section_parses_context_window_and_pricing() {
        let cfg: ReflectConfig = toml::from_str(
            r#"
            [model]
            context_window = 1000000
            input_price_micro_usd_per_mtok = 3000000
            output_price_micro_usd_per_mtok = 15000000
            "#,
        )
        .unwrap();
        let m = cfg.model.expect("[model] section parsed");
        assert_eq!(m.context_window, Some(1_000_000));
        assert_eq!(m.input_price_micro_usd_per_mtok, Some(3_000_000));
        assert_eq!(m.output_price_micro_usd_per_mtok, Some(15_000_000));
        // 换算 helper:$3.00/Mtok 输入,$15.00/Mtok 输出。
        assert!((m.input_price_usd_per_mtok().unwrap() - 3.0).abs() < 1e-9);
        assert!((m.output_price_usd_per_mtok().unwrap() - 15.0).abs() < 1e-9);
    }

    /// 只配 context_window 不配价格:input_price → None(不自行计价)。
    #[test]
    fn model_section_context_window_only() {
        let cfg: ReflectConfig =
            toml::from_str("[model]\ncontext_window = 256000\n").unwrap();
        let m = cfg.model.expect("[model] section parsed");
        assert_eq!(m.context_window, Some(256_000));
        assert!(m.input_price_usd_per_mtok().is_none());
    }
}
