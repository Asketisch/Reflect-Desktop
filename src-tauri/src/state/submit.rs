//! `submit` / `submit_op` / `interrupt` —— per-turn 提交流水。
//!
//! 提交流程:1) 拿 AgentThread handle;2) 构造 `Submission`;3) 投给
//! `agent_thread.submit(...)` 拿 `TurnHandle`;4) spawn 一个 forwarder
//! 把该 turn 的所有 event 推 session broadcast。
//!
//! 前端按 `event.id == submission.id` 过滤出属于本次 turn 的事件;
//! `submit_op` 自动生成 id,审批/effort/permission 这类 op 走相同的
//! 生命周期事件 fan-out。

use super::MinimalAgent;

/// 提交 Submission → 拿 per-turn `TurnHandle` → spawn 转发到 broadcast。
///
/// 前端拿到的所有 per-turn event 都通过 session broadcast 派发,
/// 前端按 `event.id == submission.id` 过滤出属于本次 turn 的事件。
pub(crate) async fn submit(
    agent: &MinimalAgent,
    submission: reflect_protocol::Submission,
) -> anyhow::Result<()> {
    // 0. 必须先 install_agent_thread。
    let thread = {
        let guard = agent.inner.thread.lock();
        guard
            .clone()
            .ok_or_else(|| anyhow::anyhow!("agent thread not installed yet; setup not complete"))?
    };

    // 1. 拿 per-turn handle。
    let mut handle: reflect_core::TurnHandle = thread.submit(submission.clone()).await;

    // 2. spawn 转发:这个 turn 的所有 event 进 broadcast。
    let session_tx = agent.inner.session_tx.clone();
    let sub_id = submission.id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = handle.next().await {
            if session_tx.send(event).is_err() {
                tracing::debug!("[reflect-gui] no subscribers for turn {}", sub_id);
            }
        }
        tracing::debug!("[reflect-gui] turn {} forwarder closed", sub_id);
    });

    Ok(())
}

/// 投递一个非 `UserInput` 的 `Op`(compact / rewind / approval / plan / effort /
/// permission / ask_user 等)。自动生成 submission id(EVENT_ID_NONE 级别的
/// 生命周期 op 不需要前端 pairing,但审批/ask_user 的 id 与 pending 事件 id 对齐)。
///
/// 供 `commands/mod.rs` 的 12 个 Op 命令统一调用。
pub(crate) async fn submit_op(
    agent: &MinimalAgent,
    op: reflect_protocol::Op,
) -> anyhow::Result<String> {
    let thread = {
        let guard = agent.inner.thread.lock();
        guard
            .clone()
            .ok_or_else(|| anyhow::anyhow!("agent thread not installed yet; setup not complete"))?
    };
    let submission = reflect_protocol::Submission::with_id(uuid::Uuid::new_v4().to_string(), op);
    let id = submission.id.clone();
    // 这类 op 通常无 per-turn 流式输出(审批/effort/permission 立即生效),
    // 但仍走 submit 以保持生命周期事件(SessionConfigured / PermissionModeChanged 等)
    // 的 fan-out 一致性。
    let mut handle = thread.submit(submission).await;
    let session_tx = agent.inner.session_tx.clone();
    let sub_id = id.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = handle.next().await {
            let _ = session_tx.send(event);
        }
        tracing::debug!("[reflect-gui] op turn {} forwarder closed", sub_id);
    });
    Ok(id)
}

/// 中断当前 turn —— 调用 AgentThread 的 cancel token。
/// CancellationToken 的 cancel 是幂等的,重复调用安全。
pub(crate) fn interrupt(agent: &MinimalAgent) {
    let guard = agent.inner.thread.lock();
    if let Some(thread) = guard.as_ref() {
        thread.cancel_token().cancel();
    }
}

#[cfg(test)]
mod tests {
    use crate::state::MinimalAgent;

    /// submit_op 在未 install 时必须返回 Err(instead of panic)。
    /// 这是 AGENTS.md「修复根因，而非打补丁」的体现:
    /// commands 层依赖此错误路径,前端会展示为 toast。
    #[tokio::test]
    async fn submit_op_returns_err_when_not_installed() {
        let agent = MinimalAgent::new_empty();
        let result = agent.submit_op(reflect_protocol::Op::Compact).await;
        assert!(
            result.is_err(),
            "submit_op before install must error, got: {result:?}"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not installed") || err_msg.contains("setup"),
            "error message must explain root cause: {err_msg}"
        );
    }
}
