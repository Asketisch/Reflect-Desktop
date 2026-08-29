# Coding Plan 与额度耗尽自动切换

本文档描述桌面端的 coding plan（编码计划订阅）配置体系、默认供应商切换、
额度耗尽的检测与自动 failover。对标 opencode / Codex Desktop / ZCode /
MiniMax Code / Reasionx 等第一梯队 harness 桌面端时，这是 GUI 层体验的
关键差异点之一：**多计划可配置、默认供应商可选、耗尽自动接管**。

## 1. 概念映射（config ↔ 运行时）

配置载体仍是 `~/.reflect/config.toml`（`ReflectConfig`），不新增配置文件：

| 概念 | 配置 | 运行时 |
| --- | --- | --- |
| Coding Plan（计划） | `[[<provider>.credentials]]` 条目：`label` / `api_key` / `base_url` / `model` / `weight` / `[quota]` | `CredentialPool` 池条目（`reflect-config::builder::to_registry`） |
| 顶层隐式计划 | `[<provider>].api_key`（label 固定 `default`） | 同上，包装为单条目池 |
| 默认供应商 | `[active].provider` | `active_provider()` 决定 model spec 与主池 |
| 配额声明 | `[<provider>.credentials.quota]`：`window_secs` / `max_tokens` / `check_via` | `QuotaTracker`（`src-tauri/src/state/quota.rs` 构建） |

`check_via` 支持厂商用量 API：`kimi` / `zhipu` / `minimax` / `zenmux`
（火山 / Anthropic / OpenAI usage 暂未实现，退化为本地 token 统计）。

### 配置示例

```toml
[active]
provider = "anthropic"        # 默认供应商

[anthropic]
model = "claude-sonnet-4"

# 计划 A：GLM Coding Plan（anthropic 兼容端点 + 官方额度查询）
[[anthropic.credentials]]
label = "glm-coding-plan"
api_key = "sk-..."
base_url = "https://open.bigmodel.cn/api/anthropic"
model = "glm-4.7"
quota = { window_secs = 2592000, max_tokens = 120000000, check_via = "zhipu" }

# 计划 B：Kimi（官方额度查询）
[[anthropic.credentials]]
label = "kimi-coding-plan"
api_key = "sk-..."
base_url = "https://api.moonshot.cn/anthropic"
quota = { window_secs = 2592000, max_tokens = 60000000, check_via = "kimi" }

# 备用供应商（跨供应商 failover 的目标）
[openai]
api_key = "sk-..."
```

## 2. 故障切换的两层结构

1. **同供应商池内切换（上游 graph 层自动）**：`reflect-core` 的
   `model_call` 节点对 401 / 429 / 5xx / 网络错误做 `RetrySame` /
   `CooldownAndFailover` / `Failover` 分类，在凭证池内轮转；每次切换发
   `routing` 事件，最终放弃时发 `error`（`ALL_CREDENTIALS_EXHAUSTED` 等）。
2. **跨供应商计划切换（桌面端前端补齐）**：`src/stores/agent/planFailover.ts`
   的控制器监听 `quota_exhausted` 与耗尽特征的 `error` 事件 →
   `pickFailoverProvider` 挑选下一个有可用计划的供应商 →
   `reflect_save_config` 写回 `[active].provider` → 后端热重载 →
   toast 告知。开关（默认开）与防抖（60s）见该模块；偏好键
   `reflect.plans.autoFailover`（localStorage）。

已知边界：不匹配耗尽特征的 4xx 错误（上游分类为 `GiveUp`）不会触发
跨供应商切换；自动切换后已失败的 turn 不自动重发，用户重发即可走新
供应商。

## 3. 热生效链路（`state/reload.rs`）

`reflect_save_config` 在 provider 相关段（`active` / `anthropic` /
`openai` / `ollama` / `routing`）变化时：

1. 重建 `ModelRegistry`（新凭证池）+ `QuotaTracker`，热替换
   `MinimalAgentInner` 中的句柄；
2. 同步 `model_spec` 与降级原因；
3. 若有绑定会话 → `replay_for_preload`（与 `reflect_bind_session` 同源）
   → `rebind_session_forced`，当前会话换到新凭证栈且上下文保留。

其余段落（MCP、hooks 等）仍只热更新 cfg，由各自机制接管。

## 4. UI 入口

- **Settings → 编码计划**（`sections/PlansSection.tsx`）：计划卡片列表
  （增删改、密钥遮罩、额度徽标）、默认供应商一键设置、自动切换开关；
  与 ConfigForm 共享 rawToml buffer，统一 Save 落盘。
- **Models 页**（`features/models/ModelsView.tsx`）：编码计划列表 +
  「设为默认」一键切换。
- 状态提示：额度耗尽 toast（reducer）、failover 成功/失败 toast
  （planFailover 控制器）、`routing` 切换指示（`lastRouting`）。
- 纯函数层：`src/features/settings/config/plans.ts`（解析 / 增删 /
  切换 / failover 挑选），后端 `load_from_str` 是最终校验闸阀。

## 5. 测试

- Rust：`state/quota.rs`（tracker 构建/注册）、`state/reload.rs`
  （变更检测 / registry 重建 / 强制重绑）、`state/rebind.rs`（forced 变体）。
- 前端：`config/plans.test.ts`、`stores/agent/planFailover.test.ts`、
  `sections/PlansSection.test.tsx`。

## 6. 对标定位与后续方向（UX 优先）

已对齐第一梯队：多计划配置入口、默认供应商快速切换、计划耗尽自动接管 +
用户通知、切换无需重启。

后续候选（按 UX 影响排序，均不阻塞当前功能）：

1. 切换后自动重发失败 turn（需防重复计费护栏）；
2. 各计划的剩余额度可视化（配额查询 API 已接 `QuotaTracker`，缺 UI 轮询）；
3. `/provider <name>` slash 直接切换（现指向 UI 入口）；
4. 上游增加 4xx 耗尽文本分类（"insufficient balance" 等），把跨供应商
   切换下沉到 graph 层（需改 Reflect-Agent 仓库后升级 submodule）。
