# Protocol Bridge (Tauri ↔ Reflect-Protocol)

> **Wire format = `reflect_protocol::Event` / `Submission` directly (snake_case JSON).** No custom serializers, no DTO layer. The same envelope is shared with the TUI and `reflect exec`.
>
> Reading order: §1 envelope → §2 submission (UI → backend) → §3 event (backend → UI) → §4 id pairing → §5 version compatibility → §6 generated TS types.

---

## 1. Envelope

```rust
// vendor/reflect-protocol/src/event.rs
pub struct Event {
    pub id: String,     // matches Submission.id; EVENT_ID_NONE = "" for lifecycle
    pub msg: EventMsg,  // serde-tagged enum (snake_case)
}
```

Two channels:

- **UI → backend**: `invoke('reflect_submit', { submission })` where `submission: Submission = { id, op, ... }`.
- **Backend → UI**: `listen('reflect_event', handler)` where payload is `Event`. **Single channel name**; dispatch by `msg.type` (snake_case discriminator).

> Frontend never wraps in an envelope. The Tauri command name is the only framing.

---

## 2. Submission (UI → Backend)

`Op` has 14 variants in `vendor/reflect-protocol/src/op.rs`:

| # | Op variant | Tauri command | Payload |
|---|---|---|---|
| 1 | `UserInput` | `reflect_submit` | `{ items: UserInputItem[], thread_settings? }` |
| 2 | `Rewind { to_turn_id? }` | `reflect_rewind` | `to_turn_id?: string` |
| 3 | `Compact` | `reflect_compact` | `null` |
| 4 | `Interrupt` | `reflect_interrupt` | `null` |
| 5 | `Shutdown` | `reflect_shutdown` | `null` |
| 6 | `ToolApproval { id, decision }` | `reflect_tool_approval` | `{ id: string, decision: ReviewDecision }` |
| 7 | `HookApproval { id, decision }` | `reflect_hook_approval` | `{ id: string, decision: ReviewDecision }` |
| 8 | `EnterPlanMode { task }` | `reflect_enter_plan_mode` | `{ task: string }` |
| 9 | `ExitPlanMode` | `reflect_exit_plan_mode` | `null` |
| 10 | `PlanApproval { id, decision }` | `reflect_plan_approval` | `{ id: string, decision: ReviewDecision }` |
| 11 | `SetEffort { effort }` | `reflect_set_effort` | `{ effort: ReasoningEffortMirror }` |
| 12 | `AskUserQuestionResponse { id, answers }` | `reflect_ask_user_question_response` | `{ id: string, answers: AskUserAnswer }` |
| 13 | `AskUserInputResponse { id, text }` | `reflect_ask_user_input_response` | `{ id: string, text: string }` |
| 14 | `SetPermissionMode { mode }` | `reflect_set_permission_mode` | `{ mode: PermissionMode }` |
| 15 | `CyclePermissionMode` | `reflect_cycle_permission_mode` | `null` |

> **注**:除 `UserInput` 经 `reflect_submit` 接受完整 `Submission` 外,其余 14 个命令
> 后端在 `commands/mod.rs` 内部构造 `Op` 并经 `MinimalAgent::submit_op` 投递(阶段 2 实装,
> 不再是空 `Ok(())`)。所有 Op 命令返回 `string`(submission id,供前端 pairing/调试)。

### 2.0 非 Op 命令(诊断 / 配置 / 工具列表)

这些命令不对应 `Op` 变体,是 ReflectDesktop 自己的诊断/配置面:

| Command | Payload | Returns | 用途 |
|---|---|---|---|
| `ping` | `null` | `{ msg, version }` | IPC 通路健康检查 |
| `reflect_agent_status` | `null` | `{ ready, has_model, model, workspace, degraded_reason }` | 前端状态徽标 + 降级引导 |
| `reflect_get_config` | `null` | `string`(TOML) | 读 `~/.reflect/config.toml` |
| `reflect_save_config` | `{ toml: string }` | `null` | 写回(写盘前 `load_from_str` 校验,热更新共享 cfg) |
| `reflect_list_tools` | `null` | `Vec<{ name, description }>` | 当前 ToolRegistry 注册的工具 |
| `reflect_list_sessions` / `reflect_rename_session` / `reflect_delete_session` / `reflect_replay_session` | — | — | session I/O(rollout) |

`Submission` wraps the op:

```rust
pub struct Submission {
    pub id: String,
    pub op: Op,
    pub client_user_message_id: Option<String>,
    pub trace: Option<TraceContext>,
}
```

### 2.1 `UserInputItem` (`vendor/reflect-protocol/src/item.rs`)

| Variant | Fields | Frontend source |
|---|---|---|
| `Text` | `text: String` | Composer textarea |
| `Image` | `data: Vec<u8>, mime_type: String` | Drag/drop or paste (base64-decoded on frontend) |
| `LocalImage` | `path: PathBuf` | File picker (backend reads from disk) |
| `Skill` | `name: String, args: Option<JSON>` | `/skill name` or Skill picker |
| `QuestionAnswer` | `request_id, answers` | LLM-asked question reply (alt to `AskUserQuestionResponse`) |

### 2.2 Submission ID lifecycle

- Frontend generates UUID v4 → `Submission.id`.
- Backend returns same id from `reflect_submit` (echo).
- Frontend correlates inbound `Event.id` to the originating `Submission.id` to unlock composer / mark turn done.
- `EVENT_ID_NONE = ""` events (lifecycle: `SessionConfigured`, `ShutdownComplete`, `PermissionModeChanged`, …) have no matching submission.

---

## 3. Event (Backend → UI)

`EventMsg` is `#[serde(tag = "type", rename_all = "snake_case")]` (current = 32 variants in `vendor/reflect-protocol/src/event_msg.rs`).

> **Naming**: Rust `enum` uses **PascalCase struct variants** (e.g. `TurnStarted(TurnStartedEvent)`), but serde emits **snake_case** discriminators (`turn_started`) for the frontend. Frontend TypeScript uses the snake_case form.

### 3.1 Lifecycle (5)

| Variant | Triggered by | UI impact |
|---|---|---|
| `session_configured` | First turn of a thread | Show banner; persist `session_id` |
| `turn_started` | `Op::UserInput` accepted | Spinner on; clear composer draft |
| `turn_complete` | LLM turn finished | Spinner off; unlock composer; accumulate cost |
| `turn_aborted` | `Op::Interrupt` / error | Red toast |
| `turn_rewound` | `Op::Rewind` success | Truncate local scrollback; redraw |
| `shutdown_complete` | `Op::Shutdown` | Quit / hide |

### 3.2 LLM output (4)

| Variant | Fields | UI |
|---|---|---|
| `agent_message` | `text: String` | Non-streaming final bubble |
| `agent_message_delta` | `delta: String` | Stream chunk → append to last assistant bubble |
| `thinking_delta` | `delta: String` | Reasoning block (collapsible) |
| `token_count` | `TokenCountEvent` (info) | Status bar token / cost / context ring |

### 3.3 Tools (2)

| Variant | Fields | UI |
|---|---|---|
| `tool_call_begin` | `ToolCallBeginEvent` | Tool row running indicator |
| `tool_call_end` | `ToolCallEndEvent` | Tool row done / failed / cancelled |

### 3.4 Approvals / AskUser (4)

| Variant | Frontend reaction | Pairing Op |
|---|---|---|
| `approval_request` (Tool/Hook/Plan) | Open `ApprovalModal` | `tool_approval` / `hook_approval` |
| `ask_user_question` | Open `QuestionModal` (multi-Q) | `ask_user_question_response` (id = sub_id) |
| `ask_user_input` | Open `AskUserModal` (free text) | `ask_user_input_response` |
| `permission_bubble` | Non-blocking toast (Bubble mode) | none |

### 3.5 Compaction / Errors (3)

| Variant | UI |
|---|---|
| `context_compacted` | "Compacting..." badge → fade |
| `error` | Red error bubble (recoverable) |
| `stream_error` | Retry countdown |

### 3.6 Config / Routing / Collab (5)

| Variant | UI |
|---|---|
| `config_reloaded` | Toast "Config reloaded" |
| `routing` | Status bar provider switch indicator |
| `collab_started` / `collab_message` / `collab_finished` | Collab sidebar / timeline |

### 3.7 MCP (3)

| Variant | UI |
|---|---|
| `mcp_server_started` | Status bar MCP segment |
| `mcp_server_failed` | Error toast |
| `mcp_tool_invoked` | Tool row "(via mcp:server)" badge |

### 3.8 LSP (2) — v0.5+

| Variant | UI |
|---|---|
| `lsp_server_started` | Status bar LSP segment |
| `lsp_server_failed` | Error toast |

### 3.9 Plan (5)

| Variant | UI |
|---|---|
| `plan_request` | "Enter plan mode?" confirm modal |
| `plan_ready` | Plan markdown modal (Accept / Submit Changes) |
| `plan_approved` | Status bar plan badge |
| `plan_rejected` | Status hint |
| `permission_mode_changed` | Status bar mode badge updates |

---

## 4. Id Pairing Rules

| Event | Trigger Op | Field | Notes |
|---|---|---|---|
| `approval_request` (Tool) | `tool_approval` | `id = request_id` | |
| `approval_request` (Hook) | `hook_approval` | `id = request_id` | |
| `approval_request` (Plan) | `plan_approval` | `id = request_id` | |
| `ask_user_question` | `ask_user_question_response` | `id = sub_id` | Each question has its own sub_id (per-question waiter) |
| `ask_user_input` | `ask_user_input_response` | `id = sub_id` | |
| `plan_request` | `enter_plan_mode` | `task` echoed | Client confirms before sending |
| `plan_ready` | `plan_approval` | `id = plan_id` | |
| `*_delta` / `*_complete` | (any user op) | `id = submission.id` | Stream chunks share submission id |

---

## 5. Version Compatibility

- **Additive-only evolution**: new variants are non-breaking (deserializer ignores unknown).
- **Snake_case tag** protects against Rust rename refactors leaking to wire.
- **`#[serde(default)]`** on optional fields shields frontends from missing keys during rollout.
- **Frontend type regeneration**: when `reflect-protocol` changes, run:

  ```bash
  # In Reflect-Agent repo:
  cargo run -p reflect-protocol --example dump_schema > /tmp/reflect-schema.json
  # In ReflectDesktop:
  npx json2ts /tmp/reflect-schema.json -o src/types/protocol.ts
  ```

- **Vendor sync**: `bash scripts/vendor-sync.sh /path/to/Reflect-Agent main` mirrors crates. After sync, re-run the dump → json2ts pipeline.

---

## 6. Frontend TypeScript Surface

`src/types/protocol.ts` is the canonical mirror. Two halves:

```ts
// Submission side (UI → backend)
export type ReflectSubmission = {
  id: string;
  op: ReflectOp;
  client_user_message_id?: string;
  trace?: TraceContext;
};

export type ReflectOp =
  | { type: 'user_input'; items: ReflectItem[]; thread_settings?: ThreadSettingsOverrides }
  | { type: 'compact' }
  | { type: 'interrupt' }
  | { type: 'shutdown' }
  | { type: 'rewind'; to_turn_id?: string }
  | { type: 'tool_approval'; id: string; decision: ReviewDecision }
  | { type: 'hook_approval'; id: string; decision: ReviewDecision }
  | { type: 'enter_plan_mode'; task: string }
  | { type: 'exit_plan_mode' }
  | { type: 'plan_approval'; id: string; decision: ReviewDecision }
  | { type: 'set_effort'; effort: ReasoningEffort }
  | { type: 'ask_user_question_response'; id: string; answers: AskUserAnswer }
  | { type: 'ask_user_input_response'; id: string; text: string }
  | { type: 'set_permission_mode'; mode: PermissionMode }
  | { type: 'cycle_permission_mode' };

// Event side (backend → UI)
export type ReflectEvent = { id: string; msg: ReflectEventMsg };

export type ReflectEventMsg =
  | { type: 'session_configured'; /* ... */ }
  | { type: 'turn_started'; /* ... */ }
  | { type: 'turn_complete'; /* ... */ }
  // ... 32 variants, snake_case discriminators
  | { type: 'permission_mode_changed'; from: PermissionMode; to: PermissionMode };
```

### 6.1 Bridging pattern (one place)

`src/utils/tauri.ts` is the only file that touches `invoke` / `listen`. Features call typed wrappers (`reflect_submit`, `onReflectEvent`).

```ts
// src/utils/tauri.ts
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return await invoke<string>('reflect_submit', { submission: s });
}

export async function onReflectEvent(handler: (e: ReflectEvent) => void) {
  return await listen<ReflectEvent>('reflect_event', (ev) => handler(ev.payload));
}
```

### 6.2 Reducer pattern

`src/services/agent.ts::handle_event` is the single dispatch fan-out. New variants:

1. Add the `ReflectEventMsg` arm in `src/types/protocol.ts`.
2. Add a `case 'new_variant':` in `handle_event`.
3. Cover with a unit test in `src/services/agent.test.ts`.

---

## 7. Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| Frontend gets `unknown variant` | Protocol added but TS not regenerated | Re-run `dump_schema` + `json2ts` |
| Submission silently dropped | Backend deserialization error | Check `tracing` log on `commands/mod.rs` |
| Event arrives with stale id | Submission id was overridden | Use `crypto.randomUUID()` only |
| Approval modal won't close | Frontend forgot to dispatch `*_approval` Op | Check `ModalShell` `onSubmit` |
| `EVENT_ID_NONE` paired as submission | Lifecycle event matched by id | Filter `e.id === ''` in reducer |

---

## 8. Reference

- Rust source of truth: `vendor/reflect-protocol/src/{event,event_msg,op,item,submission}.rs`
- TS mirror: `src/types/protocol.ts`
- Tauri command registry: `src-tauri/src/lib.rs` + `src-tauri/src/commands/mod.rs`
- Event forwarder: `src-tauri/src/events.rs::forward_agent_events`
- Frontend fanout: `src/services/agent.ts::useAgent` / `handle_event`
- IPC wrapper: `src/utils/tauri.ts`