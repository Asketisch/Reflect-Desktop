//! Config + tool registry 命令。

use serde::Serialize;
use tauri::State;

use reflect_config::{default_config_path, load_from_str};

use crate::commands::error::{CommandError, CommandResult};
use crate::state::{AgentStatus, MinimalAgent};

/// 返回 agent 状态快照(ready / has_model / model / workspace / degraded_reason)。
/// 前端用来显示状态徽标 + 引导用户去 Settings 配 API key。
#[tauri::command]
pub async fn reflect_agent_status(agent: State<'_, MinimalAgent>) -> CommandResult<AgentStatus> {
    Ok(agent.agent_status())
}

/// 返回当前 `~/.reflect/config.toml` 的 TOML 字符串。Settings 页加载用。
#[tauri::command]
pub async fn reflect_get_config(agent: State<'_, MinimalAgent>) -> CommandResult<String> {
    let cfg = agent.cfg();
    let toml = toml::to_string_pretty(&*cfg.read()).map_err(|e| CommandError {
        msg: format!("serialize config: {e}"),
    })?;
    Ok(toml)
}

/// 新配置写盘 + 热替换共享 cfg(校验由调用方完成)。返回旧快照供调用方
/// 做变更检测(`hot_reload_provider_stack` 需要 old/new 对比)。
fn persist_config(
    agent: &MinimalAgent,
    new_cfg: reflect_config::ReflectConfig,
) -> CommandResult<reflect_config::ReflectConfig> {
    let toml = toml::to_string_pretty(&new_cfg).map_err(|e| CommandError {
        msg: format!("serialize config: {e}"),
    })?;
    let path = default_config_path().ok_or_else(|| CommandError {
        msg: "no HOME dir for config".into(),
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &toml)?;
    let old_cfg = {
        let cfg_arc = agent.cfg();
        let mut guard = cfg_arc.write();
        std::mem::replace(&mut *guard, new_cfg)
    };
    tracing::info!(
        "[reflect-gui] config saved to {} (hot-reloaded in-memory)",
        path.display()
    );
    Ok(old_cfg)
}

/// 写回 `~/.reflect/config.toml`。写盘前用 `load_from_str` 校验合法性,
/// 防止坏 TOML 损坏配置;校验通过才覆盖,并热更新共享 cfg。
///
/// provider 相关段(`active` / `anthropic` / `openai` / `ollama` / `routing`)
/// 变化时,进一步重建 ModelRegistry + QuotaTracker 并强制重绑当前会话
/// (见 `state/reload.rs`)—— coding plan 的新增/切换**无需重启**即对
/// 运行中的会话生效。其余段落(MCP、hooks、显示等)仍只热更新 cfg,
/// 由各自的重载机制接管。
#[tauri::command]
pub async fn reflect_save_config(
    agent: State<'_, MinimalAgent>,
    toml: String,
) -> CommandResult<()> {
    // 1. 校验:能否解析回 ReflectConfig。
    let new_cfg = load_from_str(&toml)?;
    // 2. 写盘 + 热更新共享 cfg(保留旧快照供变更检测)。
    let old_cfg = persist_config(&agent, new_cfg.clone())?;
    // 3. provider 栈变化 → 重建 registry/tracker + 重绑当前会话。
    crate::state::reload::hot_reload_provider_stack(&agent, &old_cfg, &new_cfg).await;
    Ok(())
}

/// 运行中切换模型(spec = `{provider}/{model}`,如 `openai/gpt-4o`)。
///
/// 协议 `Op` 没有 SetModel —— 模型 spec 在配置层解析
/// (`resolved_model_spec()` = env > `[<provider>].model` > 内置默认),
/// 因此走与 ModelsView 切 coding plan 相同的路径:改 `[active].provider`
/// + `[<provider>].model` → 写盘 → `hot_reload_provider_stack`
/// (重建 registry + 强制重绑当前会话),下一个 turn 立即生效。
///
/// `model` 传空串 = 清除该 provider 的段级 model 覆盖,回落到内置默认。
#[tauri::command]
pub async fn reflect_set_model(
    agent: State<'_, MinimalAgent>,
    provider: String,
    model: String,
) -> CommandResult<String> {
    if !matches!(provider.as_str(), "anthropic" | "openai" | "ollama") {
        return Err(CommandError {
            msg: format!("unknown provider: {provider} (expected anthropic | openai | ollama)"),
        });
    }
    let mut new_cfg = agent.cfg().read().clone();
    new_cfg.active.provider = Some(provider.clone());
    let trimmed = model.trim().to_string();
    match provider.as_str() {
        "anthropic" => {
            let section = new_cfg.anthropic.get_or_insert_with(Default::default);
            section.model = (!trimmed.is_empty()).then(|| trimmed.clone());
        }
        "openai" => {
            let section = new_cfg.openai.get_or_insert_with(Default::default);
            section.model = (!trimmed.is_empty()).then(|| trimmed.clone());
        }
        "ollama" => {
            let section = new_cfg.ollama.get_or_insert_with(Default::default);
            section.model = (!trimmed.is_empty()).then(|| trimmed.clone());
        }
        _ => unreachable!("provider validated above"),
    }
    let old_cfg = persist_config(&agent, new_cfg.clone())?;
    crate::state::reload::hot_reload_provider_stack(&agent, &old_cfg, &new_cfg).await;
    let spec = new_cfg.resolved_model_spec().unwrap_or_default();
    tracing::info!("[reflect-gui] model switched to {spec} (hot-reloaded)");
    Ok(spec)
}

/// 列出当前 ToolRegistry 中所有工具(name + description)。
/// 前端 Settings / Skills 页展示可用工具列表用。
#[tauri::command]
pub async fn reflect_list_tools(agent: State<'_, MinimalAgent>) -> CommandResult<Vec<ToolInfo>> {
    let tools = agent.tools();
    let mut out: Vec<ToolInfo> = tools
        .list()
        .into_iter()
        .filter_map(|name| {
            let t = tools.get(&name)?;
            Some(ToolInfo {
                name,
                description: t.description().to_string(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// 单个工具的 name + description。
#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

// ── Coding plan 余量查询 ─────────────────────────────────────────

/// coding plan 余量快照(序列化给前端)。字段对齐 `reflect_llm::QuotaSnapshot`,
/// 仅把 `resets_at` 从 `DateTime<Utc>` 转成 RFC3339 字符串。
#[derive(Debug, Clone, Serialize)]
pub struct PlanQuotaSnapshot {
    /// 查询是否成功。`false` = 鉴权/解析等确定性失败(或网络瞬时失败),
    /// 原因见 `error`。
    pub success: bool,
    pub error: Option<String>,
    /// 主窗口已用百分比 0-100(厂商未返回为 null)。
    pub utilization: Option<f64>,
    /// 剩余 token 绝对值(Kimi 等直接给 remaining;百分比型厂商为 null)。
    pub remaining_tokens: Option<u64>,
    /// 窗口上限 token 绝对值。
    pub max_tokens: Option<u64>,
    /// 窗口重置时间(RFC3339;厂商未返回为 null)。
    pub resets_at: Option<String>,
}

/// 把 `check_via` 字符串(`kimi | zhipu | minimax | zenmux | …`,与
/// `reflect_config::QuotaSource` 的 snake_case 序列化一致)解析成 provider 实例。
/// 复用 `state::quota::make_provider` 的同一份映射;`None`(火山/Anthropic/
/// OpenAI usage 暂未实现)报错给前端,而非静默走本地统计。
fn make_quota_provider(
    check_via: &str,
) -> Result<std::sync::Arc<dyn reflect_llm::QuotaProvider>, CommandError> {
    if check_via.is_empty() {
        return Err(CommandError {
            msg: "check_via is empty — set quota.check_via on the plan first".into(),
        });
    }
    let src: reflect_config::QuotaSource =
        serde_json::from_value(serde_json::Value::String(check_via.to_string())).map_err(|_| {
            CommandError {
                msg: format!("unknown check_via: {check_via}"),
            }
        })?;
    crate::state::quota::make_provider(&src).ok_or_else(|| CommandError {
        msg: format!(
            "quota query via '{check_via}' is not implemented yet (volcengine needs AK/SK signing; \
             anthropic_usage / open_a_i_usage need OAuth credentials)"
        ),
    })
}

/// 查询指定 coding plan 的剩余额度(对着厂商用量 API,端点与 cc-switch 一致:
/// Kimi `/coding/v1/usages`、智谱 `/api/monitor/usage/quota/limit`、
/// MiniMax `/coding_plan/remains`、ZenMux base_url)。查询本体在
/// `reflect_llm` 的 `QuotaProvider` 实现里,与运行时 failover 判定共用。
///
/// 传显式 `base_url` / `api_key` 而非 (provider, label):前端编辑中的未保存
/// 表单也能直接试查,无需先落盘。网络瞬时失败同样折叠为
/// `success=false`——对手动查询而言展示层处理方式相同。
#[tauri::command]
pub async fn reflect_query_plan_quota(
    base_url: String,
    api_key: String,
    check_via: String,
) -> CommandResult<PlanQuotaSnapshot> {
    let provider = make_quota_provider(&check_via)?;
    let snapshot = match provider.query(&base_url, &api_key).await {
        Ok(s) => s,
        Err(e) => {
            // 瞬时失败(网络/读体超时)对 UI 与确定性失败同形:success=false + error。
            return Ok(PlanQuotaSnapshot {
                success: false,
                error: Some(e.to_string()),
                utilization: None,
                remaining_tokens: None,
                max_tokens: None,
                resets_at: None,
            });
        }
    };
    tracing::info!(
        check_via = %check_via,
        success = snapshot.success,
        utilization = ?snapshot.utilization,
        "plan quota queried"
    );
    Ok(PlanQuotaSnapshot {
        success: snapshot.success,
        error: snapshot.error,
        utilization: snapshot.utilization,
        remaining_tokens: snapshot.remaining_tokens,
        max_tokens: snapshot.max_tokens,
        resets_at: snapshot.resets_at.map(|dt| dt.to_rfc3339()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// check_via → provider 映射:已实现四家命中,未实现三家报错。
    #[test]
    fn check_via_routes_to_provider_or_error() {
        for ok in ["kimi", "zhipu", "minimax", "zenmux"] {
            assert!(
                make_quota_provider(ok).is_ok(),
                "{ok} should map to a provider"
            );
        }
        for unsupported in ["volcengine", "anthropic_usage", "open_a_i_usage"] {
            let Err(err) = make_quota_provider(unsupported) else {
                panic!("{unsupported} should not map to a provider yet");
            };
            assert!(err.msg.contains("not implemented"), "{err:?}");
        }
        // 注意 serde rename_all = "snake_case" 对连续大写逐字母拆词:
        // OpenAIUsage → "open_a_i_usage"(探测确认)。因此 openai_usage /
        // open_ai_usage 等常见拼法都走 unknown 分支被拒绝,防止静默吞错。
        assert!(make_quota_provider("openai_usage").is_err());
        assert!(make_quota_provider("open_ai_usage").is_err());
        assert!(make_quota_provider("").is_err());
        assert!(make_quota_provider("nonsense").is_err());
    }
}
