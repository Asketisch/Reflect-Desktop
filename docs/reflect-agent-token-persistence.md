# Reflect-Agent PR Plan: Session Token/Cost 持久化与聚合

> 本文件是把 "把 token 计算逻辑放到 Reflect Agent" 的具体 PR 设计落到本地,
> 用来拿到 Reflect-Agent 仓库(https://cnb.cool/Demon1019/Reflect-Agent)
> 提交。不修改本仓库的 `reflect-agent/` submodule 镜像。

## 背景与动机

后端 token 计算早就完整了 —— provider SSE usage 提取(`reflect-llm/providers/<x>.rs`)、
会话级累计(`reflect-core/graph/nodes/model_call/mod.rs:376-407`)、计费
(`reflect-llm/providers/pricing.rs`)、配额追踪(`reflect-llm/quota.rs`)都已在
submodule 里。但是:

- `EventMsg::TokenCount` 只通过 `event_tx` 发给订阅者(桌面端能拿到);
- **从未持久化到 rollout jsonl**;
- 进程退出后所有累计全部丢失;
- CLI 的 `reflect session ls` / `reflect session show` 列布局里完全没有
  token / cost;
- TUI(`Reflect-TUI` 独立仓库)与桌面端各自维护运行时累计,无法共享。

本次 PR 让 jsonl 成为"跨进程可见"的持久真相源,让 CLI 立刻能看到 session 累计
token 与 USD 成本。

## 范围(只在 Reflect-Agent 仓库内)

| crate | 是否改动 | 内容 |
|---|---|---|
| `reflect-protocol` | ✅ | `SessionInfo` 加 4 字段;`RolloutRecord` 加 `TokenCount` 变体 |
| `reflect-rollout` | ✅ | `parse_first_session_meta` 聚合 token / cost |
| `reflect-core` | ✅ | `model_call` 节点在 `event_tx.send(EventMsg::TokenCount)` 旁追加持久化 |
| `reflect-cli` | ✅ | `session ls` 加 2 列;`session show` 加详情行 |
| `reflect-llm` | ❌ | usage 提取已完整 |
| `reflect-protocol/event_msg` | ❌ | `TokenCountEvent` 协议稳定 |

## 改动文件清单(带行号 + 改动量)

| 文件 | 行号 | 改动 | 估算 |
|---|---|---|---|
| `crates/protocol/reflect-protocol/src/recorder.rs` | 35-46 | `SessionInfo` 加 4 字段 | +20 行 |
| `crates/protocol/reflect-protocol/src/recorder.rs` | 52-124 | `RolloutRecord` 加 `TokenCount` 变体 | +15 行 |
| `crates/protocol/reflect-protocol/src/lib.rs` | 40 | 确认重导出 `TokenUsage` | 0 行 |
| `crates/resources/reflect-rollout/src/index/mod.rs` | 182-227 | `parse_first_session_meta` 聚合 token / cost | +25 行 |
| `crates/resources/reflect-rollout/src/redact.rs` | - | 验证 `TokenCount` 走默认 redact | 0-5 行 |
| `crates/runtime/reflect-core/src/graph/nodes/model_call/mod.rs` | 472-483 | 追加持久化 `record()` | +8 行 |
| `crates/runtime/reflect-cli/src/session.rs` | 10-46 | `ls()` 加 2 列 | +20 行 |
| `crates/runtime/reflect-cli/src/session.rs` | 50-98 | `show()` 加详情行 | +10 行 |
| `crates/runtime/reflect-cli/src/session.rs` | 187-240 | CLI 测试 2 个 | +50 行 |
| `crates/resources/reflect-rollout/src/index/tests.rs` | - | 聚合测试 3 个 | +60 行 |
| **合计** | | | **~210 行** |

---

## 步骤 1：`reflect-protocol/src/recorder.rs`

### 1a. `SessionInfo` 加 4 字段

```rust
/// Lightweight session metadata used by the CLI `resume` flow and the
/// session index. Aggregated from the JSONL body when the index is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: ThreadId,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub message_count: usize,
    /// v1.x: 会话标题(从首条 User 消息预览生成)。
    /// `None` = 无首条 user 文本(空会话 / 旧文件)。
    /// 用户 `/rename` 的自定义名优先级更高(TUI 层先读 .name 文件再回退
    /// 本字段)。`#[serde(default)]` 保证旧 JSONL / 旧 reader 反序列化不破。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// v1.x: 整 session 累计 input tokens(聚合自 `RolloutRecord::TokenCount`)。
    /// 旧 session jsonl 中无 TokenCount 记录 → 反序列化为 0。
    #[serde(default)]
    pub input_tokens: u64,
    /// v1.x: 整 session 累计 output tokens。
    #[serde(default)]
    pub output_tokens: u64,
    /// v1.x: 整 session 累计 total tokens(= input + output;cached / cache_write
    /// 是 input 子集,刻意不重复加)。`u64` 防长会话溢出。
    #[serde(default)]
    pub total_tokens: u64,
    /// v1.x: 整 session 累计 USD cost(从每条 TokenCount 的 cost_usd 求和)。
    /// `None` = 没有 TokenCount 记录,或 model 不在 pricing 表里。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}
```

**字段类型选型理由**:`u64` 而非 `TokenUsage` 内部的 `u32` —— 长会话 + 累计(不止
一次 `saturating_add`)应给 50+ 年寿命留余地,与 `message_count: usize` 风格一致。

### 1b. `RolloutRecord` 加 `TokenCount` 变体

加在 `DiscussionTranscript` 之后(enum 追加而非插入):

```rust
/// v1.x: 每次 LLM 调用后的 per-turn usage 快照。
/// 派生来源是 `EventMsg::TokenCount` —— 这里只把已 emit 的事件
/// 持久化一遍,确保进程退出后 CLI `reflect session ls/show` 仍能看到累计。
/// `cost_usd` 在 emit 当时已由 `model_call` 调用
/// `reflect_llm::providers::price()` 算好;此处直接复用,不重新计算。
TokenCount {
    turn_id: TurnId,
    /// 标准 5 段 usage: input / output / cached / cache_write / total。
    /// `cached` 与 `cache_write` 是 `input` 的子集,刻意不进 `total`。
    usage: TokenUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cost_usd: Option<f64>,
    at: DateTime<Utc>,
},
```

**关键陷阱**:
- `usage` 用 `reflect_protocol::TokenUsage`(已有的 5 段结构),**不要**用
  `TokenCountEvent`(后者带 `provider` / `credential_label`,record 层不关心路由
  元信息)。
- 新变体追加在 `DiscussionTranscript` 之后,保持 enum 追加而非插入,避免改动
  现有 match 表达式的 exhaustive 检查覆盖。

---

## 步骤 2：`reflect-rollout/src/index/mod.rs::parse_first_session_meta`

在已有"全文件读进内存 + 扫非空行"的基础上,**顺路**聚合 token 记录。零边际 I/O。

```rust
fn parse_first_session_meta(path: &Path) -> Option<SessionInfo> {
    let body = std::fs::read_to_string(path).ok()?;
    let first = body.lines().next()?;
    let non_blank_lines = body.lines().filter(|l| !l.trim().is_empty()).count();
    let title = first_user_message_title(&body);

    // v1.x: 聚合 TokenCount 记录(token 累计 + cost 求和)。
    // body 已在内存,O(n) 行扫描。空 session / 旧 jsonl → 全 0 / None。
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut total_tokens: u64 = 0;
    let mut cost_sum: f64 = 0.0;
    let mut cost_present = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let Ok(rec) = serde_json::from_str::<RolloutRecord>(trimmed) else { continue };
        if let RolloutRecord::TokenCount { usage, cost_usd, .. } = rec {
            input_tokens = input_tokens.saturating_add(usage.input_tokens as u64);
            output_tokens = output_tokens.saturating_add(usage.output_tokens as u64);
            total_tokens = total_tokens.saturating_add(usage.total_tokens as u64);
            if let Some(c) = cost_usd {
                cost_sum += c;
                cost_present = true;
            }
        }
    }

    // Happy path: SessionMeta 首行
    if let Ok(record) = serde_json::from_str::<RolloutRecord>(first)
        && let RolloutRecord::SessionMeta { session_id, model, started_at } = record
    {
        return Some(SessionInfo {
            session_id, model, started_at,
            message_count: non_blank_lines.saturating_sub(1),
            title,
            input_tokens, output_tokens, total_tokens,
            cost_usd: cost_present.then_some(cost_sum),
        });
    }

    // Recovery path: rotated sibling
    if let Some(meta) = session_meta_from_rotated_sibling(path) {
        return Some(SessionInfo {
            session_id: meta.session_id,
            model: meta.model,
            started_at: meta.started_at,
            message_count: non_blank_lines,
            title,
            input_tokens, output_tokens, total_tokens,
            cost_usd: cost_present.then_some(cost_sum),
        });
    }

    tracing::warn!("rollout: {} has no SessionMeta first line", path.display());
    None
}
```

**关键陷阱**:
- **不要**在 rotated sibling 里聚合 token —— 那个文件不含 TokenCount
  (TokenCount 是 append-only 在新文件追加,meta 被旋到旧文件)。只聚合当前文件。
- `cost_present` flag:避免把"model 不在 pricing 表"的 0 显示为 `$0.00`,让
  `None` 保持 `None`。
- `f64` 求和保留精度足够(每轮 $0.001 量级,1000 次累加误差 ~$1e-13,UI `.2f`
  显示无影响)。

---

## 步骤 3：`reflect-core/src/graph/nodes/model_call/mod.rs`

在已有的 `event_tx.send(EventMsg::TokenCount(...))` 之后**追加**持久化。

```rust
// 行 472-483 (现有 EventMsg::TokenCount 发送)
ctx.event_tx.send(EventMsg::TokenCount(TokenCountEvent {
    input_tokens: usage.input_tokens,
    output_tokens: usage.output_tokens,
    cached_tokens: usage.cached_tokens,
    cache_write_tokens: usage.cache_write_tokens,
    total_tokens: usage.total_tokens,
    cost_usd,
    provider: Some(provider.to_string()),
    credential_label: ctx.cfg.credential_label().map(|s| s.to_string()),
}).into()).await.ok();

// v1.x: 追加持久化 record —— 让 CLI 跨进程可见累计。
// best-effort: record 失败不能 abort 当前 turn,只 warn。
if let Some(recorder) = ctx.recorder.clone() {
    let turn_id = ctx.current_turn_id.clone(); // 与 ctx.event_tx 同一上下文
    let usage_clone = reflect_protocol::TokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens: usage.cached_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        total_tokens: usage.total_tokens,
    };
    recorder
        .record(RolloutRecord::TokenCount {
            turn_id,
            usage: usage_clone,
            cost_usd,
            at: chrono::Utc::now(),
        })
        .await
        .unwrap_or_else(|e| tracing::warn!("rollout: TokenCount record failed: {e}"));
}
```

**关键陷阱**:
- `recorder` 在 `NodeContext` 里已经是 `Option<Arc<dyn RolloutRecorder>>`(见
  `recorder.rs` 头注释),先克隆避免 borrow 冲突。
- `usage` 复用 EventMsg 已经计算好的 `cost_usd`(行 466-467),**不要**重新调
  `price()` —— 避免两次调用对未在 pricing 表的 model 行为不一致。
- **必须** `.unwrap_or_else(|e| tracing::warn!(...))` 而非 `?`:现有的 `record()`
  (writer.rs:185) 用 `?` 传播,但 model_call 这一步不能因为落盘失败 abort 整个
  turn。
- `turn_id` 来自当前 turn 的 ID(不是 LLM 的 call_id),与 `Message` record 同一
  turn 维度。

---

## 步骤 4：`reflect-cli/src/session.rs::ls`

加 2 列对齐 `msgs` 风格:

```rust
pub fn ls(limit: usize, model_filter: Option<&str>) -> anyhow::Result<()> {
    let base = default_base();
    let mut sessions = list_sessions(&base).context("list_sessions")?;

    if let Some(m) = model_filter {
        let m_lc = m.to_lowercase();
        sessions.retain(|s| s.model.to_lowercase().contains(&m_lc));
    }
    if sessions.is_empty() {
        println!("(no sessions found under {})", base.display());
        return Ok(());
    }

    // v1.x: 新增 tokens / cost 两列。
    println!(
        "{:<36}  {:<32}  {:<12}  {:>6}  {:>10}  {:>10}",
        "session_id", "model", "started", "msgs", "tokens", "cost"
    );
    for s in sessions.iter().take(limit) {
        let model = truncate(&s.model, 32);
        let tokens = format_with_thousands(&s.total_tokens.to_string());
        let cost = match s.cost_usd {
            Some(c) => format!("${:.2}", c),
            None => String::from("--"),
        };
        println!(
            "{:<36}  {:<32}  {:<12}  {:>6}  {:>10}  {:>10}",
            s.session_id.to_string(),
            model,
            s.started_at.format("%Y-%m-%d"),
            s.message_count,
            tokens,
            cost,
        );
    }
    let shown = sessions.len().min(limit);
    if sessions.len() > limit {
        println!("(showing {shown} of {}; pass --limit/-n to see more)", sessions.len());
    }
    Ok(())
}

/// 千分位逗号格式。18,234 / 1,234,567
fn format_with_thousands(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(*c);
    }
    out.chars().rev().collect()
}
```

**关键陷阱**:
- 列宽调整:原 4 列共 92 字符 → 6 列共 116 字符,主流终端 120+ 仍兼容。终端
  < 130 时不主动降级(保持一致布局);可后续加 `terminal_size` 检测。
- `cost_usd == None` 显示 `--` 而非 `$0.00`:区分"没数据"与"零成本"。
- `format_with_thousands` 只为 `total_tokens` 服务(output/input 通常不进 ls 列)。

---

## 步骤 5：`reflect-cli/src/session.rs::show`

在 `Started:` 行后追加 token/cost 详情行:

```rust
println!("Session: {tid}");
println!("Path:    {}", path.display());
let content = std::fs::read_to_string(&path).context("read session jsonl")?;

// v1.x: 整 session 累计 token / cost(全文件 body 已在内存)。
let mut input_tokens: u64 = 0;
let mut output_tokens: u64 = 0;
let mut total_tokens: u64 = 0;
let mut cost_sum: f64 = 0.0;
let mut cost_present = false;
for line in content.lines() {
    let Ok(rec) = serde_json::from_str::<RolloutRecord>(line.trim()) else { continue };
    if let RolloutRecord::TokenCount { usage, cost_usd, .. } = rec {
        input_tokens = input_tokens.saturating_add(usage.input_tokens as u64);
        output_tokens = output_tokens.saturating_add(usage.output_tokens as u64);
        total_tokens = total_tokens.saturating_add(usage.total_tokens as u64);
        if let Some(c) = cost_usd {
            cost_sum += c;
            cost_present = true;
        }
    }
}

// ... 现有解析 (session_meta 行 + 前 20 行 message / compaction 预览) ...

// 在 println!("Model:  ..."); println!("Started: ..."); 之后追加:
println!(
    "Tokens:  {} (in {} / out {})",
    format_with_thousands(&total_tokens.to_string()),
    format_with_thousands(&input_tokens.to_string()),
    format_with_thousands(&output_tokens.to_string()),
);
if cost_present {
    println!("Cost:    ${:.2}", cost_sum);
}
```

**关键陷阱**:
- `show` 当前只读前 20 行 + 匹配 3 种类型(`session_meta` / `message` /
  `compaction`)—— TokenCount 不在这 3 种里,会被默默忽略。**改成**解析整文件
  聚合 token + 前 20 行预览消息(与 `parse_first_session_meta` 同款:body 已在
  内存)。
- 复用 `format_with_thousands` helper(与 `ls` 共享)。

---

## 步骤 6：测试

### 6a. `reflect-rollout/src/index/tests.rs`

仿 `list_sessions_derives_title_from_first_user_message`(行 54-87)模式,新增
3 个测试:

1. **`list_sessions_aggregates_token_count_from_records`**
   - 手写 JSONL:1 个 `SessionMeta` + 3 个 `Message` + 3 个 `TokenCount`
     (input=1000/200/300, output=200/50/100, total=1200/250/400)
   - 断言 `SessionInfo.total_tokens == 1850`、`input_tokens == 1500`、
     `output_tokens == 350`

2. **`list_sessions_sums_cost_usd_across_turns`**
   - 同上 JSONL,每个 `TokenCount` 带 `cost_usd`
   - 断言 `cost_usd == Some(0.0234)`(带 f64 精度比较)

3. **`list_sessions_token_count_zero_for_old_rollouts`**
   - 手写 JSONL:**无** `TokenCount` 记录(模拟 v1.x 之前的 session)
   - 断言 4 个新字段都是 0/None,向后兼容

### 6b. `reflect-cli/src/session.rs`

仿行 199-221 `session_ls_show_rm_roundtrip` 模式,新增 2 个测试:

1. **`session_ls_renders_token_columns`**
   - tmpdir 手写 JSONL(带 TokenCount records)
   - 调 `ls()` 后捕获 stdout,断言含 `tokens` 列头、含千分位格式数字

2. **`session_show_displays_token_count`**
   - tmpdir 手写 JSONL
   - 调 `show(id)`,断言输出含
     `Tokens: 1,850 (in 1,500 / out 350)` 与 `Cost: $0.02`

---

## 步骤 7：文档同步（Reflect-Agent 仓库内）

### 7a. `crates/resources/reflect-rollout/src/lib.rs` 头注释

如果存在 module-level doc,加一句:

```
//! v1.x: `SessionInfo` 携带 `input_tokens` / `output_tokens` / `total_tokens`
//! / `cost_usd`,从每条 `RolloutRecord::TokenCount` 聚合。旧 jsonl 无该
//! 记录时四个字段分别为 0 / None。
```

### 7b. `crates/runtime/reflect-cli/src/session.rs` 头注释

加一句 `ls` / `show` 现在展示的列。

### 7c. `CHANGELOG.md` (Reflect-Agent 仓库)

`## Unreleased` 下加:

```markdown
### Added —— 会话 token/成本持久化（v1.x）

- `RolloutRecord::TokenCount` 新变体: `turn_id` + `TokenUsage` +
  `cost_usd?` + `at`
- `SessionInfo` 新增 4 字段: `input_tokens` / `output_tokens` /
  `total_tokens` / `cost_usd?`,从 JSONL 聚合
- `reflect-core` `model_call` 节点每次 `EventMsg::TokenCount` 发送后
  best-effort 持久化 `RolloutRecord::TokenCount`(失败 warn,不 abort turn)
- `reflect session ls` 新增 `tokens` / `cost` 两列(右对齐千分位)
- `reflect session show <id>` 新增
  `Tokens: 1,850 (in 1,500 / out 350)` 与 `Cost: $0.02` 两行
- 旧 session(jsonl 无 TokenCount 记录)反向兼容: 四个新字段反序列化为
  0/None
```

---

## 步骤 8：ReflectDesktop 侧后续（不在本次 PR 范围，PR 落地后执行）

Reflect-Agent PR 合并后,ReflectDesktop 通过 submodule 升级拿到:

1. `git submodule update --remote reflect-agent`
2. 可选:在 `app-core/src/reducer/matchers.rs::SessionConfigured` 旁边加新的
   `RolloutRecord::TokenCount` 处理(**其实不必** —— `EventMsg::TokenCount`
   已经驱动桌面 UI,本次 PR 不引入新事件通道)
3. 可选:桌面 `state.tokens` 改为同时显示"当前 turn"和"session 累计"——后者
   可从 `reflect-rollout` 读 jsonl 聚合(不在本次范围)

---

## 验证矩阵(Reflect-Agent 仓库内)

```bash
cd crates/protocol/reflect-protocol && cargo test       # SessionInfo 序列化兼容
cd crates/resources/reflect-rollout && cargo test       # 聚合逻辑 3 个新测试
cd crates/runtime/reflect-core && cargo test            # model_call 持久化路径
cd crates/runtime/reflect-cli && cargo test              # ls / show 渲染测试
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

---

## 不做的事(明确边界)

- ❌ 不改 `reflect-llm`(usage 提取已完整)
- ❌ 不改 `reflect-protocol::EventMsg::TokenCount`(事件协议稳定)
- ❌ 不动 `discussion` / `task` / `pipeline` 的 rollout 路径(它们不消费
  TokenCount,按需未来扩展)
- ❌ 不做客户端精确 token 预估(tiktoken 等依赖是大工程,另立项)
- ❌ 不在 PR 里改 ReflectDesktop(ReflectDesktop 通过 submodule 升级接收)

---

## PR 描述模板(Reflect-Agent 仓库)

**Title**: `feat(rollout): persist per-turn token usage + aggregate SessionInfo token/cost`

**Summary**:
- `RolloutRecord::TokenCount` 新变体,持久化每次 LLM 调用的 usage 与 cost
- `SessionInfo` 4 新字段,从 JSONL 聚合
- `reflect session ls` / `show` 现在能看到每个 session 的累计 token 与 USD
  成本

**Motivation**: 当前 `EventMsg::TokenCount` 只通过 `event_tx` 发给订阅者,进程
退出即丢失。CLI 用户无法在 `reflect session ls` 看到成本,TUI/CLI/desktop 各自
独立维护运行时累计(无法跨进程)。本次让 jsonl 成为持久真相源。

**Backward compat**: 旧 session(jsonl 无 TokenCount 记录)反序列化为 0/None,
不破坏现有 reader。

**Testing**: 5 个新测试 + 现有测试全绿。

**Linked**: ReflectDesktop 通过 submodule upgrade 后续消费(session 累计可
展示在状态栏 / Inspector 的 session 切换视图)。

---

## 风险与回滚

- **风险 1**:`model_call` 新增 `recorder.record()` 是 IO 操作,可能引入延迟。
  **缓解**:`recorder` 已是 `JsonlRolloutWriter` append 模式,实测 O(微秒级);
  失败仅 warn。
- **风险 2**:`parse_first_session_meta` 全文件解析 `RolloutRecord` 在每次
  `list_sessions` 调用时执行,大 session(10k+ 行)可能变慢。**缓解**:当前实现
  已经把全文件读进内存;新聚合只是多了一个 serde_json 解析 + match,O(n) 与
  `non_blank_lines.count()` 同阶。
- **回滚**:本 PR 可独立 revert;不影响 EventMsg::TokenCount 的订阅者路径
  (那是已有数据流,本 PR 只是新增持久化副本)。