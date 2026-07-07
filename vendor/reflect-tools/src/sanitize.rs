//! 工具输出密钥脱敏 —— 工具输出进入 LLM 上下文前的唯一脱敏节点。
//!
//! ## 契约
//!
//! 本模块在 [`crate::queue::ToolExecutionQueue::execute_single`] 的
//! `Ok(Ok(output))` 分支、`tool.execute(...)` 返回之后、`PostToolUse`
//! 钩子派发之前被调用,把 [`reflect_protocol::ToolOutput`] 内每个
//! [`ContentBlock`] 的可文本字段按既定 pattern 替换为脱敏 marker。
//!
//! 设计要点:
//!
//! - **位置唯一**:PostToolUse 钩子的 [`HookDecision`](reflect_hooks::HookDecision)
//!   无法 mutate `result`(详见 `reflect-hooks/src/decision.rs`),
//!   所以脱敏必须做在 queue 层,不能委托给 hook。
//! - **PostToolUse 钩子只看到脱敏后版本**:LangfuseTracker 等 telemetry
//!   钩子不会泄露原始密钥到 tracing span / rollout 落盘。
//! - **错误路径不二次扫描**:`Ok(Err(_))` 与 timeout 分支合成的
//!   `ContentBlock::text("...")` 是工具错误模板,不携带真实密钥,
//!   重复扫描会引入 false positive 与 CPU 浪费。
//! - **正则无回溯爆炸风险**:所有 pattern 的量词均有字面量上界(如
//!   `{16}` / `{20,}`);`PRIVATE_KEY_BLOCK` 用 non-greedy `+?` 以
//!   `-----END ... PRIVATE KEY-----` 字面量收尾,无 catastrophic
//!   backtracking。
//!
//! ## 默认 pattern
//!
//! 详见 [`default_patterns`],目前覆盖 10 类常见密钥格式:
//!
//! 1. `KEY_ASSIGN` —— `KEY=value` / `password: xxx` 类赋值
//! 2. `BEARER_TOKEN` —— HTTP `Authorization: Bearer xxx`
//! 3. `AWS_ACCESS_KEY` —— `AKIA*` / `ASIA*` 16-char 标识
//! 4. `OPENAI_KEY` —— `sk-...` 20+ 字符
//! 5. `GITHUB_TOKEN` —— `ghp_*` / `gho_*` / `ghs_*` / `ghr_*` / `ghu_*`
//! 6. `ANTHROPIC_KEY` —— `sk-ant-...` 20+ 字符
//! 7. `PRIVATE_KEY_BLOCK` —— `-----BEGIN ... PRIVATE KEY-----` 整块
//! 8. `JWT` —— `eyJ*.eyJ*.*` 三段 base64url
//! 9. `DB_URL` —— `postgres://...` / `mongodb://...` 等连接串
//! 10. `SLACK_TOKEN` —— `xox[abprs]-...`
//!
//! ## 幂等性
//!
//! 默认 pattern 的值字符类显式排除 `[` 与 `]`,因此 `[REDACTED]`
//! marker 不会触发再次匹配,二次调用结果与一次一致。
//! 用户通过 `extra_patterns` 提供的自定义 pattern 不保证此性质,
//! 文档提醒用户自行测试。

use std::sync::Arc;

use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use reflect_protocol::{ContentBlock, ToolOutput};

/// 默认脱敏 marker。出现在脱敏后的文本中,模型应识别为「此处曾有密钥」。
///
/// 自定义 marker 时务必保证 marker 文本不会再次命中任何 pattern ——
/// 例如 marker 含 `[` / `]` 或其他在 pattern 值字符类中被排除的字符。
pub const DEFAULT_MARKER: &str = "[REDACTED]";

/// 工具输出脱敏器。内部状态用 [`Arc`] 共享,克隆廉价。
///
/// 通过 [`Sanitizer::with_defaults`] / [`Sanitizer::from_config`] /
/// [`Sanitizer::disabled`] 三种构造方式。
#[derive(Debug, Clone)]
pub struct Sanitizer {
    inner: Arc<SanitizerInner>,
}

#[derive(Debug)]
struct SanitizerInner {
    /// 编译后的 pattern 列表。空向量表示「no-op」(`disabled()` 情形)。
    patterns: Vec<CompiledPattern>,
    marker: String,
    /// 用户是否提供 `extra_patterns`。short-circuit 启发式只看默认 pattern
    /// 的 trigger 字面量;若有 extras,保守起见不走 short-circuit。
    has_extras: bool,
}

impl Sanitizer {
    /// 用全部 10 个默认 pattern + 默认 marker 构造。`enabled = true`。
    pub fn with_defaults() -> Self {
        Self {
            inner: Arc::new(SanitizerInner {
                patterns: default_patterns(),
                marker: DEFAULT_MARKER.to_string(),
                has_extras: false,
            }),
        }
    }

    /// 完全关闭脱敏,任何 [`sanitize_text`] 调用直接返回输入副本。
    pub fn disabled() -> Self {
        Self {
            inner: Arc::new(SanitizerInner {
                patterns: Vec::new(),
                marker: DEFAULT_MARKER.to_string(),
                has_extras: false,
            }),
        }
    }

    /// 从 [`SanitizeConfig`] 构造。用户配置无效时返回错误。
    pub fn from_config(cfg: &SanitizeConfig) -> Result<Self, SanitizeError> {
        // `enabled = false` 走短路。
        if cfg.enabled == Some(false) {
            return Ok(Self::disabled());
        }

        let marker = cfg
            .marker
            .clone()
            .unwrap_or_else(|| DEFAULT_MARKER.to_string());

        // 按 `disable_default_patterns` 与 `extra_patterns` 决定 pattern 列表。
        let mut patterns: Vec<CompiledPattern> = if cfg.disable_default_patterns != Some(true) {
            default_patterns()
        } else {
            Vec::new()
        };

        if let Some(extras) = &cfg.extra_patterns {
            for (i, raw) in extras.iter().enumerate() {
                let regex = Regex::new(raw).map_err(|e| SanitizeError::InvalidPattern {
                    index: i,
                    pattern_source: raw.clone(),
                    message: e.to_string(),
                })?;
                patterns.push(CompiledPattern {
                    id: "<extra>",
                    regex,
                    // 默认额外 pattern 把整段匹配直接替换为 marker。
                    replace: replace_whole,
                });
            }
        }

        Ok(Self {
            inner: Arc::new(SanitizerInner {
                patterns,
                marker,
                has_extras: cfg.extra_patterns.as_ref().is_some_and(|v| !v.is_empty()),
            }),
        })
    }

    /// 当前是否启用(有 pattern 可跑)。`disabled()` 返回 `false`。
    pub fn is_enabled(&self) -> bool {
        !self.inner.patterns.is_empty()
    }

    /// 当前 marker 文本。
    pub fn marker(&self) -> &str {
        &self.inner.marker
    }

    /// 已注册的 pattern 数量(供 `/hooks ls` 与 `docs/sanitize.md` 展示)。
    pub fn pattern_count(&self) -> usize {
        self.inner.patterns.len()
    }
}

/// 配置镜像,对应 TOML `[sanitize]` 段。所有字段 `Option` —— 缺省即默认。
///
/// 详见 `docs/sanitize.md` 与 `reflect-config/src/schema.rs::SanitizeSection`。
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SanitizeConfig {
    /// `false` 显式关闭脱敏(等同 [`Sanitizer::disabled`])。
    pub enabled: Option<bool>,
    /// 覆盖默认 `[REDACTED]` marker。
    pub marker: Option<String>,
    /// `true` 时不加载 10 个默认 pattern,只跑 `extra_patterns`。
    pub disable_default_patterns: Option<bool>,
    /// 用户补充的额外 pattern。整段匹配被替换为 marker,无 capture group 语义。
    pub extra_patterns: Option<Vec<String>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SanitizeError {
    /// 用户 `extra_patterns[i]` 不是合法 regex。
    #[error("invalid extra_patterns[{index}]: {pattern_source:?} ({message})")]
    InvalidPattern {
        index: usize,
        /// 触发错误的原始正则字符串(命名避开 `source`,免得被 thiserror
        /// 当作 `#[source]` 处理)。
        pattern_source: String,
        /// `regex::Error` 的人类可读消息。
        message: String,
    },
}

/// 单个编译后的 pattern + 替换策略。
#[derive(Debug)]
pub struct CompiledPattern {
    /// 调试 / 日志用稳定 id(`"KEY_ASSIGN"` / `"AWS_ACCESS_KEY"` 等)。
    #[allow(dead_code)]
    id: &'static str,
    regex: Regex,
    /// `(&Captures, marker) -> String` —— 用 capture group 拼装替换文本。
    /// marker 作为第二参数传入,避免每个 pattern 持有 marker 副本。
    replace: fn(&Captures, marker: &str) -> String,
}

// ── 默认 pattern 集合 ────────────────────────────────────────────
//
// 注意:值字符类显式排除 `[` 与 `]`,确保默认 marker `[REDACTED]` 不
// 会触发再次匹配,达到二次调用幂等。

/// 整段匹配直接替换为 marker。给用户 `extra_patterns` 用。
fn replace_whole(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    marker.to_string()
}

/// `KEY=value` / `password: "xxx"` 赋值 —— 帮助函数(当前未被
/// `default_patterns` 直接使用,KEY_ASSIGN 用的是 inline 闭包以保留
/// 前缀字符;保留 helper 以便未来复用)。
#[allow(dead_code)]
fn replace_key_assign(caps: &Captures, marker: &str) -> String {
    let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    let sep = caps.get(2).map(|m| m.as_str()).unwrap_or("=");
    format!("{key}{sep}{marker}")
}

/// HTTP `Authorization: Bearer xxx`。保留 `Bearer ` 前缀,替换 token。
fn replace_bearer(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    format!("Bearer {marker}")
}

/// 把 `[REDACTED]` 类带括号 marker 转成 `[REDACTED:<type>]` 形式。
///
/// 用户自定义 marker `[HIDDEN]` 会得到 `[HIDDEN:aws_key]`。
/// 无括号的 marker(如 `REDACTED`)则得到 `[REDACTED:aws_key]`。
fn typed_marker(marker: &str, type_id: &str) -> String {
    let inner = marker.strip_prefix('[').unwrap_or(marker);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    format!("[{inner}:{type_id}]")
}

/// AWS access key。直接整段替换,带类型后缀。
fn replace_aws(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "aws_key")
}

/// OpenAI API key。
fn replace_openai(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "openai_key")
}

/// GitHub token。
fn replace_github(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "github_token")
}

/// Anthropic API key。
fn replace_anthropic(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "anthropic_key")
}

/// PEM 私钥整块。
fn replace_private_key(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "private_key")
}

/// JWT。
fn replace_jwt(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "jwt")
}

/// 数据库连接串。保留 scheme 前缀,替换后续整段(含 user:pass@host)。
fn replace_db_url(caps: &Captures, marker: &str) -> String {
    let scheme = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    format!("{scheme}{marker}")
}

/// Slack token。
fn replace_slack(caps: &Captures, marker: &str) -> String {
    let _ = caps;
    typed_marker(marker, "slack_token")
}

/// 构造全部 10 个默认 pattern。每个的 regex 都经过手工审查,确认无回溯爆炸。
///
/// 顺序关键:`KEY_ASSIGN` 必须放在所有具体 provider pattern 之后,
/// 否则 `AWS_ACCESS_KEY_ID=AKIA...` 会被 `KEY_ASSIGN` 整段吞掉,
/// 后续的 `AWS_ACCESS_KEY` 看不到原始 key。下方排列即此顺序。
pub fn default_patterns() -> Vec<CompiledPattern> {
    vec![
        // ── 具体 provider key pattern(必须先于 KEY_ASSIGN) ──────
        CompiledPattern {
            id: "AWS_ACCESS_KEY",
            regex: Regex::new(r"\b(AKIA|ASIA)[A-Z0-9]{16}\b")
                .expect("AWS_ACCESS_KEY regex must compile"),
            replace: replace_aws,
        },
        // ANTHROPIC_KEY 必须先于 OPENAI_KEY —— `sk-ant-...` 是 `sk-...`
        // 的子集,否则 OPENAI 会先抢跑把整段替换成 `[REDACTED:openai_key]`。
        CompiledPattern {
            id: "ANTHROPIC_KEY",
            regex: Regex::new(r"\bsk-ant-[A-Za-z0-9_\-]{20,}\b")
                .expect("ANTHROPIC_KEY regex must compile"),
            replace: replace_anthropic,
        },
        CompiledPattern {
            id: "OPENAI_KEY",
            regex: Regex::new(r"\bsk-[A-Za-z0-9_\-]{20,}\b")
                .expect("OPENAI_KEY regex must compile"),
            replace: replace_openai,
        },
        CompiledPattern {
            id: "GITHUB_TOKEN",
            regex: Regex::new(r"\b(ghp|gho|ghs|ghr|ghu)_[A-Za-z0-9]{30,}\b")
                .expect("GITHUB_TOKEN regex must compile"),
            replace: replace_github,
        },
        CompiledPattern {
            id: "PRIVATE_KEY_BLOCK",
            regex: Regex::new(
                r"-----BEGIN (RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY-----[\s\S]+?-----END (RSA |EC |DSA |OPENSSH |PGP |ENCRYPTED )?PRIVATE KEY-----",
            )
            .expect("PRIVATE_KEY_BLOCK regex must compile"),
            replace: replace_private_key,
        },
        CompiledPattern {
            id: "JWT",
            regex: Regex::new(r"\beyJ[A-Za-z0-9_\-]+\.eyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+")
                .expect("JWT regex must compile"),
            replace: replace_jwt,
        },
        CompiledPattern {
            id: "DB_URL",
            regex: Regex::new(
                r#"\b((?:postgres|postgresql|mysql|mongodb(\+srv)?|redis|amqp|amqps)://)[^\s"'<>]+"#,
            )
            .expect("DB_URL regex must compile"),
            replace: replace_db_url,
        },
        CompiledPattern {
            id: "SLACK_TOKEN",
            regex: Regex::new(r"\bxox[abprs]-[A-Za-z0-9\-]{10,}\b")
                .expect("SLACK_TOKEN regex must compile"),
            replace: replace_slack,
        },
        CompiledPattern {
            id: "BEARER_TOKEN",
            regex: Regex::new(r"(?i)\bBearer\s+([A-Za-z0-9\-._~+/]+=*)")
                .expect("BEARER_TOKEN regex must compile"),
            replace: replace_bearer,
        },
        // KEY_ASSIGN —— 区分大小写不敏感;要求 `(key|token|...)=value` 形式。
        // 值字符类排除 `[`,`]`,以保证 `[REDACTED]` 不会被二次匹配。
        // 注意:Rust 的 `regex` crate 不支持 look-around,所以用
        // 显式 capturing 前缀字符的方式代替 `(?<![A-Za-z0-9_])`。
        // 允许 `^` / 空白 / `_` / `-` 作为关键词与前一字符的分隔,既能
        // 命中 `API_KEY=...` / `MY_PASSWORD=...` 这类带前缀的字段,
        // 又能排除 `mykey=...` / `dontkey=...` 这类非密钥字段。
        CompiledPattern {
            id: "KEY_ASSIGN",
            regex: Regex::new(
                r#"(?i)(^|[\s_\-])(key|token|secret|password|credential|passwd|pwd)\s*([=:])\s*["']?([^ \t\r\n"',;)\[\]]+)"#,
            )
            .expect("KEY_ASSIGN regex must compile"),
            replace: |caps, marker| {
                let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let key = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let sep = caps.get(3).map(|m| m.as_str()).unwrap_or("=");
                format!("{prefix}{key}{sep}{marker}")
            },
        },
    ]
}

// ── short-circuit 快速路径 ───────────────────────────────────────

/// short-circuit 阈值 —— 短于这个长度的文本几乎不可能含完整 provider
/// key(prefix + value 加起来最少 30+ 字符),跳过 10 个正则直接返回。
///
/// Review 2026-06-30 P2-3:提为命名常量,加注释解释魔数由来。
///
/// 历史:这个数字从 v1.0.0-rc2 初始实现起就是字面量 `32`,没有任何
/// benchmark 支撑 —— 选 32 是因为它比所有默认 pattern 的"prefix +
/// minimum value length"和稍大(AWS AKIA + 16 = 20,加前后空白 32
/// 足够),又不会太长到把含密钥的工具输出错误跳过。如果未来要把
/// pattern 列表扩充到支持更长 prefix(比如 JWT 的 `eyJ...` 段),需要
/// 重新评估这个数字。
const SHORT_CIRCUIT_MIN_LEN: usize = 32;

/// 文本明显不含任何密钥字面量 —— 直接返回输入,跳过正则编译与遍历。
///
/// 启发式:短文本 + 缺少数值类触发字符,99% 的常见 `ok` / `done` /
/// `from read` 类输出命中此路径。完整模式覆盖仍依赖后续正则。
///
/// 当 sanitizer 含用户 `extra_patterns` 时(`has_extras = true`),
/// 不走 short-circuit —— 用户的自定义 trigger 字面量不可枚举。
fn short_circuit_ok(text: &str, has_extras: bool) -> bool {
    !has_extras
        && text.len() < SHORT_CIRCUIT_MIN_LEN
        // KEY_ASSIGN 同时接受 `=` 与 `:` 作为分隔符,两者都需要触发。
        && !text.contains('=')
        && !text.contains(':')
        && !text.contains("Bearer")
        && !text.contains("BEGIN")
        && !text.contains("sk-")
        && !text.contains("ghp_")
        && !text.contains("AKIA")
        && !text.contains("ASIA")
        && !text.contains("eyJ")
        && !text.contains("xox")
        // DB_URL schemes 一律含 `://`,加这个触发即可覆盖
        // postgres / mongodb / mysql / redis / amqp / amqps 等。
        && !text.contains("://")
}

// ── 公共入口 ─────────────────────────────────────────────────────

/// 对单段文本做脱敏。`Sanitizer` 不可变借用,无 clone 开销。
///
/// 若 [`Sanitizer::is_enabled`] 为 `false` 或短路径命中,返回原 `&str`(无 `String` 分配)。
pub fn sanitize_text(text: &str, sanitizer: &Sanitizer) -> String {
    if !sanitizer.is_enabled() || short_circuit_ok(text, sanitizer.inner.has_extras) {
        return text.to_string();
    }
    let mut out = text.to_string();
    for pat in &sanitizer.inner.patterns {
        out = pat
            .regex
            .replace_all(&out, |caps: &Captures| {
                (pat.replace)(caps, &sanitizer.inner.marker)
            })
            .into_owned();
    }
    out
}

/// 对单个 [`ContentBlock`] 做脱敏。`ToolUse` 与 `Image` 不动,`Text` / `Diff` /
/// `ToolResult` 递归处理。
pub fn sanitize_block(block: &mut ContentBlock, sanitizer: &Sanitizer) {
    match block {
        ContentBlock::Text { text } => {
            *text = sanitize_text(text, sanitizer);
        }
        ContentBlock::Diff { unified_diff } => {
            *unified_diff = sanitize_text(unified_diff, sanitizer);
        }
        ContentBlock::ToolResult { output, .. } => {
            // 递归对内层 ToolOutput 脱敏 —— 即使子代理 / 上层包装器
            // 已脱敏过一层,这里再次兜底,避免嵌套结构里的密钥通过
            // `ContentBlock::ToolResult` 边界漏出。
            sanitize_output(output, sanitizer);
        }
        ContentBlock::Image { .. } | ContentBlock::ToolUse { .. } => {
            // 二进制 / 请求类载荷,不动。
        }
    }
}

/// 对 [`ToolOutput`] 整体脱敏 —— 即 `Vec<ContentBlock>` 中每一块。
pub fn sanitize_output(output: &mut ToolOutput, sanitizer: &Sanitizer) {
    for block in output.content.iter_mut() {
        sanitize_block(block, sanitizer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect_protocol::{ContentBlock, ToolOutput};

    // ── 每个 pattern 正例 ─────────────────────────────────────────

    #[test]
    fn key_assign_redacts_key_equals_value() {
        let s = Sanitizer::with_defaults();
        assert_eq!(sanitize_text("API_KEY=secret123", &s), "API_KEY=[REDACTED]");
        assert_eq!(sanitize_text("password=hunter2", &s), "password=[REDACTED]");
        assert_eq!(sanitize_text("TOKEN: abcdefg", &s), "TOKEN:[REDACTED]");
        // 大小写不敏感
        assert_eq!(
            sanitize_text("Password=topsecret", &s),
            "Password=[REDACTED]"
        );
    }

    #[test]
    fn bearer_token_redacts_with_prefix_kept() {
        let s = Sanitizer::with_defaults();
        assert_eq!(
            sanitize_text("Authorization: Bearer eyJabc.def.ghi", &s),
            "Authorization: Bearer [REDACTED]"
        );
    }

    #[test]
    fn aws_access_key_redacts_with_type() {
        let s = Sanitizer::with_defaults();
        // 20 字符 AWS access key id(AKIA + 16 位)。
        assert_eq!(
            sanitize_text("AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE", &s),
            "AWS_ACCESS_KEY_ID=[REDACTED:aws_key]"
        );
    }

    #[test]
    fn openai_key_redacts_with_type() {
        let s = Sanitizer::with_defaults();
        let key = "sk-abcdefghijklmnopqrstuv";
        let out = sanitize_text(key, &s);
        assert!(out.contains("[REDACTED:openai_key]"));
        assert!(!out.contains("abcdefghijklmnopqrstuv"));
    }

    #[test]
    fn github_token_redacts_with_type() {
        let s = Sanitizer::with_defaults();
        let token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let out = sanitize_text(token, &s);
        assert!(out.contains("[REDACTED:github_token]"));
        assert!(!out.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn anthropic_key_redacts_with_type() {
        let s = Sanitizer::with_defaults();
        let key = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz";
        let out = sanitize_text(key, &s);
        assert!(out.contains("[REDACTED:anthropic_key]"));
        assert!(!out.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn private_key_block_redacts_whole_block() {
        let s = Sanitizer::with_defaults();
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAK...\n-----END RSA PRIVATE KEY-----";
        let out = sanitize_text(pem, &s);
        assert!(out.contains("[REDACTED:private_key]"));
        assert!(!out.contains("MIIEowIBAAK"));
    }

    #[test]
    fn jwt_redacts_with_type() {
        let s = Sanitizer::with_defaults();
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.SflKxw";
        let out = sanitize_text(jwt, &s);
        assert!(out.contains("[REDACTED:jwt]"));
        assert!(!out.contains("SflKxw"));
    }

    #[test]
    fn db_url_redacts_preserving_scheme() {
        let s = Sanitizer::with_defaults();
        assert_eq!(
            sanitize_text("postgres://user:pass@localhost/db", &s),
            "postgres://[REDACTED]"
        );
        assert_eq!(
            sanitize_text("mongodb://user:pass@host", &s),
            "mongodb://[REDACTED]"
        );
        assert_eq!(sanitize_text("redis://localhost", &s), "redis://[REDACTED]");
    }

    #[test]
    fn slack_token_redacts_with_type() {
        let s = Sanitizer::with_defaults();
        let tok = "xoxb-1234567890-abcdefghij";
        let out = sanitize_text(tok, &s);
        assert!(out.contains("[REDACTED:slack_token]"));
        assert!(!out.contains("1234567890"));
    }

    // ── false-positive guards ───────────────────────────────────

    #[test]
    fn bare_word_password_in_prose_not_redacted() {
        let s = Sanitizer::with_defaults();
        // 不带 `=` / `:` 的「password」字面量不应触发 KEY_ASSIGN。
        let out = sanitize_text("please enter your password above to continue", &s);
        assert!(out.contains("password"));
        assert!(!out.contains("[REDACTED]"));
    }

    #[test]
    fn basic_auth_header_not_redacted_as_bearer() {
        let s = Sanitizer::with_defaults();
        // `Authorization: Basic <base64>` 不应被 BEARER_TOKEN 命中。
        let out = sanitize_text("Authorization: Basic dXNlcjpwYXNz", &s);
        assert!(out.contains("dXNlcjpwYXNz"));
    }

    #[test]
    fn short_circuit_fast_path_skips_regex() {
        let s = Sanitizer::with_defaults();
        let out = sanitize_text("ok", &s);
        assert_eq!(out, "ok");
        let out = sanitize_text("done", &s);
        assert_eq!(out, "done");
    }

    #[test]
    fn short_hex_string_not_redacted_as_github_token() {
        // 32 字符 hex 串不是 GitHub token(GitHub token 是 36+ 字符 base62)。
        let s = Sanitizer::with_defaults();
        let out = sanitize_text("hash=deadbeef0123456789abcdef01234567", &s);
        assert!(!out.contains("[REDACTED:github_token]"));
    }

    #[test]
    fn ssh_public_key_not_redacted() {
        let s = Sanitizer::with_defaults();
        let ssh = "ssh-rsa AAAA... user@host";
        let out = sanitize_text(ssh, &s);
        assert!(out.contains("ssh-rsa"));
        // PRIVATE_KEY_BLOCK 只匹配 PRIVATE KEY,不会命中 ssh-rsa 公钥。
        assert!(!out.contains("[REDACTED:private_key]"));
    }

    // ── 幂等性 ───────────────────────────────────────────────────

    #[test]
    fn default_patterns_are_idempotent() {
        let s = Sanitizer::with_defaults();
        let fixtures = [
            "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE",
            "Authorization: Bearer abc.def.ghi",
            "sk-abcdefghijklmnopqrstuv",
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "postgres://user:pass@host/db",
            "-----BEGIN PRIVATE KEY-----\nABC\n-----END PRIVATE KEY-----",
            "xoxb-1234567890-abcdef",
        ];
        for raw in fixtures {
            let once = sanitize_text(raw, &s);
            let twice = sanitize_text(&once, &s);
            assert_eq!(
                once, twice,
                "pattern should be idempotent on input: {raw:?} → first {once:?} → second {twice:?}"
            );
        }
    }

    #[test]
    fn marker_does_not_re_match_after_redaction() {
        let s = Sanitizer::with_defaults();
        // 一旦脱敏完成,marker 文本不应触发任何 pattern 再匹配。
        let marker_only = DEFAULT_MARKER;
        let out = sanitize_text(marker_only, &s);
        assert_eq!(out, marker_only);
    }

    // ── disabled / 配置 ──────────────────────────────────────────

    #[test]
    fn disabled_sanitizer_is_noop() {
        let s = Sanitizer::disabled();
        assert!(!s.is_enabled());
        let raw = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        assert_eq!(sanitize_text(raw, &s), raw);
    }

    #[test]
    fn from_config_with_enabled_false_yields_disabled() {
        let cfg = SanitizeConfig {
            enabled: Some(false),
            ..Default::default()
        };
        let s = Sanitizer::from_config(&cfg).unwrap();
        assert!(!s.is_enabled());
    }

    #[test]
    fn from_config_with_custom_marker() {
        let cfg = SanitizeConfig {
            marker: Some("[HIDDEN]".into()),
            ..Default::default()
        };
        let s = Sanitizer::from_config(&cfg).unwrap();
        assert_eq!(sanitize_text("API_KEY=secret", &s), "API_KEY=[HIDDEN]");
    }

    #[test]
    fn from_config_with_extra_pattern() {
        let cfg = SanitizeConfig {
            extra_patterns: Some(vec![r"(?i)\bMYTOKEN\b".into()]),
            ..Default::default()
        };
        let s = Sanitizer::from_config(&cfg).unwrap();
        // 默认 pattern 仍生效
        assert!(sanitize_text("API_KEY=x", &s).contains("[REDACTED]"));
        // 额外 pattern 也生效
        assert_eq!(sanitize_text("MYTOKEN=abc", &s), "[REDACTED]=abc");
    }

    #[test]
    fn from_config_with_disable_default_patterns() {
        let cfg = SanitizeConfig {
            disable_default_patterns: Some(true),
            extra_patterns: Some(vec![r"(?i)\bFOO\b".into()]),
            ..Default::default()
        };
        let s = Sanitizer::from_config(&cfg).unwrap();
        // AWS key 不再脱敏(默认 pattern 已禁)
        assert_eq!(
            sanitize_text("AKIAIOSFODNN7EXAMPLE", &s),
            "AKIAIOSFODNN7EXAMPLE"
        );
        // extra pattern 仍生效
        assert_eq!(
            sanitize_text("hello FOO world", &s),
            "hello [REDACTED] world"
        );
    }

    #[test]
    fn from_config_invalid_extra_pattern_returns_error() {
        let cfg = SanitizeConfig {
            extra_patterns: Some(vec!["[unclosed".into()]),
            ..Default::default()
        };
        let err = Sanitizer::from_config(&cfg).unwrap_err();
        match err {
            SanitizeError::InvalidPattern {
                index,
                pattern_source,
                ..
            } => {
                assert_eq!(index, 0);
                assert_eq!(pattern_source, "[unclosed");
            }
        }
    }

    #[test]
    fn from_config_empty_uses_defaults() {
        let cfg = SanitizeConfig::default();
        let s = Sanitizer::from_config(&cfg).unwrap();
        // 默认 pattern 数应 ≥ 10。
        assert!(s.pattern_count() >= 10);
        assert!(s.is_enabled());
        assert_eq!(s.marker(), DEFAULT_MARKER);
    }

    /// Review 2026-06-30 P2-7:`extra_patterns = Some(vec![])`(显式空 vec)
    /// 与 `extra_patterns = None`(未声明字段)行为一致 —— 不影响默认 pattern
    /// 集合,`has_extras` 标志保持 false。
    ///
    /// 实现层:`has_extras = is_some_and(|v| !v.is_empty())`,空 vec 时走 false,
    /// short-circuit 仍可用。
    #[test]
    fn from_config_with_empty_extra_patterns_uses_defaults() {
        let cfg = SanitizeConfig {
            enabled: Some(true),
            extra_patterns: Some(vec![]),
            ..Default::default()
        };
        let s = Sanitizer::from_config(&cfg).unwrap();
        assert_eq!(
            s.pattern_count(),
            10,
            "空 extra_patterns 不应改变默认 10-pattern"
        );
        assert!(s.is_enabled());
        // 默认 pattern 仍生效
        assert_eq!(sanitize_text("API_KEY=x", &s), "API_KEY=[REDACTED]");
        // short-circuit 仍命中(has_extras == false)
        assert_eq!(sanitize_text("ok", &s), "ok");
    }

    // ── ContentBlock 处理 ─────────────────────────────────────────

    #[test]
    fn text_block_is_redacted() {
        let s = Sanitizer::with_defaults();
        let mut block = ContentBlock::text("API_KEY=secret");
        sanitize_block(&mut block, &s);
        match block {
            ContentBlock::Text { text } => assert_eq!(text, "API_KEY=[REDACTED]"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn image_block_passes_through_untouched() {
        let s = Sanitizer::with_defaults();
        let mut block = ContentBlock::Image {
            data: vec![0xFF, 0xD8, 0xFF],
            mime_type: "image/jpeg".into(),
        };
        sanitize_block(&mut block, &s);
        match block {
            ContentBlock::Image { data, mime_type } => {
                assert_eq!(data, vec![0xFF, 0xD8, 0xFF]);
                assert_eq!(mime_type, "image/jpeg");
            }
            _ => panic!("expected Image"),
        }
    }

    #[test]
    fn tool_use_block_passes_through() {
        let s = Sanitizer::with_defaults();
        let mut block = ContentBlock::ToolUse {
            id: "call_1".into(),
            name: "bash".into(),
            args: serde_json::json!({"cmd": "echo AKIAIOSFODNN7EXAMPLE"}),
        };
        sanitize_block(&mut block, &s);
        // ToolUse 是请求类载荷,不脱敏 —— 用户消息脱敏是后续 v1.1 范围。
        match block {
            ContentBlock::ToolUse { args, .. } => {
                assert!(args.to_string().contains("AKIAIOSFODNN7EXAMPLE"));
            }
            _ => panic!("expected ToolUse"),
        }
    }

    #[test]
    fn tool_result_block_recurses() {
        let s = Sanitizer::with_defaults();
        let inner_output = ToolOutput {
            content: vec![ContentBlock::text("API_KEY=secret")],
            is_error: false,
            metadata: serde_json::json!({}),
            elapsed_ms: 0,
        };
        let mut block = ContentBlock::ToolResult {
            call_id: "c".into(),
            output: inner_output,
        };
        sanitize_block(&mut block, &s);
        match block {
            ContentBlock::ToolResult { output, .. } => match &output.content[0] {
                ContentBlock::Text { text } => {
                    assert_eq!(text, "API_KEY=[REDACTED]");
                }
                _ => panic!("expected inner Text"),
            },
            _ => panic!("expected ToolResult"),
        }
    }

    #[test]
    fn diff_block_is_redacted() {
        let s = Sanitizer::with_defaults();
        let mut block = ContentBlock::Diff {
            unified_diff: "+API_KEY=secret\n-ok".into(),
        };
        sanitize_block(&mut block, &s);
        match block {
            ContentBlock::Diff { unified_diff } => {
                assert!(unified_diff.contains("API_KEY=[REDACTED]"));
                assert!(unified_diff.contains("-ok"));
            }
            _ => panic!("expected Diff"),
        }
    }

    // ── ToolOutput 整体 ─────────────────────────────────────────

    #[test]
    fn sanitize_output_handles_multi_block() {
        let s = Sanitizer::with_defaults();
        let mut output = ToolOutput {
            content: vec![
                ContentBlock::text("hello"),
                ContentBlock::text("API_KEY=secret"),
                ContentBlock::Image {
                    data: vec![1, 2, 3],
                    mime_type: "image/png".into(),
                },
            ],
            is_error: false,
            metadata: serde_json::json!({}),
            elapsed_ms: 0,
        };
        sanitize_output(&mut output, &s);
        match &output.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "hello"),
            _ => panic!(),
        }
        match &output.content[1] {
            ContentBlock::Text { text } => assert_eq!(text, "API_KEY=[REDACTED]"),
            _ => panic!(),
        }
        // Image 未动。
        assert!(matches!(&output.content[2], ContentBlock::Image { .. }));
    }

    // ── 元信息 ──────────────────────────────────────────────────

    #[test]
    fn default_patterns_count_is_ten() {
        let s = Sanitizer::with_defaults();
        assert_eq!(s.pattern_count(), 10);
    }

    /// Review 2026-06-30 P0-2:把 `default_patterns()` 的隐式顺序约束
    /// 写进测试,防止后续 PR 调整 pattern 顺序时静默打破语义。
    ///
    /// 关键顺序(详见 `default_patterns` 函数内注释):
    /// 1. `KEY_ASSIGN` 必须放最后 —— 否则 `AWS_ACCESS_KEY_ID=AKIA...`
    ///    会被 KEY_ASSIGN 整段吞成 `[REDACTED:key]`,丢失 type 标识。
    /// 2. `ANTHROPIC_KEY` 必须先于 `OPENAI_KEY` —— `sk-ant-...` 是
    ///    `sk-...` 子集,否则 OPENAI 会先抢跑替换成 `[REDACTED:openai_key]`。
    /// 3. 具体 provider pattern (AWS/ANTHROPIC/OPENAI/GITHUB/PRIVATE_KEY/
    ///    JWT/DB_URL/SLACK/BEARER) 必须在 `KEY_ASSIGN` 之前,否则
    ///    KEY_ASSIGN 会把它们的字面量提前吞掉。
    #[test]
    fn default_patterns_order_is_canonical() {
        let patterns = default_patterns();
        let ids: Vec<&'static str> = patterns.iter().map(|p| p.id).collect();

        // 1. KEY_ASSIGN 必须放最后
        assert_eq!(
            ids.last().copied(),
            Some("KEY_ASSIGN"),
            "KEY_ASSIGN must be the last pattern (got order: {ids:?})"
        );

        // 2. ANTHROPIC_KEY 必须在 OPENAI_KEY 之前
        let anthropic_pos = ids
            .iter()
            .position(|&id| id == "ANTHROPIC_KEY")
            .expect("ANTHROPIC_KEY must exist");
        let openai_pos = ids
            .iter()
            .position(|&id| id == "OPENAI_KEY")
            .expect("OPENAI_KEY must exist");
        assert!(
            anthropic_pos < openai_pos,
            "ANTHROPIC_KEY (pos {anthropic_pos}) must precede OPENAI_KEY (pos {openai_pos}); \
             otherwise sk-ant-... gets replaced as [REDACTED:openai_key]"
        );

        // 3. 具体 provider pattern 必须在 KEY_ASSIGN 之前
        let key_assign_pos = anthropic_pos.max(openai_pos);
        for concrete in [
            "AWS_ACCESS_KEY",
            "GITHUB_TOKEN",
            "PRIVATE_KEY_BLOCK",
            "JWT",
            "DB_URL",
            "SLACK_TOKEN",
            "BEARER_TOKEN",
        ] {
            let pos = ids
                .iter()
                .position(|&id| id == concrete)
                .unwrap_or_else(|| panic!("{concrete} must exist in default_patterns"));
            assert!(
                pos < key_assign_pos.max(ids.len() - 1),
                "{concrete} (pos {pos}) must come before KEY_ASSIGN (pos {})",
                ids.len() - 1
            );
        }
    }

    #[test]
    fn short_circuit_skips_all_known_pattern_triggers() {
        let s = Sanitizer::with_defaults();
        // 含数字但不含任何触发字符:short-circuit 路径,不应被任何 pattern 影响。
        let inputs = [
            "loaded 100 lines",
            "no matches found",
            "build succeeded",
            "tests passed: 42",
        ];
        for raw in inputs {
            assert_eq!(
                sanitize_text(raw, &s),
                raw,
                "short-circuit broken on {raw:?}"
            );
        }
    }

    // ── 复杂场景 ───────────────────────────────────────────────

    #[test]
    fn multi_secret_in_one_string() {
        let s = Sanitizer::with_defaults();
        let raw = "AWS_KEY=AKIAIOSFODNN7EXAMPLE Authorization: Bearer eyJhbGciOi.body.sig";
        let out = sanitize_text(raw, &s);
        assert!(out.contains("[REDACTED:aws_key]"));
        assert!(out.contains("Bearer [REDACTED]"));
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn multiline_tool_output_redacts_per_line() {
        let s = Sanitizer::with_defaults();
        let raw = "line 1\nAPI_KEY=secret\nline 3\nTOKEN=hunter2\nline 5";
        let out = sanitize_text(raw, &s);
        assert_eq!(
            out,
            "line 1\nAPI_KEY=[REDACTED]\nline 3\nTOKEN=[REDACTED]\nline 5"
        );
    }
}
