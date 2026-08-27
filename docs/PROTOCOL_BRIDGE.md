# 协议桥（Tauri ↔ Reflect-Protocol）

> **线格式 = `reflect_protocol::Event` / `Submission` 直接传递（snake_case JSON）。** 无自定义序列化器，无 DTO 层。同一信封与 TUI 和 `reflect exec` 共享。
>
> 阅读顺序：§1 信封 → §2 提交（UI → 后端）→ §3 事件（后端 → UI）→ §4 ID 配对 → §5 版本兼容性 → §6 生成的 TS 类型。

---

## 1. 信封

```rust
// reflect-agent/crates/protocol/reflect-protocol/src/event.rs
pub struct Event {
    pub id: String,     // matches Submission.id; EVENT_ID_NONE = "" for lifecycle
    pub msg: EventMsg,  // serde-tagged enum (snake_case)
}
```

两条通道：

- **UI → 后端**：`invoke('reflect_submit', { submission })` 其中 `submission: Submission = { id, op, ... }`。
- **后端 → UI**：`listen('reflect_event', handler)` 载荷为 `Event`。**单一通道名**；按 `msg.type`（snake_case 鉴别器）分派。

> 前端从不使用信封包装。Tauri 命令名是唯一框架。

---

## 2. 提交（UI → 后端）

`Op` 变体列在 `reflect-agent/crates/protocol/reflect-protocol/src/op.rs` 中；每个 1:1 映射到一个 Tauri 命令，其主体位于 `src-tauri/src/commands/<domain>.rs`（由 `src-tauri/src/commands/mod.rs` re-export）。完整集合见下方 §2.0 表格；规范的 Op→命令映射是唯一真相源，应在 `reflect-protocol/src/op.rs` 变更时重新生成。

> **注**:除 `UserInput` 经 `reflect_submit` 接受完整 `Submission` 外,其余 Op 命令
> 后端在 `commands/<domain>.rs` 内部构造 `Op` 并经 `MinimalAgent::submit_op` 投递。
> 所有 Op 命令返回 `string`(submission id,供前端 pairing/调试)。

> **Goal 模式(v1.2 P1)**:`enter_goal_mode { goal, verify_command?, token_budget? }` /
> `exit_goal_mode` 映射到 `reflect_enter_goal_mode` / `reflect_exit_goal_mode`
> (`commands/agent.rs` / `commands/goal.ts`)。进入目标模式后,core 侧
> `GoalController` 在每轮 turn 结束自校验(LM judge + 可选校验命令),未完成则
> steering 自动续作 —— **不需要新的用户提交**;GUI 经常规 turn/流式事件呈现
> 多轮续作,事件侧无 goal 专属变体。

### 2.0 Op 之外的命令（诊断 / 配置 / 域 I/O）

这些命令不对应 `Op` 变体；它们与 Op 派生的命令共存于相同的域模块中：

| 命令 | 负载 | 返回 | 位置（Rust）/ 位置（TS）|
|---|---|---|---|
| `ping` | `null` | `{ msg, version }` | `src-tauri/src/commands/agent.rs` (barrel) / `src/utils/commands/health.ts` |
| `reflect_agent_status` | `null` | `{ ready, has_model, model, workspace, degraded_reason }` | `commands/agent.rs` / `commands/agent.ts` |
| `reflect_get_config` | `null` | `string` (TOML) | `commands/config.rs` / `commands/config.ts` |
| `reflect_save_config` | `{ toml }` | `null` | `commands/config.rs` / `commands/config.ts` |
| `reflect_list_tools` | `null` | `Vec<{ name, description }>` | `commands/agent.rs` / `commands/agent.ts` |
| `reflect_list_sessions` / `reflect_rename_session` / `reflect_delete_session` / `reflect_replay_session` | `{ workspace?, limit?, offset? }` / — / — / — | `SessionInfo[]` / — / — / — | `commands/sessions.rs` / `commands/sessions.ts`。`workspace` 为可选过滤（`SessionInfo.workspace` 精确匹配；旧会话 `workspace = None` 仅在全量列表出现） |
| `reflect_create_session` | `null` | `string`（ThreadId） | `commands/sessions.rs` / `commands/sessions.ts`。纯 ID 分配（New Chat 先拿路由 id），不触发后端绑定；会话文件在首条 `UserInput` 时由后端物化并写 `SessionMeta.workspace`（workspace 值取自任意类型首条 `Submission` 的顶层 `workspace`，缺省回退 `cfg.current_workspace()`） |
| `reflect_bind_session` | `{ id }` | `null` | `commands/sessions.rs` / `commands/sessions.ts`。把后端 `AgentThread` 换绑到指定 session id：replay 该 id 的 JSONL → `records_to_preload` → 重建线程（`with_session_id` + M4 recorder 同 id）。**后端幂等**（同 id 短路）；未知 id（刚 `reflect_create_session` 分配、未落盘）走空历史分支不报错。ChatView 挂载每个会话都调一次（bind → replay → hydrate 序列）；旧线程经 `cancel_token` 有界排空，cron driver 换 sender 后重启（job uuid 保留） |
| `reflect_archive_session` / `reflect_unarchive_session` / `reflect_list_archived_sessions` | `{ id }` / `{ id }` / `null` | `null` / `null` / `SessionInfo[]` | `commands/sessions.rs` / `commands/sessions.ts`（归档树 `~/.reflect/sessions-archive`，整树搬移、相对路径不变） |
| `reflect_export_session` / `reflect_export_session_markdown` | `{ id }` / `{ id }` | `string` (path) / `string` (markdown) | `commands/export.rs` / `commands/sessions.ts` |
| `reflect_git_status` / `reflect_git_diff` / `reflect_git_log` | — | `GitStatus` / `string` / `Vec<GitLogEntry>` | `commands/git.rs` / `commands/git.ts` |
| `reflect_run_shell` | `{ cmd }` | `ShellSession { id, command, cwd }` | `commands/shell.rs` / `commands/terminal.ts`; `reflect_terminal_output` event streams output |
| `reflect_kill_shell` | `{ session_id }` | `null` | `commands/shell.rs` / `commands/terminal.ts` |
| `reflect_list_shell_sessions` | `null` | `string[]` | `commands/shell.rs` / `commands/terminal.ts` |
| `reflect_list_dir` / `reflect_read_file` / `reflect_search_files` | `{ path?, maxDepth? }` / `{ path }` / `{ query, ... }` | `DirListing` / `FileReadResult` / search results | `commands/files.rs` / `commands/files.ts`; `commands/search.rs` / `commands/search.ts` |
| `reflect_load_allowlist` / `reflect_save_allowlist` / `reflect_check_allowlist` | — | allowlist payload | `commands/allowlist.rs` / `commands/allowlist.ts` |
| `reflect_list_workspaces` / `reflect_set_workspace` / `reflect_current_workspace` | — | workspace payloads（`WorkspaceInfo { path, label, last_used, session_count }`，历史持久化于 `~/.reflect/workspaces.json`） | `commands/workspaces.rs` / `commands/workspaces.ts` |
| `reflect_pick_workspace_folder` | `null` | `string?`（原生目录选择框，取消 → `null`；走 `tauri-plugin-dialog` Rust API，不经 JS IPC） | `commands/workspaces.rs` / `commands/workspaces.ts` |
| `reflect_reveal_path` | `{ path }` | `null`（在 Finder / 资源管理器 / xdg-open 中定位） | `commands/workspaces.rs` / `commands/workspaces.ts` |
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
    pub workspace: Option<String>,   // v1.x：会话归属（首条 UserInput 时写入 SessionMeta.workspace）
}
```

`workspace` 带 `#[serde(default, skip_serializing_if = "Option::is_none")]` —— 旧线格式不写该键，
后端回退 `cfg.current_workspace()`；GUI 端由 `useCurrentWorkspace` 注入当前激活工作区。

### 2.1 `UserInputItem`（`reflect-agent/crates/protocol/reflect-protocol/src/item.rs`）

| 变体 | 字段 | 前端来源 |
|---|---|---|
| `Text` | `text: String` | 输入框文本 |
| `Image` | `data: Vec<u8>, mime_type: String` | 拖拽或粘贴（前端 base64 解码） |
| `LocalImage` | `path: PathBuf` | 文件选择器（后端从磁盘读取） |
| `Skill` | `name: String, args: Option<JSON>` | `/skill name` 或技能选择器 |
| `QuestionAnswer` | `request_id, answers` | LLM 提问的回复（`AskUserQuestionResponse` 的替代路径） |
| `File` | `path: String, range?: FileRange` | Composer `@` 文件弹层（`MentionPicker`）。`path` 为相对工作区 POSIX 路径；`FileRange { start_line, end_line }` 可选。core 侧 `user_input_items_to_messages` 展开为 `@<path>` / `@<path>:L<n>-L<m>` 文本块，真实读取由 LLM `/read` 工具按需触发 |

`File` 为纯新增 variant（旧 reader 不受影响）；`SessionInfo.workspace` / `RolloutRecord::SessionMeta.workspace`
同样带 serde 兼容注解 —— 旧 JSONL 反序列化为 `None`，前端按"未归属"展示。

### 2.2 提交 ID 生命周期

- 前端生成 UUID v4 → `Submission.id`。
- 后端从 `reflect_submit` 返回相同 id（回显）。
- 前端将入站 `Event.id` 与原始 `Submission.id` 关联以解锁输入器 / 标记 turn 完成。
- `EVENT_ID_NONE = ""` 事件（生命周期：`SessionConfigured`、`ShutdownComplete`、`PermissionModeChanged` …）没有对应的提交。
- `SessionConfigured.session_id`（v1.x 起）等于线程预分配 id：与 rollout 文件名
  （`~/.reflect/sessions/YYYY/MM/DD/<id>.jsonl`）、`SessionMeta.session_id` 及
  前端路由 id 一致（`reflect_bind_session` 经 `with_session_id` 注入，`submission_loop`
  在 `SessionConfiguredEvent::new` 后覆盖 `sc.session_id = session_id`）。旧版本曾发出
  随机 UUID，仅作 turn 事件标识。

---

## 3. 事件（后端 → UI）

`EventMsg` 使用 `#[serde(tag = "type", rename_all = "snake_case")]`（变体列表位于 `reflect-agent/crates/protocol/reflect-protocol/src/event_msg.rs`）。

> **命名**：Rust `enum` 使用 **PascalCase 结构变体**（如 `TurnStarted(TurnStartedEvent)`），但 serde 发出 **snake_case** 鉴别器（`turn_started`）供前端使用。前端 TypeScript 使用 snake_case 形式。

### 3.1 Lifecycle (5)

| 变体 | 触发方 | UI 影响 |
|---|---|---|
| `session_configured` | First turn of a thread | Show banner; persist `session_id` |
| `turn_started` | `Op::UserInput` accepted | Spinner on; clear composer draft |
| `turn_complete` | LLM turn finished | Spinner off; unlock composer; accumulate cost |
| `turn_aborted` | `Op::Interrupt` / error | Red toast |
| `turn_rewound` | `Op::Rewind` success | Truncate local scrollback; redraw |
| `shutdown_complete` | `Op::Shutdown` | Quit / hide |

### 3.2 LLM output (4)

| 变体 | 字段 | UI |
|---|---|---|
| `agent_message` | `text: String` | Non-streaming final bubble |
| `agent_message_delta` | `delta: String` | Stream chunk → append to last assistant bubble |
| `thinking_delta` | `delta: String` | Reasoning block (collapsible) |
| `token_count` | `TokenCountEvent` (info) | StatusBar token/cost indicator + Inspector "Token Usage" section (input/output/cached/cache_write/total/cost/provider/credential). `TokenCountEvent` 字段：`input_tokens`、`output_tokens`、`cached_tokens`、`cache_write_tokens`（M8，input 子集，不计入 total）、`total_tokens`、`cost_usd?`、`provider?`、`credential_label?` |

### 3.3 Tools (3)

| 变体 | 字段 | UI |
|---|---|---|
| `tool_call_begin` | `ToolCallBeginEvent` | Tool row running indicator |
| `tool_call_end` | `ToolCallEndEvent` | Tool row done / failed / cancelled |
| `tool_execution_request` | `ToolExecutionRequestEvent` (v1.3 SDK) | none —— serve 模式下 core 请求客户端本地执行其经 `Op::RegisterTools` 注册的远程工具（回执 `Op::ToolExecutionResponse`，`call_id` 配对）。Desktop 内嵌 AgentThread 不注册远程工具，reducer no-op 保穷尽 |

### 3.4 Approvals / AskUser (4)

| 变体 | 前端响应 | 配对 Op |
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

## 4. ID 配对规则

| 事件 | 触发 Op | 字段 | 备注 |
|---|---|---|---|
| `approval_request` (工具) | `tool_approval` | `id = request_id` | |
| `approval_request` (钩子) | `hook_approval` | `id = request_id` | |
| `approval_request` (计划) | `plan_approval` | `id = request_id` | |
| `ask_user_question` | `ask_user_question_response` | `id = sub_id` | 每个问题有独立的 sub_id（每问题等待器） |
| `ask_user_input` | `ask_user_input_response` | `id = sub_id` | |
| `plan_request` | `enter_plan_mode` | `task` 回显 | 客户端发送前确认 |
| `plan_ready` | `plan_approval` | `id = plan_id` | |
| `*_delta` / `*_complete` | （任何用户操作） | `id = submission.id` | 流式块共享提交 id |

---

## 5. 版本兼容性

- **仅追加式演化**：新变体不会破坏现有代码（反序列化器忽略未知字段）。
- **Snake_case 标签**防止 Rust 重命名重构泄漏到线格式。
- 可选字段上的 **`#[serde(default)]`**在发布期间保护前端免受缺失键的影响。
- **前端类型重新生成**：当 `reflect-protocol` 变更时，运行：

  ```bash
  # 在 Reflect-Agent 仓库中：
  cargo run -p reflect-protocol --example dump_schema > /tmp/reflect-schema.json
  # 在 ReflectDesktop 中：
  npx json2ts /tmp/reflect-schema.json -o src/types/protocol.ts
  ```

- **子模块升级**：`git submodule update --remote reflect-agent` 拉取最新核心 crate（见 `SUBMODULE.md`）。升级后，重新运行 dump → json2ts 流水线。

---

## 6. 前端 TypeScript 接口

`src/types/protocol.ts` 是规范的镜像。分为两部分：

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
  | { type: 'enter_goal_mode'; goal: string; verify_command?: string; token_budget?: number }
  | { type: 'exit_goal_mode' }
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

### 6.1 桥接模式

前端 IPC 层分为三个角色：

1. **底层桥接** — `src/utils/bridge.ts` 导出 `invoke` / `listen`，带 Tauri 上下文回退和 `isMissingTauriInvokeError` 防护。
2. **域包装器** — `src/utils/commands/{health,agent,approvals,plan,permissions,questions,config,sessions,events,workspaces,skills,memory,hooks,git,terminal,files,allowlist,updates,search}.ts` re-export 类型化包装器。`src/utils/commands/index.ts` 桶文件聚合它们。
3. **兼容桶文件** — `src/utils/tauri.ts` 和 `src/utils/commands.ts` 从 `./bridge`、`./commands`、`./types` re-export，使现有的 `@/utils/tauri` 和 `@/utils/commands` 导入继续工作。**新代码应直接从 `@/utils/commands/{domain}` 或 `@/utils/bridge` 导入。**

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

### 6.2 Reducer 模式

`src/services/agentEventBus.ts`（引用计数总线）转发至 `src/stores/agent/reducer.ts` —— `ReflectEventMsg` 的规范 reducer。`src/services/agent.ts` 保留为兼容性 re-export，将 `useAgent` / `handle_event` 转发到 store。`src/stores/agentStore.ts` 是用户可见的兼容门面，从 `./agent` re-export `useAgentStore` / `reduceEvent` / 类型。新增变体：

1. 在 `src/types/protocol.ts` 添加 `ReflectEventMsg` 分支。
2. 在 `src/stores/agent/reducer.ts` 添加 `case 'new_variant':`。
3. 在 `src/stores/agentStore.test.ts` 和/或总线测试中覆盖单元测试。

---

## 7. 故障模式

| 症状 | 原因 | 修复 |
|---|---|---|
| 前端收到 `unknown variant` | 协议已添加但 TS 未重新生成 | 重新运行 `dump_schema` + `json2ts` |
| 提交被静默丢弃 | 后端反序列化错误 | 检查 `src-tauri/src/commands/<domain>.rs` 中的 `tracing` 日志 |
| 事件携带过时 id | 提交 id 被覆盖 | 仅使用 `crypto.randomUUID()` |
| 审批模态框不关闭 | 前端忘记派发 `*_approval` Op | 检查 `src/features/modals/ModalShell.tsx` `onSubmit` |
| `EVENT_ID_NONE` 配对为提交 | 生命周期事件按 id 匹配 | 在 `src/stores/agent/reducer.ts` 过滤 `e.id === ''` |

---

## 8. 参考

- Rust 真相源：`reflect-agent/crates/protocol/reflect-protocol/src/{event,event_msg,op,item,submission}.rs`
- TS 镜像：`src/types/protocol.ts`
- Tauri 命令注册表：`src-tauri/src/lib.rs`（处理程序列表）+ `src-tauri/src/commands/mod.rs`（桶文件）+ `src-tauri/src/commands/<domain>.rs` 中各域主体
- 事件转发器：`src-tauri/src/events.rs::forward_agent_events`
- 前端事件扇出：`src/services/agentEventBus.ts` → `src/stores/agent/reducer.ts`；`src/services/agent.ts` 中的兼容 re-export
- IPC 底层桥接：`src/utils/bridge.ts`
- IPC 包装器（规范，按域）：`src/utils/commands/<domain>.ts` 由 `src/utils/commands/index.ts` 聚合
- IPC 兼容桶文件：`src/utils/tauri.ts`、`src/utils/commands.ts`
- Agent store：`src/stores/agent/`（规范实现）带兼容门面 `src/stores/agentStore.ts`
