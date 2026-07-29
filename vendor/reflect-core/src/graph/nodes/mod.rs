//! 4-node graph node implementations (M2/M3).
//!
//! Each function takes `&mut AgentState` plus shared context (registry,
//! hooks, tools, etc.) and returns the next node to run, or `None` if
//! the turn is done.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use reflect_llm::{ChatEvent, ChatMessage, ChatRequest, CooldownReason, LlmError, Role};
use reflect_protocol::{
    AbortReason, AgentMessageDelta, ContentBlock, ContextCompactedStrategy, ErrorEvent, Event,
    EventMsg, RolloutRecord, RoutingEvent, RoutingEventKind, StreamErrorEvent, TokenCountEvent,
    TokenUsage, ToolCallBeginEvent, ToolCallEndEvent, TriedCredential, TurnAbortedEvent,
    UserInputItem,
};
use tokio::time::sleep;

use reflect_hooks::HookEvent;
use reflect_hooks::StopReason;

use super::GraphNode;
use super::state::AgentState;
use crate::submission_loop::NodeContext;
use reflect_recovery::{MetaKind, RecoveryEntry, recovery_meta_to_messages};

/// `pre_loop` — M4: full pre-turn pipeline.
///
/// When `ctx.m4` is `Some` (production):
/// 1. Take the user's input from `ctx.messages` and load it into
///    `state.messages`.
/// 2. Call `compactor.compact` on the message list; if a strategy
///    fires, emit `ContextCompacted` and update `state.compact_triggered`.
/// 3. Load memory (project + user + session scopes) per
///    `active_agent_def.memory`; truncate to 8KB.
/// 4. Build the layered system prompt (core = agent def + memory;
///    ephemeral = tools + skills catalog + iteration reminder).
/// 5. Compute `state.effective_tools` (always_on ∪ active_skills).
/// 6. Stash the result back into `state.messages` for `model_call`.
///
/// When `ctx.m4` is `None` (tests / legacy): pass through. The
/// `state.messages.messages` is left empty; `model_call` falls back to
/// `ctx.messages` (preserved for backwards compatibility).
pub async fn pre_loop(state: &mut AgentState, ctx: &NodeContext) -> Option<GraphNode> {
    let Some(m4) = ctx.m4.as_ref() else {
        return Some(GraphNode::ModelCall);
    };

    // Review 2026-06-29 BUG-2 修复: `compact_triggered` 必须先重置再决策,
    // 否则 `AgentState` 跨 turn 持久化时,上一轮触发的 true 会泄漏到本轮,
    // 导致 active file meta-message 反复注入。
    state.compact_triggered = false;

    // 1. Pull user input from ctx.messages into state.messages.
    state.messages.messages.clear();
    for m in &ctx.messages {
        state.messages.messages.push(m.clone());
    }

    // 2. Compact. M8: thread the LLM-reported `input_tokens` from the prior
    // turn so it can take priority over the local `estimate_messages` heuristic
    // (see `Compactor::compact_with_prior_and_tokens`). On the first turn
    // `state.total_usage.input_tokens == 0`, so this is equivalent to `None`
    // and the local estimate is the sole signal.
    let llm_reported =
        (state.total_usage.input_tokens > 0).then_some(state.total_usage.input_tokens);
    // v1.2 P1-12(已有-B):`/compact` 手动触发 —— 读 + 清零
    // `force_compact_next`,若为 true 则把 `llm_reported` 强制成 `u32::MAX`
    // 让 `before_tokens` 超过任何阈值,compactor 必然运行(microcompact /
    // smart_prune)。这样 `Op::Compact` 设置的标志在本轮 pre_loop 生效。
    let force_compact = {
        let mut g = ctx.force_compact_next.write();
        let v = *g;
        *g = false;
        v
    };
    let llm_reported = if force_compact {
        Some(u32::MAX)
    } else {
        llm_reported
    };
    let (compacted, evt) = m4
        .compactor
        .compact_with_prior_and_tokens(
            state.messages.messages.clone(),
            state.compaction_summary.as_deref(),
            llm_reported,
        )
        .await;
    state.compact_triggered = !matches!(evt.strategy, ContextCompactedStrategy::Noop);
    if state.compact_triggered {
        // Capture fields we'll need after `evt` moves into the wire event.
        let strategy = evt.strategy;
        let removed_count = evt.removed_messages;

        // Pull the LLM-generated summary out of the compacted message list
        // (it's the System message with `<summary>...</summary>`).
        let summary_text = compacted.iter().find_map(|m| match m {
            ChatMessage::System(s) if s.contains("<summary>") => Some(s.clone()),
            _ => None,
        });
        if summary_text.is_some() {
            state.compaction_summary = summary_text.clone();
        }

        let _ = ctx
            .event_tx
            .send(Event::new(
                ctx.sub_id.clone(),
                EventMsg::ContextCompacted(evt),
            ))
            .await;

        // M5: persist the Compaction record so `resume` can replay it.
        if let Some(rec) = ctx.recorder.as_ref() {
            let strategy_str = match strategy {
                ContextCompactedStrategy::Noop => "noop",
                ContextCompactedStrategy::Microcompact => "microcompact",
                ContextCompactedStrategy::SmartPrune => "smart_prune",
                ContextCompactedStrategy::LlMSummarize => "llm_summarize",
            };
            let _ = rec
                .record(RolloutRecord::Compaction {
                    turn_id: ctx.turn_id,
                    strategy: strategy_str.into(),
                    removed_count,
                    summary: summary_text.unwrap_or_default(),
                })
                .await;
        }
    }
    state.messages.messages = compacted;

    // 3. Load memory per active_agent_def.memory scopes.
    let memory_text = match m4
        .memory
        .load_combined(&m4.active_agent_def.memory, &m4.active_agent_def.name)
    {
        Ok(s) => reflect_memory::truncate_for_injection(&s),
        Err(e) => {
            tracing::warn!(?e, "failed to load memory; using empty");
            String::new()
        }
    };

    // 4. Build the layered system prompt.
    let core = reflect_prompt::LayeredPrompt::compose_core(
        &m4.active_agent_def.system_prompt,
        &memory_text,
    );
    let ephemeral_skills = m4.skills.render_for_system_prompt();
    let reminder = format!("Iteration {}/{}", state.iteration, ctx.max_iterations);
    let effective = m4.skills.active_tool_names();
    let tool_specs: Vec<reflect_llm::ToolSpec> = ctx
        .tools_queue
        .registry()
        .list_specs()
        .into_iter()
        .filter_map(|s| match s {
            reflect_tools::ToolSpec::Function {
                name,
                description,
                parameters,
                ..
            } => {
                if effective.contains(name.as_str()) {
                    Some(reflect_llm::ToolSpec::Function {
                        name: name.clone(),
                        description: description.clone(),
                        parameters: parameters.clone(),
                    })
                } else {
                    None
                }
            }
        })
        .collect();
    let ephemeral =
        reflect_prompt::LayeredPrompt::compose_ephemeral(&tool_specs, &ephemeral_skills, &reminder);

    // Prepend core as a system block (cleaner than a System message
    // — providers like Anthropic prefer the `system` array for
    // cacheable content).
    state.system_blocks = reflect_llm::SystemBlocks(vec![reflect_llm::SystemBlock {
        text: core,
        cache_control: None,
        ephemeral: false,
    }]);

    // Ephemeral reminder — appended to the messages list as a User
    // block so it doesn't go through the cache.
    state.ephemeral_text = ephemeral;

    // 5. Save effective tools for model_call.
    state.effective_tools = tool_specs;

    // 6. v1.1.0 Phase 6 P0:收集上下文恢复元消息。
    //
    // 每轮从 `M4Deps` 的共享源重新计算(Notes 走 JSONL 落盘,
    // SubagentRegistry / FileRecovery 走 in-memory Arc),所以本字段
    // 不需跨 turn 持久化,`AgentState::default()` 已经给空 Vec。
    // 渲染在 `model_call` 完成,见下文 ephemeral push 之后的循环。
    //
    // Phase A 启用:SessionMemory。
    // Phase B 启用:ActiveFiles(post-compact)。
    // Phase C 启用:SubagentRegistry(每轮)。
    if let Some(text) = m4.note_store.as_meta_message() {
        state
            .recovery_meta
            .push(RecoveryEntry::new(MetaKind::SessionMemory, text));
    }
    // 6a. Active File Recovery:仅在 compact 触发后注入(pre_loop
    // 的 post-compact block),文件读取静默 skip 失败项。
    // Review 2026-06-29 BUG-5: 使用 `recover_with_deleted` 把 NotFound
    // 收集到 deleted, 在 meta 顶部提示 LLM。
    if state.compact_triggered {
        let (recovered, deleted) = m4
            .file_recovery
            .recover_with_deleted(&state.messages.messages);
        if !recovered.is_empty() || !deleted.is_empty() {
            let content = m4
                .file_recovery
                .to_meta_message_with_deleted(&recovered, &deleted);
            state
                .recovery_meta
                .push(RecoveryEntry::new(MetaKind::ActiveFiles, content));
        }
    }
    // 6b. Subagent Registry:每轮渲染(防止 LLM 重复 spawn)。
    if let Some(text) = m4.subagent_registry.as_meta_message() {
        state
            .recovery_meta
            .push(RecoveryEntry::new(MetaKind::SubagentRegistry, text));
    }

    Some(GraphNode::ModelCall)
}

/// `model_call` — call the LLM and stream its response. If the LLM emits
/// `ToolUseStart` events, return `Some(ToolExec)` so the queue runs them.
/// Otherwise return `Some(CheckStop)`.
pub async fn model_call(state: &mut AgentState, ctx: &NodeContext) -> Option<GraphNode> {
    state.iteration = state.iteration.saturating_add(1);
    if state.iteration > ctx.max_iterations {
        return None;
    }
    // v0.2.2: 在函数入口把 model spec 拍成 String 快照,后续本轮 LLM
    // 调用(含重试)都沿用这一份 —— 重试本来就是同一请求的重复,
    // 不应受中途热重载影响;若用户在两次 LLM 调用之间改 model,
    // 下一次进 `model_call` 时会自然读到新值。
    let model = ctx.model.read().clone();
    // v1.0 多 Provider 路由:从 `ctx.policy` 拿主 role slot 的 primary
    // 作 spec 起点,失败时由 `ModelRegistry::next_for` 在 pool 内自动
    // 切下一个 credential。`spec` 为空时回退 `model`(向后兼容)。
    let initial_spec = ctx.policy.resolve(Role::Main).primary.clone();
    let spec = if initial_spec.is_empty() {
        model.clone()
    } else {
        initial_spec
    };

    // Build a ChatRequest. M4: when state.messages was populated by
    // pre_loop, use it; otherwise fall back to ctx.messages (legacy /
    // test path).
    let mut messages = if !state.messages.messages.is_empty() {
        state.messages.messages.clone()
    } else {
        ctx.messages.clone()
    };
    // M4: append the ephemeral reminder as a trailing User message.
    if !state.ephemeral_text.trim().is_empty() {
        messages.push(ChatMessage::User(reflect_llm::UserContent {
            blocks: vec![reflect_llm::ContentBlock::Text {
                text: format!(
                    "<system-reminder>\n{}\n</system-reminder>",
                    state.ephemeral_text
                ),
            }],
        }));
    }
    // v1.1.0 Phase 6 P0:把 `pre_loop` 收集的 `recovery_meta` 渲染成
    // `<system-reminder>` User 块追加到消息流(按 `MetaKind` canonical
    // 顺序 + 单实例去重)。`recovery_meta_to_messages` 内部已用
    // `format!("<system-reminder>...")`,此处直接 extend 即可。
    messages.extend(recovery_meta_to_messages(&state.recovery_meta));
    let tools: Vec<reflect_llm::ToolSpec> = if !state.effective_tools.is_empty() {
        state.effective_tools.clone()
    } else {
        ctx.tools_queue
            .registry()
            .list_specs()
            .into_iter()
            .map(|s| match s {
                reflect_tools::ToolSpec::Function {
                    name,
                    description,
                    parameters,
                    ..
                } => reflect_llm::ToolSpec::Function {
                    name: name.clone(),
                    description: description.clone(),
                    parameters: parameters.clone(),
                },
            })
            .collect()
    };
    let mut request = ChatRequest {
        model: spec.clone(),
        messages,
        tools,
        system: state.system_blocks.clone(),
        // v1.x S4:按当前 effort 构造 `thinking` 字段。Provider 端
        // (openai.rs) 看到 `OpenAIReasoning { effort }` 时把它映射到
        // OpenAI 的 `reasoning_effort` request body;Anthropic 端忽略此
        // 字段,改走 server-side thinking(由 model_call 后面的
        // capabilities 分支决定)。`From<ReasoningEffortMirror>` 在
        // `reflect-llm::request` 定义。
        thinking: Some(reflect_llm::ThinkingConfig::OpenAIReasoning {
            effort: (*ctx.effort.read()).into(),
        }),
        ..Default::default()
    };

    // M4: inject cache_control breakpoints if the provider supports it
    // and an M4 PromptBuilder is configured.
    if let Some(m4) = ctx.m4.as_ref() {
        let caps = ctx
            .registry
            .resolve(&spec)
            .map(|c| c.capabilities())
            .unwrap_or_default();
        let mut builder = m4.prompt_builder.lock();
        let _ = builder.build_request(
            &reflect_prompt::LayeredPrompt::new(),
            request.messages.clone(),
            request.tools.clone(),
            request.model.clone(),
            &caps,
        );
        // Apply cache_control to the actual request.
        if caps.prompt_caching {
            let _ = reflect_prompt::inject_cache_control(&mut request, &caps);
        }
    }

    // v1.0 多 Provider 路由:池轮询 retry loop。`exclude` 记录本轮已
    // 试过的 client Arc 指针,`tried` 记录诊断信息,`per_cred_attempts`
    // 限制同 credential 至多重试 2 次(瞬时 5xx 1 次,网络 1 次)。
    let mut exclude: Vec<Arc<dyn reflect_llm::ModelClient>> = Vec::new();
    let mut tried: Vec<TriedCredential> = Vec::new();
    let mut per_cred_attempts: HashMap<usize, u32> = HashMap::new();
    let mut attempt: u32 = 0;
    let max_attempts = ctx.policy.max_attempts;
    let default_cooldown_rate_limited = ctx.policy.default_cooldown_rate_limited;

    let (text_buf, tool_calls, _usage, success_provider, success_label) = loop {
        let next_client = match ctx.registry.next_for(&spec, &exclude) {
            Some(nc) => nc,
            None => {
                failed(tried.clone(), ctx, "ALL_CREDENTIALS_EXHAUSTED").await;
                return None;
            }
        };
        let client: Arc<dyn reflect_llm::ModelClient> = next_client.client.clone();
        let label: String = next_client.label.clone();
        let provider_name: String = client.name().to_string();
        let cred_ptr = Arc::as_ptr(&client) as *const () as usize;

        attempt += 1;
        if attempt > max_attempts {
            tried.push(TriedCredential {
                label: label.clone(),
                outcome: "max_attempts".into(),
            });
            failed(tried.clone(), ctx, "MAX_ATTEMPTS").await;
            return None;
        }
        let cred_attempt = per_cred_attempts.entry(cred_ptr).or_insert(0);
        *cred_attempt += 1;

        // tracing span:role / spec / provider / credential / attempt
        let span = tracing::info_span!(
            "llm_call",
            role = "main",
            spec = %spec,
            provider = %provider_name,
            credential = %label,
            attempt = attempt,
        );
        let _enter = span.enter();

        let stream_result = client.stream(request.clone(), ctx.cancel.clone()).await;
        let mut s = match stream_result {
            Ok(s) => {
                ctx.registry.clear_cooldown(&provider_name, &label);
                std::pin::pin!(s)
            }
            Err(e) => {
                let action = classify_action(&e, default_cooldown_rate_limited);
                let outcome = outcome_code(&e).to_string();
                let cooldown_until_ms: Option<u64>;
                let retry_ms: u64;
                match action {
                    RetryAction::CooldownAndFailover { cooldown } => {
                        let reason = match &e {
                            LlmError::RateLimited { .. } => {
                                CooldownReason::RateLimited { retry_after_ms: 0 }
                            }
                            LlmError::Overloaded { .. } => CooldownReason::Overloaded,
                            LlmError::Provider { status, .. } => {
                                CooldownReason::Provider5xx { status: *status }
                            }
                            LlmError::Auth => CooldownReason::Auth,
                            _ => CooldownReason::Auth,
                        };
                        ctx.registry
                            .mark_cooldown(&provider_name, &label, cooldown, reason);
                        cooldown_until_ms = Some(cooldown.as_millis() as u64);
                        retry_ms = cooldown.as_millis() as u64;
                        let _ = ctx
                            .event_tx
                            .send(Event::new(
                                ctx.sub_id.clone(),
                                EventMsg::Routing(RoutingEvent {
                                    kind: RoutingEventKind::CooldownStarted,
                                    role: "main".into(),
                                    from_credential: Some(label.clone()),
                                    to_credential: None,
                                    reason: outcome.clone(),
                                    cooldown_until_ms,
                                }),
                            ))
                            .await;
                    }
                    RetryAction::Failover => {
                        cooldown_until_ms = None;
                        retry_ms = 0;
                    }
                    RetryAction::RetrySame { delay_ms } => {
                        cooldown_until_ms = None;
                        retry_ms = delay_ms;
                        if *cred_attempt >= 2 {
                            let _ = ctx
                                .event_tx
                                .send(Event::new(
                                    ctx.sub_id.clone(),
                                    EventMsg::Routing(RoutingEvent {
                                        kind: RoutingEventKind::Switched,
                                        role: "main".into(),
                                        from_credential: Some(label.clone()),
                                        to_credential: None,
                                        reason: "retry_same_exhausted".into(),
                                        cooldown_until_ms: None,
                                    }),
                                ))
                                .await;
                        } else {
                            sleep(Duration::from_millis(delay_ms)).await;
                        }
                    }
                    RetryAction::GiveUp => {
                        tried.push(TriedCredential {
                            label: label.clone(),
                            outcome: outcome.clone(),
                        });
                        let _ = ctx
                            .event_tx
                            .send(Event::new(
                                ctx.sub_id.clone(),
                                EventMsg::Error(ErrorEvent {
                                    code: error_code(&e).into(),
                                    message: e.to_string(),
                                    details: Some(serde_json::json!({
                                        "provider": provider_name,
                                        "credential_label": label,
                                        "tried": tried,
                                    })),
                                }),
                            ))
                            .await;
                        return None;
                    }
                }
                tried.push(TriedCredential {
                    label: label.clone(),
                    outcome: outcome.clone(),
                });
                let _ = ctx
                    .event_tx
                    .send(Event::new(
                        ctx.sub_id.clone(),
                        EventMsg::StreamError(StreamErrorEvent {
                            code: error_code(&e).into(),
                            message: e.to_string(),
                            retry_in_ms: retry_ms,
                            provider: Some(provider_name.clone()),
                            credential_label: Some(label.clone()),
                            tried: Some(tried.clone()),
                        }),
                    ))
                    .await;
                exclude.push(client.clone());
                if !matches!(action, RetryAction::RetrySame { .. }) {
                    let _ = ctx
                        .event_tx
                        .send(Event::new(
                            ctx.sub_id.clone(),
                            EventMsg::Routing(RoutingEvent {
                                kind: RoutingEventKind::Switched,
                                role: "main".into(),
                                from_credential: Some(label.clone()),
                                to_credential: None,
                                reason: outcome.clone(),
                                cooldown_until_ms,
                            }),
                        ))
                        .await;
                }
                continue;
            }
        };

        let mut text_buf = String::new();
        let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();
        let mut current_tool: Option<(String, String)> = None;
        let mut current_args_str = String::new();
        let mut usage = TokenUsage::default();
        let mut stop = false;
        let mut error: Option<LlmError> = None;

        loop {
            tokio::select! {
                biased;
                _ = ctx.cancel.cancelled() => {
                    let _ = ctx.event_tx.send(Event::new(
                        ctx.sub_id.clone(),
                        EventMsg::TurnAborted(TurnAbortedEvent {
                            turn_id: ctx.turn_id,
                            reason: AbortReason::UserInterrupt,
                        }),
                    )).await;
                    return None;
                }
                evt = s.next() => {
                    match evt {
                        Some(Ok(ChatEvent::ContentDelta(d))) => {
                            text_buf.push_str(&d);
                            let _ = ctx.event_tx.send(Event::new(
                                ctx.sub_id.clone(),
                                EventMsg::AgentMessageDelta(AgentMessageDelta { delta: d }),
                            )).await;
                        }
                        Some(Ok(ChatEvent::ToolUseStart { id, name, .. })) => {
                            // Emit ToolCallBegin
                            let _ = ctx.event_tx.send(Event::new(
                                ctx.sub_id.clone(),
                                EventMsg::ToolCallBegin(ToolCallBeginEvent {
                                    call_id: id.clone(),
                                    tool_name: name.clone(),
                                    args: serde_json::Value::Null,
                                }),
                            )).await;
                            current_tool = Some((id, name));
                            current_args_str.clear();
                        }
                        Some(Ok(ChatEvent::ToolUseDelta(partial))) => {
                            current_args_str.push_str(&partial);
                        }
                        Some(Ok(ChatEvent::MessageStop)) | None => { stop = true; }
                        Some(Ok(ChatEvent::Usage {
                            input_tokens,
                            output_tokens,
                            cached_tokens,
                            cache_write_tokens,
                        })) => {
                            usage = TokenUsage {
                                input_tokens,
                                output_tokens,
                                cached_tokens,
                                cache_write_tokens,
                                total_tokens: input_tokens + output_tokens,
                            };
                        }
                        Some(Ok(ChatEvent::Error(e))) => { error = Some(e); stop = true; }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => { error = Some(e); stop = true; }
                    }
                    if stop { break; }
                }
            }
        }

        if let Some(e) = error {
            // 中流错误:也走 classify_action 决策。
            let action = classify_action(&e, default_cooldown_rate_limited);
            let outcome = outcome_code(&e).to_string();
            match action {
                RetryAction::CooldownAndFailover { cooldown } => {
                    let reason = match &e {
                        LlmError::RateLimited { .. } => {
                            CooldownReason::RateLimited { retry_after_ms: 0 }
                        }
                        LlmError::Overloaded { .. } => CooldownReason::Overloaded,
                        LlmError::Provider { status, .. } => {
                            CooldownReason::Provider5xx { status: *status }
                        }
                        LlmError::Auth => CooldownReason::Auth,
                        _ => CooldownReason::Auth,
                    };
                    ctx.registry
                        .mark_cooldown(&provider_name, &label, cooldown, reason);
                    let _ = ctx
                        .event_tx
                        .send(Event::new(
                            ctx.sub_id.clone(),
                            EventMsg::StreamError(StreamErrorEvent {
                                code: error_code(&e).into(),
                                message: e.to_string(),
                                retry_in_ms: cooldown.as_millis() as u64,
                                provider: Some(provider_name.clone()),
                                credential_label: Some(label.clone()),
                                tried: Some({
                                    let mut t = tried.clone();
                                    t.push(TriedCredential {
                                        label: label.clone(),
                                        outcome: outcome.clone(),
                                    });
                                    t
                                }),
                            }),
                        ))
                        .await;
                }
                RetryAction::RetrySame { delay_ms } => {
                    if *cred_attempt >= 2 {
                        exclude.push(client.clone());
                    } else {
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
                RetryAction::Failover => {
                    exclude.push(client.clone());
                }
                RetryAction::GiveUp => {
                    tried.push(TriedCredential {
                        label: label.clone(),
                        outcome: outcome.clone(),
                    });
                    let _ = ctx
                        .event_tx
                        .send(Event::new(
                            ctx.sub_id.clone(),
                            EventMsg::Error(ErrorEvent {
                                code: error_code(&e).into(),
                                message: e.to_string(),
                                details: Some(serde_json::json!({
                                    "provider": provider_name,
                                    "credential_label": label,
                                    "tried": tried,
                                })),
                            }),
                        ))
                        .await;
                    return None;
                }
            }
            tried.push(TriedCredential {
                label: label.clone(),
                outcome: outcome.clone(),
            });
            exclude.push(client.clone());
            continue;
        }

        // Flush any pending tool call.
        if let Some((id, name)) = current_tool.take() {
            let args: serde_json::Value =
                serde_json::from_str(&current_args_str).unwrap_or(serde_json::Value::Null);
            tool_calls.push((id, name, args));
        }
        break (text_buf, tool_calls, usage, provider_name, label);
    };

    // Build latest_content: text + tool_use blocks.
    state.latest_content.clear();
    if !text_buf.is_empty() {
        state
            .latest_content
            .push(ContentBlock::Text { text: text_buf });
    }
    for (id, name, args) in &tool_calls {
        state.latest_content.push(ContentBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            args: args.clone(),
        });
    }
    state.total_usage = TokenUsage {
        input_tokens: state.total_usage.input_tokens + _usage.input_tokens,
        output_tokens: state.total_usage.output_tokens + _usage.output_tokens,
        cached_tokens: state.total_usage.cached_tokens + _usage.cached_tokens,
        cache_write_tokens: state.total_usage.cache_write_tokens + _usage.cache_write_tokens,
        total_tokens: state.total_usage.total_tokens + _usage.total_tokens,
    };
    // v1.2 P1-12:同步累加进会话级总量(跨 turn 累加,与单 turn 的
    // `state.total_usage` 正交)。共享 `ctx.session_usage` 的 `Arc<RwLock>`,
    // `get_context_remaining` 工具 / 预算检查读同一把锁。saturating_add
    // 防溢出(u32 累加跨长会话理论上可能溢出,饱和更安全)。
    {
        let mut su = ctx.session_usage.write();
        su.input_tokens = su.input_tokens.saturating_add(_usage.input_tokens);
        su.output_tokens = su.output_tokens.saturating_add(_usage.output_tokens);
        su.cached_tokens = su.cached_tokens.saturating_add(_usage.cached_tokens);
        su.cache_write_tokens = su.cache_write_tokens.saturating_add(_usage.cache_write_tokens);
        su.total_tokens = su.total_tokens.saturating_add(_usage.total_tokens);
    }
    // v1.2 P1-12:会话预算硬上限检查。累计 `total_tokens >= budget` 时
    // 标记 `budget_exceeded` 并提前结束当前 turn(`submission_loop` 据
    // 此 emit `TurnStatus::TokenBudgetExceeded`)。`None` budget 不检查。
    if let Some(limit) = *ctx.token_budget.read() {
        if ctx.session_usage.read().total_tokens as u64 >= limit {
            state.budget_exceeded = true;
            return None;
        }
    }
    // M8 P1a: compute the per-turn USD cost via the pricing table. The
    // model name from `ctx.model` is `provider/canonical_id` (e.g.
    // `anthropic/claude-3-5-sonnet-latest`); strip the prefix so
    // `pricing::price` can match against the static table. v0.2.2:
    // 用本轮开头的 `model` 快照(`ctx.model.read().clone()`)而不是
    // 再次读 RwLock —— 重试场景应保持同 model。
    let bare_model = model.split_once('/').map(|(_, m)| m).unwrap_or(&model);
    let cost_usd = reflect_llm::providers::price(bare_model, &_usage);
    // Emit TokenCount after each model call (so callers see a snapshot
    // per turn — the TurnComplete event is the per-turn summary).
    let _ = ctx
        .event_tx
        .send(Event::new(
            ctx.sub_id.clone(),
            EventMsg::TokenCount(TokenCountEvent {
                input_tokens: _usage.input_tokens,
                output_tokens: _usage.output_tokens,
                cached_tokens: _usage.cached_tokens,
                cache_write_tokens: _usage.cache_write_tokens,
                total_tokens: _usage.total_tokens,
                cost_usd,
                provider: Some(success_provider),
                credential_label: Some(success_label),
            }),
        ))
        .await;

    if tool_calls.is_empty() {
        Some(GraphNode::CheckStop)
    } else {
        Some(GraphNode::ToolExec)
    }
}

/// v1.0 多 Provider 路由:`model_call` 全部 candidate 失败时的收尾
/// 路径:发 `Error` 事件 + 返回 `None` 让 `model_call` 终止,不发
/// `CheckStop` / `TurnComplete`。
async fn failed(tried: Vec<TriedCredential>, ctx: &NodeContext, code: &'static str) {
    let _ = ctx
        .event_tx
        .send(Event::new(
            ctx.sub_id.clone(),
            EventMsg::Error(ErrorEvent {
                code: code.into(),
                message: format!("all credentials exhausted ({} tried)", tried.len()),
                details: Some(serde_json::json!({ "tried": tried })),
            }),
        ))
        .await;
}

/// `tool_exec` — dispatch the tool_use blocks through the queue and append
/// their results to `state.latest_content`. Emits `ToolCallEnd` for each.
#[tracing::instrument(
    name = "agent.tool_exec",
    level = "info",
    skip_all,
    fields(sub_id = %ctx.sub_id)
)]
pub async fn tool_exec(state: &mut AgentState, ctx: &NodeContext) -> Option<GraphNode> {
    let calls: Vec<_> = state
        .latest_content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ToolUse { id, name, args } => Some(reflect_tools::ToolCallRequest {
                id: id.clone(),
                name: name.clone(),
                args: args.clone(),
            }),
            _ => None,
        })
        .collect();
    if calls.is_empty() {
        return Some(GraphNode::CheckStop);
    }
    let results = ctx
        .tools_queue
        .execute_all_with_gate(calls, ctx.approval_gate.clone())
        .await;
    // Remove the ToolUse blocks; replace with their results.
    state
        .latest_content
        .retain(|b| !matches!(b, ContentBlock::ToolUse { .. }));
    for r in results {
        // Emit ToolCallEnd for the wire protocol.
        let _ = ctx
            .event_tx
            .send(Event::new(
                ctx.sub_id.clone(),
                EventMsg::ToolCallEnd(ToolCallEndEvent {
                    call_id: r.call_id.clone(),
                    output: reflect_protocol::ToolOutput {
                        content: r.content.clone(),
                        is_error: r.is_error,
                        metadata: r.metadata.clone(),
                        elapsed_ms: r.elapsed_ms,
                    },
                    is_error: r.is_error,
                    elapsed_ms: r.elapsed_ms,
                }),
            ))
            .await;
        state.latest_content.push(r.into_content_block());
    }
    Some(GraphNode::CheckStop)
}

/// `check_stop` — dispatch the `Stop` hook. If the hook denies, the turn
/// continues; otherwise the turn ends.
pub async fn check_stop(state: &mut AgentState, ctx: &NodeContext) -> Option<GraphNode> {
    let decision = ctx
        .hook_engine
        .dispatch(&HookEvent::Stop {
            reason: StopReason::AgentDecision,
            attempt: state.stop_hook_attempts,
        })
        .await;
    match decision {
        reflect_hooks::HookDecision::Deny { reason } => {
            state.stop_hook_attempts = state.stop_hook_attempts.saturating_add(1);
            tracing::warn!(%reason, attempt = state.stop_hook_attempts, "Stop hook vetoed completion");
            Some(GraphNode::PreLoop)
        }
        _ => None,
    }
}

#[allow(dead_code)]
fn classify_retry(e: &LlmError) -> (bool, u64) {
    match e {
        LlmError::Http(_) | LlmError::Provider { .. } => (true, 1000),
        LlmError::Overloaded { .. } => (true, 1000),
        LlmError::RateLimited { retry_after_ms } => (true, *retry_after_ms),
        LlmError::Auth
        | LlmError::InvalidRequest { .. }
        | LlmError::ContextLengthExceeded { .. }
        | LlmError::Cancelled
        | LlmError::SseParse(_)
        | LlmError::Internal(_) => (false, 0),
    }
}

/// v1.0 多 Provider 路由:把 LLM 错误分类为「下一步动作」决策。
/// 比 v0.x 的 `classify_retry` 多了 Failover / CooldownAndFailover 两支,
/// 由 `model_call` 据此切换 `ModelRegistry::next_for` 而非同 credential
/// 重试。
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // 留作 Phase 3/4 扩展 / 单测用
pub(crate) enum RetryAction {
    /// 同 credential 立即重试(瞬时网络/5xx)。
    RetrySame { delay_ms: u64 },
    /// 切到下一个 credential,无 cooldown。
    Failover,
    /// 把当前 credential 标 cooldown + 切下一个。
    CooldownAndFailover { cooldown: Duration },
    /// 不可重试(`Auth` 自身也走 CooldownAndFailover,这条留给
    /// `Cancelled` / `ContextLengthExceeded` / 4xx 客户端错误)。
    GiveUp,
}

fn classify_action(e: &LlmError, default_cooldown_rate_limited: Duration) -> RetryAction {
    match e {
        LlmError::Auth => RetryAction::CooldownAndFailover {
            // Auth 不会自愈,1 小时冷却,期间 routing 切到别的 key。
            cooldown: Duration::from_secs(3600),
        },
        LlmError::RateLimited { retry_after_ms } => RetryAction::CooldownAndFailover {
            cooldown: Duration::from_millis(*retry_after_ms).max(default_cooldown_rate_limited),
        },
        LlmError::Overloaded { retry_after_ms } => RetryAction::CooldownAndFailover {
            cooldown: Duration::from_millis(*retry_after_ms),
        },
        LlmError::Provider { status, .. } if *status >= 500 => RetryAction::CooldownAndFailover {
            cooldown: Duration::from_secs(60),
        },
        LlmError::Http(_) => RetryAction::RetrySame { delay_ms: 1000 },
        LlmError::SseParse(_) | LlmError::Internal(_) => RetryAction::RetrySame { delay_ms: 500 },
        LlmError::Provider { .. } => RetryAction::GiveUp, // 任意未匹配的 4xx(包含 400..=499)
        LlmError::ContextLengthExceeded { .. }
        | LlmError::InvalidRequest { .. }
        | LlmError::Cancelled => RetryAction::GiveUp,
    }
}

/// 把 `RetryAction` 转化为 `classify_retry` 旧 API 兼容形态(供旧
/// 单测或外部 caller)。
#[allow(dead_code)]
fn retry_action_to_classify(a: &RetryAction) -> (bool, u64) {
    match a {
        RetryAction::RetrySame { delay_ms } => (true, *delay_ms),
        RetryAction::Failover | RetryAction::CooldownAndFailover { .. } => (false, 0),
        RetryAction::GiveUp => (false, 0),
    }
}

/// 抽取 `LlmError` → `RoutingEvent.outcome` 字符串码。
fn outcome_code(e: &LlmError) -> &'static str {
    match e {
        LlmError::Auth => "auth",
        LlmError::RateLimited { .. } => "rate_limited",
        LlmError::Overloaded { .. } => "overloaded",
        LlmError::Provider { status, .. } if *status >= 500 => "provider_5xx",
        LlmError::Http(_) => "network",
        LlmError::SseParse(_) => "sse_parse",
        LlmError::Internal(_) => "internal",
        LlmError::Provider { .. } => "provider_4xx",
        LlmError::ContextLengthExceeded { .. } => "context_too_long",
        LlmError::InvalidRequest { .. } => "invalid_request",
        LlmError::Cancelled => "cancelled",
    }
}

fn error_code(e: &LlmError) -> &'static str {
    match e {
        LlmError::Auth => "AUTH_FAILED",
        LlmError::RateLimited { .. } => "RATE_LIMITED",
        LlmError::ContextLengthExceeded { .. } => "CONTEXT_TOO_LONG",
        LlmError::InvalidRequest { .. } => "INVALID_REQUEST",
        LlmError::Provider { .. } => "PROVIDER_ERROR",
        LlmError::Overloaded { .. } => "OVERLOADED",
        LlmError::Http(_) => "NETWORK_ERROR",
        LlmError::SseParse(_) => "SSE_PARSE_ERROR",
        LlmError::Cancelled => "CANCELLED",
        LlmError::Internal(_) => "INTERNAL_ERROR",
    }
}

#[allow(dead_code)]
fn _suppress_unused(_e: EventMsg, _x: AgentMessageDelta, _u: UserInputItem) {}

#[allow(dead_code)]
const _DUMMY_TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_retry_matrix() {
        assert!(!classify_retry(&LlmError::Auth).0);
        assert!(
            classify_retry(&LlmError::RateLimited {
                retry_after_ms: 5000
            })
            .0
        );
        assert!(classify_retry(&LlmError::Overloaded { retry_after_ms: 0 }).0);
        assert!(classify_retry(&LlmError::Http("x".into())).0);
        assert!(
            classify_retry(&LlmError::Provider {
                status: 500,
                message: "x".into()
            })
            .0
        );
        assert!(!classify_retry(&LlmError::Cancelled).0);
        assert!(
            !classify_retry(&LlmError::InvalidRequest {
                message: "x".into()
            })
            .0
        );
    }

    #[test]
    fn error_code_covers_all_variants() {
        assert_eq!(error_code(&LlmError::Auth), "AUTH_FAILED");
        assert_eq!(error_code(&LlmError::Cancelled), "CANCELLED");
        assert_eq!(
            error_code(&LlmError::ContextLengthExceeded { used: 0, limit: 0 }),
            "CONTEXT_TOO_LONG"
        );
    }
}
