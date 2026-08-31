//! 模型列表查询:对着 provider 的用量端口拉取可用模型。
//!
//! 两种接入端口(与 plans 的 provider 语义一致):
//! - `openai`   → `GET {base_url}/models`,`Authorization: Bearer <key>`
//! - `anthropic`→ `GET {base_url}/v1/models`,`x-api-key` + `anthropic-version`
//!
//! 部分兼容网关(OpenRouter 风格)会在模型对象里带
//! `architecture.input_modalities` / `input_modalities`,据此给出视觉能力
//! 标记;官方 API 不返回该信息时为 `None`,由用户自行判断。

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

use crate::commands::error::{CommandError, CommandResult};

/// 单个模型条目。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProviderModelInfo {
    /// 模型 id(调用方直接作为 model spec 使用)。
    pub id: String,
    /// 展示名(厂商未返回时与 id 相同)。
    pub display_name: String,
    /// 厂商声明的视觉输入能力。`None` = 接口未返回,由用户自行判断。
    pub supports_vision: Option<bool>,
}

/// 模型列表 + 查询端口。
#[derive(Debug, Clone, Serialize)]
pub struct ProviderModelsResult {
    pub endpoint: String,
    pub models: Vec<ProviderModelInfo>,
}

/// 拼接模型列表 URL。base_url 允许带或不带 `/v1`(Anthropic),
/// 结尾 `/` 一律归一。
pub(crate) fn build_models_url(base_url: &str, endpoint: &str) -> Result<String, CommandError> {
    let base = base_url.trim_end_matches('/');
    if base.is_empty() {
        return Err(CommandError {
            msg: "base_url is empty".into(),
        });
    }
    match endpoint {
        "openai" => Ok(format!("{base}/models")),
        "anthropic" => {
            // 官方约定 https://api.anthropic.com(无 /v1);用户若已填 /v1 不重复追加。
            if base.ends_with("/v1") {
                Ok(format!("{base}/models"))
            } else {
                Ok(format!("{base}/v1/models"))
            }
        }
        other => Err(CommandError {
            msg: format!("unknown endpoint: {other} (expected anthropic | openai)"),
        }),
    }
}

/// 从模型对象提取视觉能力。识别 OpenRouter 风格
/// `architecture.input_modalities` 与顶层 `input_modalities`(字符串数组,
/// 含 "image" 即具备视觉输入);字段缺失 → `None`。
fn detect_vision(item: &Value) -> Option<bool> {
    let modalities = item
        .get("architecture")
        .and_then(|a| a.get("input_modalities"))
        .or_else(|| item.get("input_modalities"))?
        .as_array()?;
    Some(
        modalities
            .iter()
            .filter_map(|v| v.as_str())
            .any(|s| s.eq_ignore_ascii_case("image")),
    )
}

/// 解析 /models 响应体。两种端口同为 `{ data: [...] }` 外层;
/// 条目字段按端口取 id / display_name。
pub(crate) fn parse_models_response(
    endpoint: &str,
    body: &[u8],
) -> Result<Vec<ProviderModelInfo>, CommandError> {
    let v: Value = serde_json::from_slice(body).map_err(|e| CommandError {
        msg: format!("failed to parse models response: {e}"),
    })?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| CommandError {
            msg: "models response missing 'data' array".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(id) = item.get("id").and_then(|i| i.as_str()) else {
            continue;
        };
        let display_name = item
            .get("display_name")
            .and_then(|d| d.as_str())
            .unwrap_or(id);
        out.push(ProviderModelInfo {
            id: id.to_string(),
            display_name: display_name.to_string(),
            supports_vision: if endpoint == "openai" {
                detect_vision(item)
            } else {
                None
            },
        });
    }
    // id 去重(部分网关同一 id 返回多条)并保持返回顺序。
    let mut seen = std::collections::HashSet::new();
    out.retain(|m| seen.insert(m.id.clone()));
    Ok(out)
}

/// 拉取指定 base_url + api_key 下的可用模型列表。
///
/// 与 `reflect_query_plan_quota` 同风格:鉴权失败 / HTTP 错误 / 解析失败
/// 报 `Err` 给前端展示;成功返回模型数组(可能为空)。
#[tauri::command]
pub async fn reflect_list_provider_models(
    base_url: String,
    api_key: String,
    endpoint: String,
) -> CommandResult<ProviderModelsResult> {
    if api_key.trim().is_empty() {
        return Err(CommandError {
            msg: "api_key is empty".into(),
        });
    }
    let url = build_models_url(&base_url, &endpoint)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| CommandError {
            msg: format!("http client: {e}"),
        })?;
    let mut req = client.get(&url);
    req = match endpoint.as_str() {
        // 智谱等兼容端点同样吃 Bearer;Anthropic 用 x-api-key + 版本头。
        "openai" => req.header("Authorization", format!("Bearer {api_key}")),
        "anthropic" => req
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01"),
        _ => unreachable!("build_models_url validated endpoint"),
    };
    let resp = req
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| CommandError {
            msg: format!("network error: {e}"),
        })?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(CommandError {
            msg: format!("authentication failed (HTTP {status}) — check the API key"),
        });
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(200).collect();
        return Err(CommandError {
            msg: format!("API error (HTTP {status}): {snippet}"),
        });
    }
    // 先读完整 body(读体失败 = 瞬时),再解析(解析失败 = 确定性)。
    let body = resp.bytes().await.map_err(|e| CommandError {
        msg: format!("failed to read response: {e}"),
    })?;
    let models = parse_models_response(&endpoint, &body)?;
    tracing::info!(endpoint = %endpoint, count = models.len(), "provider models listed");
    Ok(ProviderModelsResult { endpoint, models })
}

// ── 连通性测试(发送测试文字) ─────────────────────────────────────

/// 测试消息:固定「你好」,max_tokens 压到 32 —— 只为验证鉴权/网络/响应
/// 解析链路,把 token 消耗压到最低。
pub(crate) const TEST_PROMPT: &str = "你好";
pub(crate) const TEST_MAX_TOKENS: u32 = 32;

/// 发送测试文字的结果。
#[derive(Debug, Clone, Serialize)]
pub struct ProviderChatTestResult {
    /// 模型返回的文本(可能为空串)。
    pub reply: String,
}

/// 拼接 chat 测试 URL(与 models URL 同一套归一规则)。
pub(crate) fn build_chat_test_url(base_url: &str, endpoint: &str) -> Result<String, CommandError> {
    let base = base_url.trim_end_matches('/');
    if base.is_empty() {
        return Err(CommandError {
            msg: "base_url is empty".into(),
        });
    }
    match endpoint {
        "openai" => Ok(format!("{base}/chat/completions")),
        "anthropic" => {
            if base.ends_with("/v1") {
                Ok(format!("{base}/messages"))
            } else {
                Ok(format!("{base}/v1/messages"))
            }
        }
        other => Err(CommandError {
            msg: format!("unknown endpoint: {other} (expected anthropic | openai)"),
        }),
    }
}

/// 构造测试请求体:单条用户消息,无 system、无 tools —— 避免拉起庞大
/// 提示词造成无效消耗。
pub(crate) fn build_chat_test_body(model: &str) -> Value {
    serde_json::json!({
        "model": model,
        "max_tokens": TEST_MAX_TOKENS,
        "messages": [{ "role": "user", "content": TEST_PROMPT }],
    })
}

/// 从两种端点的响应里提取文本回复。
pub(crate) fn parse_chat_test_reply(endpoint: &str, body: &[u8]) -> Result<String, CommandError> {
    let v: Value = serde_json::from_slice(body).map_err(|e| CommandError {
        msg: format!("failed to parse chat response: {e}"),
    })?;
    let reply = match endpoint {
        // OpenAI:`choices[0].message.content`(可为 null)。
        "openai" => v
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string(),
        // Anthropic:`content[]` 里全部 text 块拼接。
        "anthropic" => v
            .get("content")
            .and_then(|c| c.as_array())
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default(),
        other => {
            return Err(CommandError {
                msg: format!("unknown endpoint: {other}"),
            });
        }
    };
    Ok(reply)
}

/// 向 provider 发送一条「你好」测试消息,验证鉴权/网络/推理链路是否可用。
///
/// **刻意不带 system 提示词与 tools 定义** —— 避免把庞大系统提示词发给
/// 厂商造成无效消耗;`max_tokens` 压到 32。只测连通,不用于生产对话。
#[tauri::command]
pub async fn reflect_test_provider_chat(
    base_url: String,
    api_key: String,
    endpoint: String,
    model: String,
) -> CommandResult<ProviderChatTestResult> {
    if api_key.trim().is_empty() {
        return Err(CommandError {
            msg: "api_key is empty".into(),
        });
    }
    if model.trim().is_empty() {
        return Err(CommandError {
            msg: "model is empty — fetch or enter a model first".into(),
        });
    }
    let url = build_chat_test_url(&base_url, &endpoint)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| CommandError {
            msg: format!("http client: {e}"),
        })?;
    let mut req = client.post(&url).json(&build_chat_test_body(model.trim()));
    req = match endpoint.as_str() {
        "openai" => req.header("Authorization", format!("Bearer {api_key}")),
        "anthropic" => req
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01"),
        _ => unreachable!("build_chat_test_url validated endpoint"),
    };
    let resp = req.send().await.map_err(|e| CommandError {
        msg: format!("network error: {e}"),
    })?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(CommandError {
            msg: format!("authentication failed (HTTP {status}) — check the API key"),
        });
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(200).collect();
        return Err(CommandError {
            msg: format!("API error (HTTP {status}): {snippet}"),
        });
    }
    let body = resp.bytes().await.map_err(|e| CommandError {
        msg: format!("failed to read response: {e}"),
    })?;
    let reply = parse_chat_test_reply(&endpoint, &body)?;
    tracing::info!(endpoint = %endpoint, reply_len = reply.len(), "provider chat test ok");
    Ok(ProviderChatTestResult { reply })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn models_url_by_endpoint() {
        assert_eq!(
            build_models_url("https://api.openai.com/v1/", "openai").unwrap(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            build_models_url("https://api.anthropic.com", "anthropic").unwrap(),
            "https://api.anthropic.com/v1/models"
        );
        // 已带 /v1 的 anthropic base 不重复追加。
        assert_eq!(
            build_models_url("https://api.anthropic.com/v1", "anthropic").unwrap(),
            "https://api.anthropic.com/v1/models"
        );
        assert!(build_models_url("", "openai").is_err());
        assert!(build_models_url("https://x", "ollama").is_err());
    }

    #[test]
    fn parse_openai_models_with_vision_hint() {
        let body = serde_json::to_vec(&json!({
            "data": [
                { "id": "gpt-4o", "object": "model",
                  "architecture": { "input_modalities": ["text", "image"] } },
                { "id": "o1", "object": "model",
                  "architecture": { "input_modalities": ["text"] } },
                { "id": "gpt-3.5-turbo", "object": "model" },
                { "id": "gpt-4o", "object": "model" }
            ]
        }))
        .unwrap();
        let models = parse_models_response("openai", &body).unwrap();
        assert_eq!(models.len(), 3, "重复 id 去重");
        assert_eq!(models[0].supports_vision, Some(true));
        assert_eq!(models[1].supports_vision, Some(false));
        assert_eq!(models[2].supports_vision, None, "无字段 → 用户自行判断");
        assert_eq!(models[0].display_name, "gpt-4o");
    }

    #[test]
    fn parse_anthropic_models_with_display_name() {
        let body = serde_json::to_vec(&json!({
            "data": [
                { "type": "model", "id": "claude-sonnet-4-5", "display_name": "Claude Sonnet 4.5" }
            ]
        }))
        .unwrap();
        let models = parse_models_response("anthropic", &body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].display_name, "Claude Sonnet 4.5");
        // 官方 /v1/models 不返回能力信息。
        assert_eq!(models[0].supports_vision, None);
    }

    #[test]
    fn parse_rejects_missing_data() {
        let body = serde_json::to_vec(&json!({ "error": { "message": "nope" } })).unwrap();
        assert!(parse_models_response("openai", &body).is_err());
    }

    #[test]
    fn chat_test_url_by_endpoint() {
        assert_eq!(
            build_chat_test_url("https://api.openai.com/v1", "openai").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_chat_test_url("https://api.anthropic.com", "anthropic").unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
        assert!(build_chat_test_url("", "openai").is_err());
        assert!(build_chat_test_url("https://x", "zenmux").is_err());
    }

    #[test]
    fn chat_test_body_is_minimal_no_system_no_tools() {
        let body = build_chat_test_body("gpt-4o");
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["max_tokens"], TEST_MAX_TOKENS);
        assert!(body.get("system").is_none(), "不得携带 system 提示词");
        assert!(body.get("tools").is_none(), "不得携带 tools 定义");
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], TEST_PROMPT);
    }

    #[test]
    fn chat_test_reply_parsing() {
        let openai = serde_json::to_vec(&json!({
            "choices": [{ "message": { "role": "assistant", "content": "你好！有什么可以帮你？" } }]
        }))
        .unwrap();
        assert_eq!(
            parse_chat_test_reply("openai", &openai).unwrap(),
            "你好！有什么可以帮你？"
        );

        let anthropic = serde_json::to_vec(&json!({
            "content": [
                { "type": "text", "text": "你好" },
                { "type": "text", "text": "！" }
            ]
        }))
        .unwrap();
        assert_eq!(
            parse_chat_test_reply("anthropic", &anthropic).unwrap(),
            "你好！"
        );

        // content 为 null / 缺字段 → 空串而非报错。
        let null_content =
            serde_json::to_vec(&json!({ "choices": [{ "message": { "content": null } }] }))
                .unwrap();
        assert_eq!(parse_chat_test_reply("openai", &null_content).unwrap(), "");
    }
}
