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

`Op` variants are listed in `vendor/reflect-protocol/src/op.rs`; each maps 1:1 to a Tauri command whose body lives in `src-tauri/src/commands/<domain>.rs` (re-exported by `src-tauri/src/commands/mod.rs`). The full set is enumerated in the §2.0 table below; the canonical Op→command mapping is the single source of truth and should be regenerated whenever `vendor/reflect-protocol/src/op.rs` changes.

> **注**:除 `UserInput` 经 `reflect_submit` 接受完整 `Submission` 外,其余 Op 命令
> 后端在 `commands/<domain>.rs` 内部构造 `Op` 并经 `MinimalAgent::submit_op` 投递。
> 所有 Op 命令返回 `string`(submission id,供前端 pairing/调试)。

### 2.0 Commands outside Op (diagnostic / config / domain I/O)

These commands do not correspond to an `Op` variant; they live alongside Op-derived commands in the same per-domain modules:

| Command | Payload | Returns | Where (Rust) / Where (TS) |
|---|---|---|---|
| `ping` | `null` | `{ msg, version }` | `src-tauri/src/commands/agent.rs` (barrel) / `src/utils/commands/health.ts` |
| `reflect_agent_status` | `null` | `{ ready, has_model, model, workspace, degraded_reason }` | `commands/agent.rs` / `commands/agent.ts` |
| `reflect_get_config` | `null` | `string` (TOML) | `commands/config.rs` / `commands/config.ts` |
| `reflect_save_config` | `{ toml }` | `null` | `commands/config.rs` / `commands/config.ts` |
| `reflect_list_tools` | `null` | `Vec<{ name, description }>` | `commands/agent.rs` / `commands/agent.ts` |
| `reflect_list_sessions` / `reflect_rename_session` / `reflect_delete_session` / `reflect_replay_session` | — | — | `commands/sessions.rs` / `commands/sessions.ts` |
| `reflect_export_session` / `reflect_export_session_markdown` | `{ id }` / `{ id }` | `string` (path) / `string` (markdown) | `commands/export.rs` / `commands/sessions.ts` |
| `reflect_git_status` / `reflect_git_diff` / `reflect_git_log` | — | `GitStatus` / `string` / `Vec<GitLogEntry>` | `commands/git.rs` / `commands/git.ts` |
| `reflect_run_shell` | `{ cmd }` | `ShellSession { id, command, cwd }` | `commands/shell.rs` / `commands/terminal.ts`; `reflect_terminal_output` event streams output |
| `reflect_kill_shell` | `{ session_id }` | `null` | `commands/shell.rs` / `commands/terminal.ts` |
| `reflect_list_shell_sessions` | `null` | `string[]` | `commands/shell.rs` / `commands/terminal.ts` |
| `reflect_list_dir` / `reflect_read_file` / `reflect_search_files` | `{ path?, maxDepth? }` / `{ path }` / `{ query, ... }` | `DirListing` / `FileReadResult` / search results | `commands/files.rs` / `commands/files.ts`; `commands/search.rs` / `commands/search.ts` |
| `reflect_load_allowlist` / `reflect_save_allowlist` / `reflect_check_allowlist` | — | allowlist payload | `commands/allowlist.rs` / `commands/allowlist.ts` |
| `reflect_list_workspaces` / `reflect_set_workspace` / `reflect_current_workspace` | — | workspace payloads | `commands/workspaces.rs` / `commands/workspaces.ts` |
| `reflect_list_skills` | `null` | `SkillEntry[]` | `commands/skills.rs` / `commands/skills.ts` |
| `reflect_list_memory` / `reflect_add_memory` / `reflect_remove_memory` | `{ scope?, key?, value? }` | memory payloads | `commands/memory.rs` / `commands/memory.ts` |
| `reflect_list_hooks` / `reflect_toggle_hook` | — | hook payloads | `commands/hooks.rs` / `commands/hooks.ts` |
| `reflect_check_update` | `null` | update info | `commands/update.rs` / `commands/updates.ts` |
| `reflect_list_tasks` / `reflect_create_task` / `reflect_get_task` / `reflect_update_task` / `reflect_claim_task` / `reflect_delete_task` | `{ list }` / `{ list, subject, description, active_form?, owner?, metadata? }` / `{ list, id }` / `{ list, id, patch }` / `{ list, claimer }` / `{ list, id }` | `Task[]` / `Task` / `Task` / `{ task, updatedFields, statusChange? }` / `Task?` / `null` | `commands/tasks.rs` (Phase 1 multi-agent) / `commands/tasks.ts` |
| `reflect_list_teams` / `reflect_upsert_team` / `reflect_get_team` / `reflect_delete_team` | `null` / `{ team }` / `{ name }` / `{ name }` | `TeamFile[]` / `null` / `TeamFile` / `null` | `commands/tasks.rs` (Phase 1 multi-agent) / `commands/teams.ts` |
| `reflect_list_schedules` / `reflect_add_schedule` / `reflect_update_schedule` / `reflect_remove_schedule` / `reflect_get_schedule_status` | `null` / `{ schedule, prompt, name? }` / `{ id, schedule?, prompt?, name?, enabled? }` / `{ id }` / `null` | `CronJobSpec[]` / `CronJobSpec` / `CronJobSpec` / `bool` / `{ status, line, total, enabled }` | `commands/schedule.rs` (Phase 1 item 2) / `commands/schedule.ts` |
| `reflect_list_agent_defs` / `reflect_get_agent_def` / `reflect_save_agent_def` / `reflect_delete_agent_def` / `reflect_parse_agent_md` | `null` / `{ name }` / `{ def }` / `{ name }` / `{ text }` | `AgentDefinition[]` / `AgentDefinition` / `AgentDefinition` / `bool` / `AgentDefinition` | `commands/agents.rs` (Phase 1 item 3) / `commands/agents.ts` |
| `reflect_start_side_channel` / `reflect_cancel_side_channel` / `reflect_list_side_channels` / `reflect_get_side_channel` | `{ agent_name, prompt }` / `{ id }` / `null` / `{ id }` | `StartSideChannelResult` / `bool` / `SideChannelInfo[]` / `SideChannelInfo` | `commands/side_channel.rs` (Phase 2 item 1) / `commands/side_channel.ts` |
| `reflect_get_remote_config` / `reflect_update_remote_config` / `reflect_get_remote_status` | `null` / `{ host?, port?, auth_token?, auto_connect? }` / `null` | `RemoteConfigSnapshot { config, endpoint, isReady }` / `null` / `RemoteStatus { state, message, endpoint, sinceMs }` | `commands/remote.rs` (Phase 2 item 2) / `commands/remote.ts` |
| `reflect_tailscale_status` / `reflect_tailscale_daemon_command_preview` / `reflect_tailscale_daemon_start` / `reflect_tailscale_daemon_stop` / `reflect_tailscale_daemon_status` | `null` (all) | `TailscaleStatus { installed, running, dnsName, ipv4, ipv6, suggestedRemoteHost }` / `string` / `string` / `string` / `{ running, pid }` | `commands/remote.rs` (Phase 2 item 2) / `commands/remote.ts` |
| `reflect_kms_list` / `reflect_kms_create` / `reflect_kms_delete` / `reflect_kms_list_pages` / `reflect_kms_get_page` / `reflect_kms_save_page` / `reflect_kms_search` / `reflect_dream` | `null` / `{ name, description? }` / `{ name }` / `{ wiki }` / `{ wiki, page }` / `{ wiki, page, content, title?, tags? }` / `{ query }` / `{ insights, sessionsAnalyzed }` | `WikiInfo[]` / `WikiInfo` / `null` / `Page[]` / `Page` / `Page` / `SearchResult[]` / `DreamResult` | `commands/kms.rs` (Phase 3 item 12) / `commands/kms.ts` |
| `reflect_get_autopilot_config` / `reflect_update_autopilot_config` / `reflect_autopilot_history` | `null` / `{ config }` / `null` | `AutopilotConfig` / `null` / `AutopilotRun[]` | `commands/autopilot.rs` (Phase 3 item 10) / `commands/autopilot.ts` |
| `reflect_list_activity` / `reflect_search_activity` / `reflect_clear_activity` / `reflect_activity_count` | `{ filter? }` / `{ query }` / `null` / `null` | `ActivityEvent[]` (camelCase; `id`, `tsMs`, `kind`, `actor`, `summary`, `taskId?`, `teamName?`, `level`) / `ActivityEvent[]` / `null` / `number` | `commands/activity.rs` (Phase 3 item 9) / `commands/activity.ts` |
| `reflect_list_squads` / `reflect_create_squad` / `reflect_get_squad` / `reflect_delete_squad` / `reflect_delegate_next` / `reflect_assign_squad_task` | `null` / `{ spec }` / `{ name }` / `{ name }` / `{ name, leaderActorId }` / `{ name, taskId, assignee? }` | `SquadSpec[]` / `null` / `SquadSpec` / `null` / `Task?` / `Task` | `commands/squad.rs` (Phase 3 item 11) / `commands/squad.ts` |
| `reflect_list_media` / `reflect_image_process` / `reflect_screenshot` / `reflect_computer_use` / `reflect_media_capabilities` | `{ dir }` / `{ spec }` / `null` / `{ action }` / `null` | `MediaAsset[]` / `ImageProcessResult` / `string` (base64 PNG) / `null` / `MediaCapabilities` | `commands/media.rs` (Phase 3 item 13) / `commands/media.ts` |
| `reflect_set_dock_badge` | `{ count? }` | `null` | `src-tauri/src/dock.rs` (macOS) / `src/utils/tauri.ts` (compat) |

**`reflect_terminal_output` event** (B8-01):

```ts
interface ReflectShellOutputChunk {
  session_id: string;
  stream: 'stdout' | 'stderr' | 'exit' | 'error';
  data: string;     // 文本行(exit 时为退出码字符串)
  seq: number;      // 同一 session 内的单调递增序号
}
```

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

`EventMsg` is `#[serde(tag = "type", rename_all = "snake_case")]` (the variant list lives in `vendor/reflect-protocol/src/event_msg.rs`).

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
| `token_count` | `TokenCountEvent` (info) | StatusBar token/cost indicator + Inspector "Token Usage" section (input/output/cached/cache_write/total/cost/provider/credential). `TokenCountEvent` 字段：`input_tokens`、`output_tokens`、`cached_tokens`、`cache_write_tokens`（M8，input 子集，不计入 total）、`total_tokens`、`cost_usd?`、`provider?`、`credential_label?` |

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
  // ... variant union, snake_case discriminators
  | { type: 'permission_mode_changed'; from: PermissionMode; to: PermissionMode };
```

### 6.1 Bridging pattern

The frontend IPC layer is split into three roles:

1. **Low-level bridge** — `src/utils/bridge.ts` exports `invoke` / `listen` with Tauri-context fallback and an `isMissingTauriInvokeError` guard.
2. **Per-domain wrappers** — `src/utils/commands/{health,agent,approvals,plan,permissions,questions,config,sessions,events,workspaces,skills,memory,hooks,git,terminal,files,allowlist,updates,search}.ts` re-export typed wrappers. The `src/utils/commands/index.ts` barrel aggregates them.
3. **Compatibility barrels** — `src/utils/tauri.ts` and `src/utils/commands.ts` re-export from `./bridge`, `./commands`, `./types` so existing `@/utils/tauri` and `@/utils/commands` imports continue to work. **New code should import directly from `@/utils/commands/{domain}` or from `@/utils/bridge` instead.**

```ts
// src/utils/bridge.ts (low-level)
import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

function isMissingTauriInvokeError(e: unknown): boolean { /* ... */ }

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> { /* ... */ }
export async function listen<T>(event: string, handler: (e: { payload: T }) => void): Promise<UnlistenFn> { /* ... */ }
```

```ts
// src/utils/commands/agent.ts (per-domain wrapper)
import { invoke } from '../bridge';

export async function reflect_submit(s: ReflectSubmission): Promise<string> {
  return await invoke<string>('reflect_submit', { submission: s });
}

export async function onReflectEvent(handler: (e: ReflectEvent) => void) {
  return await listen<ReflectEvent>('reflect_event', (ev) => handler(ev.payload));
}
```

### 6.2 Reducer pattern

`src/services/agentEventBus.ts` (refcounted bus) forwards into `src/stores/agent/reducer.ts` — the canonical reducer for `ReflectEventMsg`. `src/services/agent.ts` remains a compatibility re-export that forwards `useAgent` / `handle_event` to the store. `src/stores/agentStore.ts` is the user-facing compat facade that re-exports `useAgentStore` / `reduceEvent` / types from `./agent`. New variants:

1. Add the `ReflectEventMsg` arm in `src/types/protocol.ts`.
2. Add a `case 'new_variant':` in `src/stores/agent/reducer.ts`.
3. Cover with a unit test in `src/stores/agentStore.test.ts` and / or the bus test.

---

## 7. Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| Frontend gets `unknown variant` | Protocol added but TS not regenerated | Re-run `dump_schema` + `json2ts` |
| Submission silently dropped | Backend deserialization error | Check `tracing` log in `src-tauri/src/commands/<domain>.rs` |
| Event arrives with stale id | Submission id was overridden | Use `crypto.randomUUID()` only |
| Approval modal won't close | Frontend forgot to dispatch `*_approval` Op | Check `src/features/modals/ModalShell.tsx` `onSubmit` |
| `EVENT_ID_NONE` paired as submission | Lifecycle event matched by id | Filter `e.id === ''` in `src/stores/agent/reducer.ts` |

---

## 8. Reference

- Rust source of truth: `vendor/reflect-protocol/src/{event,event_msg,op,item,submission}.rs`
- TS mirror: `src/types/protocol.ts`
- Tauri command registry: `src-tauri/src/lib.rs` (handler list) + `src-tauri/src/commands/mod.rs` (barrel) + per-domain bodies in `src-tauri/src/commands/<domain>.rs`
- Event forwarder: `src-tauri/src/events.rs::forward_agent_events`
- Frontend event fanout: `src/services/agentEventBus.ts` → `src/stores/agent/reducer.ts`; compat re-export in `src/services/agent.ts`
- IPC low-level bridge: `src/utils/bridge.ts`
- IPC wrappers (canonical, per-domain): `src/utils/commands/<domain>.ts` aggregated by `src/utils/commands/index.ts`
- IPC compat barrels: `src/utils/tauri.ts`, `src/utils/commands.ts`
- Agent store: `src/stores/agent/` (canonical impl) with compat facade `src/stores/agentStore.ts`
