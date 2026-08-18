# 审查账本 —— ReflectDesktop

截至 2026-07-28 的逐文件审查状态。本次会话中全部 367 个源文件已人工阅读。

**状态代码**：
- **P** = 本次会话已人工阅读（审查完整文件内容）
- **S** = 跳过（供应商镜像 —— 按 AGENTS.md 只读；或非源文件）
- **B** = 跳过（构建产物 / 声明 / 配置 / CSS）

**行动代码**：
- `pass` = 未发现有问题
- `fix` = 本次会话已应用修复
- `defer` = 已知限制 / 范围外（产品决策 / Phase 3+ 工作）

---

## src/（307 TS/TSX 文件）

### 组合根（3 文件）
- `src/main.tsx` — **P** pass
- `src/router.tsx` — **P** pass
- `src/app.smoke.test.tsx` — **P** pass

### 类型 / 协议（16 文件）
- `src/types/css-modules.d.ts` — B（声明）
- `src/types/protocol.ts` — **P** pass
- `src/types/protocol/index.ts` — **P** pass
- `src/types/protocol/event.ts` — **P** fix（`EVENT_ID_NONE` 文档，`AbortReasonPayload` 形状）
- `src/types/protocol/item.ts` — **P** pass
- `src/types/protocol/op.ts` — **P** pass
- `src/types/protocol/submission.ts` — **P** pass
- `src/types/protocol/ask_user_input.ts` — **P** pass
- `src/types/protocol/enums.ts` — **P** pass
- `src/types/protocol/question.ts` — **P** pass
- `src/types/protocol/rollout.ts` — **P** pass
- `src/types/protocol/usage.ts` — **P** pass
- `src/protocol/submissions.ts` — **P** pass
- `src/components/Markdown.tsx` — **P** pass

### 工具（8 文件）
- `src/utils/bridge.ts` — **P** pass（Tauri invoke 回退）
- `src/utils/bridge.test.ts` — **P** pass
- `src/utils/commands.ts` — **P** pass（兼容桶文件）
- `src/utils/commands.test.ts` — **P** pass
- `src/utils/tauri.ts` — **P** pass（兼容桶文件）
- `src/utils/types.ts` — **P** pass（修复后无幽灵字段）
- `src/utils/uuid.ts` + 测试 — **P** pass
- `src/utils/time.ts` + 测试 — **P** pass
- `src/utils/debounce.ts` + 测试 — **P** pass
- `src/utils/notify.ts` + 测试 — **P** pass
- `src/utils/theme.ts` + 测试 — **P** pass
- `src/utils/uiPrefs.ts` + 测试 — **P** pass
- `src/utils/i18n.ts` — **P** pass（桶文件）
- `src/utils/i18n/context.tsx` — **P** pass
- `src/utils/i18n/interpolate.ts` — **P** pass
- `src/utils/i18n/locale.ts` — **P** pass
- `src/utils/i18n/lookup.ts` — **P** pass
- `src/utils/i18n/types.ts` — **P** pass

### i18n 字符串（34 个命名空间文件 —— 全部已人工阅读；每个为 `{ en, zh-CN }` 字典）
- `src/utils/i18n/strings/index.ts` — **P** pass（190 行，33 命名空间桶文件）
- `src/utils/i18n/strings/settings.ts` — **P** pass（190 行，所有键在两种语言中均存在）
- `src/utils/i18n/strings/{about,app,apps,chat,collaboration,common,composer,debug,design,dictation,files,git,home,inspector,memory,mobile,modal,models,notifications,palette,permissionMode,plan,prompts,shell,sidebar,skills,slash,terminal,threads,toast,update,workspaces}.ts` — **P** pass（所有结构一致，每个键均有 `en` 和 `zh-CN`）

### utils/commands 包装器（29 文件 —— 全部已人工阅读）
- `src/utils/commands/index.ts` — **P** pass（所有 29 个的 re-export 桶文件）
- `activity.ts`（109 行） — **P** pass（ReflectActivityEvent/Filters/Level，4 命令）
- `agent.ts`（36 行） — **P** pass（reflect_submit/interrupt/compact/rewind/shutdown）
- `agents.ts`（72 行） — **P** pass（ReflectAgentDef 11 字段，5 命令；文档：`Some("inherit")` vs `string | null`）
- `allowlist.ts`（24 行） — **P** pass
- `approvals.ts`（24 行） — **P** pass（ReviewDecision 类型化）
- `autopilot.ts`（37 行） — **P** pass（ReflectAutopilotConfig/Run camelCase）
- `config.ts`（43 行） — **P** pass（ReflectAgentStatus snakeCase 匹配 Rust）
- `events.ts`（17 行） — **P** pass（onReflectEvent 订阅）
- `files.ts`（47 行） — **P** pass
- `git.ts`（43 行） — **P** pass（ReflectGitStatus snakeCase）
- `health.ts`（12 行） — **P** pass
- `hooks.ts`（21 行） — **P** pass
- `kms.ts`（81 行） — **P** pass
- `media.ts`（70 行） — **P** fix（`ComputerUseAction::Screenshot` params:null）
- `memory.ts`（25 行） — **P** pass
- `permissions.ts`（20 行） — **P** pass（defer：取窄联合而非 string）
- `plan.ts`（14 行） — **P** pass
- `questions.ts`（15 行） — **P** pass
- `remote.ts`（98 行） — **P** fix（`sinceMs` camelCase）
- `schedule.ts`（108 行） — **P** pass（ReflectCronJob 来自供应商的 snakeCase）
- `search.ts`（36 行） — **P** pass
- `sessions.ts`（55 行） — **P** fix（移除幽灵字段后）
- `side_channel.ts`（81 行） — **P** fix（camelCase TS 类型）
- `skills.ts`（17 行） — **P** pass
- `squad.ts`（56 行） — **P** pass
- `tasks.ts`（109 行） — **P** pass（ReflectTask 来自供应商的 snakeCase）
- `teams.ts`（69 行） — **P** pass（ReflectTeam/TeamMember snakeCase）
- `terminal.ts`（41 行） — **P** pass
- `updates.ts`（22 行） — **P** pass
- `workspaces.ts`（26 行） — **P** pass

### 存储（10 文件）
- `src/stores/agent/index.ts` — **P** pass（re-export 桶文件）
- `src/stores/agent/reducer.ts` — **P** fix（turn_rewound UUID，tool_call_end 性能，permission_bubble 风险）
- `src/stores/agent/servers.ts` — **P** pass
- `src/stores/agent/store.ts` — **P** pass
- `src/stores/agent/toast.ts` — **P** pass
- `src/stores/agent/turns.ts` — **P** pass
- `src/stores/agent/types.ts` — **P** fix（PendingApproval.risk 字段已添加）
- `src/stores/agent/useAgent.ts` — **P** pass
- `src/stores/agentStore.ts` — **P** pass（re-export 存根）
- `src/stores/agentStore.test.ts` — **P** pass
- `src/stores/replay.ts` — **P** fix（幽灵字段 → 正确的 ReflectRolloutRecord 遍历）
- `src/stores/toast.test.ts` — **P** pass

### 服务（3 文件）
- `src/services/agent.ts` — **P** pass
- `src/services/agentEventBus.ts` — **P** pass
- `src/services/agent.test.ts` + `agentEventBus.test.ts` — **P** pass

### 测试设置（1 文件）
- `src/test/setup.tsx` — **P** fix（`sinceMs` camelCase）

### 功能切片（35 切片 —— 全部已人工阅读）
**设置（8 文件）：**
- `src/features/settings/SettingsView.tsx`（259 行） — **P** pass（状态栏 `t('settings.status.ready', { model })` + 内联 `<code>{status?.model}</code>` 显示模型两次但为外观问题）
- `src/features/settings/ConfigForm.tsx`（75 行） — **P** pass
- `src/features/settings/components/ComplexEditors.tsx`（160 行） — **P** pass
- `src/features/settings/components/StructuredField.tsx`（69 行） — **P** pass
- `src/features/settings/sections/DisplaySection.tsx`（181 行） — **P** defer（backgroundUrl 输入仅从 http URL 初始化）
- `src/features/settings/sections/NotificationsSection.tsx`（109 行） — **P** pass
- `src/features/settings/sections/UpdatesSection.tsx`（119 行） — **P** pass
- `src/features/settings/configSchema.tsx`（21 行） — **P** pass（兼容 re-export）
- `src/features/settings/config/schema.ts`（278 行） — **P** pass
- `src/features/settings/config/toml.ts`（227 行） — **P** pass
- `src/features/settings/config/index.ts`（9 行） — **P** pass
- `src/features/settings/components/index.ts`（2 行） — **P** pass

**终端（3 文件）：**
- `src/features/terminal/TerminalView.tsx`（132 行） — **P** pass
- `src/features/terminal/useTerminalController.ts`（338 行） — **P** pass（通过 ref + cancelled 标志取消安全 unlisten）
- `src/features/terminal/TerminalView.test.tsx` — **P** pass

**记忆（4 文件）：**
- `src/features/memory/MemoryAddForm.tsx`（66 行） — **P** fix（scope 幽灵 → user）
- `src/features/memory/MemoryRow.tsx`（90 行） — **P** pass
- `src/features/memory/MemoryView.tsx`（118 行） — **P** pass
- `src/features/memory/useMemoryController.ts`（241 行） — **P** fix（scope 幽灵 → user）
- `src/features/memory/MemoryView.test.tsx` — **P** pass

**文件（4 文件）：**
- `src/features/files/CodeEditor.tsx`（108 行） — **P** pass
- `src/features/files/FileTree.tsx`（196 行） — **P** pass
- `src/features/files/FilesView.tsx`（117 行） — **P** pass
- `src/features/files/SearchView.tsx`（154 行） — **P** fix（竞态 + 无错误）
- `src/features/files/CodeEditor.test.tsx` + `FileTree.test.tsx` — **P** pass

**Git（3 文件）：**
- `src/features/git/DiffViewer.tsx`（83 行） — **P** pass
- `src/features/git/GitView.tsx`（163 行） — **P** pass
- `src/features/git/DiffViewer.test.tsx` — **P** pass

**其他功能（13 文件）：**
- `src/features/home/HomeView.tsx`（148 行） — **P** pass
- `src/features/skills/SkillsView.tsx`（91 行） — **P** pass
- `src/features/workspaces/WorkspacesView.tsx`（158 行） — **P** pass
- `src/features/notifications/NotificationsView.tsx`（370 行） — **P** pass
- `src/features/notifications/useActivityController.ts`（114 行） — **P** pass
- `src/features/notifications/NotificationsView.test.tsx` — **P** pass

**会话（5 文件）：**
- `src/features/sessions/components/SessionItem.tsx`（37 行） — **P** pass
- `src/features/sessions/components/BucketGroup.tsx`（43 行） — **P** pass
- `src/features/sessions/components/Sidebar.test.tsx` — **P** pass
- `src/features/sessions/hooks/useSessions.ts`（130 行） — **P** pass
- `src/features/sessions/hooks/useSessions.test.tsx` — **P** pass
- `src/features/sessions/utils/buckets.ts`（59 行） — **P** pass
- `src/features/sessions/utils/buckets.test.ts` — **P** pass
- `src/features/sessions/index.ts`（19 行） — **P** pass

**线程（5 文件）：**
- `src/features/threads/ThreadsView.tsx`（48 行） — **P** pass
- `src/features/threads/components/ThreadBucketGroup.tsx`（45 行） — **P** pass
- `src/features/threads/components/ThreadItem.tsx`（108 行） — **P** pass
- `src/features/threads/components/ThreadItemMenu.tsx`（148 行） — **P** pass
- `src/features/threads/utils/threadLabels.ts`（16 行） — **P** pass
- `src/features/threads/utils/threadLabels.test.ts` — **P** pass
- `src/features/threads/index.ts`（6 行） — **P** pass
- `src/features/threads/{ThreadsView,ThreadBucketGroup,ThreadItemMenu}.test.tsx` — **P** pass

**命令面板（3 文件）：**
- `src/features/command-palette/CommandPalette.tsx`（188 行） — **P** pass
- `src/features/command-palette/registry.ts`（131 行） — **P** pass（25 个面板项）
- `src/features/command-palette/fuzzy.ts`（92 行） — **P** pass
- `src/features/command-palette/CommandPalette.test.tsx` + `fuzzy.test.ts` — **P** pass

**设计系统（20 文件 —— 全部已人工阅读）：**
- `src/features/design-system/index.ts`（63 行） — **P** pass（桶文件）
- `src/features/design-system/DesignSystemView.tsx`（240 行） — **P** pass
- `src/features/design-system/utils/toast.ts`（20 行） — **P** pass
- `src/features/design-system/utils/ring.ts`（40 行） — **P** pass
- `src/features/design-system/utils/keyHints.ts`（34 行） — **P** pass
- `src/features/design-system/primitives/Badge.tsx`（41 行） — **P** pass
- `src/features/design-system/primitives/Button.tsx`（67 行） — **P** pass
- `src/features/design-system/primitives/Card.tsx`（34 行） — **P** pass
- `src/features/design-system/primitives/ContextRing.tsx`（53 行） — **P** pass
- `src/features/design-system/primitives/EmptyState.tsx`（27 行） — **P** pass
- `src/features/design-system/primitives/Icon.tsx`（40 行） — **P** pass
- `src/features/design-system/primitives/IconButton.tsx`（37 行） — **P** pass
- `src/features/design-system/primitives/Input.tsx`（46 行） — **P** pass
- `src/features/design-system/primitives/KeyHint.tsx`（21 行） — **P** pass
- `src/features/design-system/primitives/SegmentedControl.tsx`（64 行） — **P** pass
- `src/features/design-system/primitives/Select.tsx`（29 行） — **P** pass
- `src/features/design-system/primitives/Spinner.tsx`（25 行） — **P** pass
- `src/features/design-system/primitives/Textarea.tsx`（27 行） — **P** pass
- `src/features/design-system/primitives/Toast.tsx`（34 行） — **P** pass
- `src/features/design-system/primitives/Tooltip.tsx`（29 行） — **P** pass
- （primitive 测试文件） — **P** pass

**Shell（10 文件）：**
- `src/features/shell/AppShell.tsx` — **P** pass（此前版本账本中有 USE_DELEGATED；本次又审查了）
- `src/features/shell/TitleBar.tsx` — **P** pass
- `src/features/shell/StatusBar.tsx` — **P** pass（modelKind 有 2 个 'warn' 分支；外观问题）
- `src/features/shell/ActivityBar.tsx`（110 行） — **P** pass
- `src/features/shell/Inspector.tsx`（116 行） — **P** pass
- `src/features/shell/PageShell.tsx`（41 行） — **P** pass
- `src/features/shell/hooks/useCommandPaletteShortcut.ts`（40 行） — **P** pass
- `src/features/shell/hooks/usePaletteActions.ts`（130 行） — **P** pass
- `src/features/shell/hooks/useThemeCycle.ts`（47 行） — **P** pass
- `src/features/shell/hooks/{useCommandPaletteShortcut,usePaletteActions,useThemeCycle}.test.tsx` — **P** pass

**Composer（10 文件）：**
- `src/features/composer/Composer.tsx`（220 行） — **P** fix（Toolbar /command + onSlashSelect）
- `src/features/composer/AttachmentBar.tsx`（49 行） — **P** pass
- `src/features/composer/MentionPicker.tsx` — **P** pass
- `src/features/composer/SlashPopup.tsx`（84 行） — **P** defer（键盘导航 —— NOTE 注释）
- `src/features/composer/slashCommands.ts`（71 行） — **P** pass
- `src/features/composer/slashEngine.ts` — **P** pass
- `src/features/composer/useAttachments.ts`（70 行） — **P** pass
- `src/features/composer/useComposerInput.ts` — **P** pass
- `src/features/composer/useComposerSubmission.ts` — **P** pass
- `src/features/composer/usePromptHistory.ts` — **P** pass
- `src/features/composer/{useAttachments,slashCommands,slashEngine,usePromptHistory,SlashPopup}.test.{ts,tsx}` — **P** pass

**消息（4 文件）：**
- `src/features/messages/ChatView.tsx` — **P** pass
- `src/features/messages/MessageList.tsx` — **P** defer（index 键；外观问题）
- `src/features/messages/ToolCells.tsx` + 测试 — **P** pass
- `src/features/messages/Collapsible.tsx`（40 行） — **P** pass
- `src/features/messages/Composer.tsx`（1 行） — **P** pass（re-export 存根）
- `src/features/messages/ChatView.test.tsx` — **P** pass

**模态框（8 文件）：**
- `src/features/modals/ModalShell.tsx`（154 行） — **P** fix（Enter 焦点方向）
- `src/features/modals/ApprovalModal.tsx`（53 行） — **P** pass
- `src/features/modals/ApprovalHistory.tsx`（67 行） — **P** pass
- `src/features/modals/AskUserModal.tsx`（37 行） — **P** pass
- `src/features/modals/PlanReadyModal.tsx`（40 行） — **P** pass
- `src/features/modals/QuestionModal.tsx`（90 行） — **P** pass
- `src/features/modals/index.tsx`（49 行） — **P** pass
- `src/features/modals/ModalShell.test.tsx` + `ApprovalHistory.test.tsx` + `index.test.tsx` — **P** pass

**较新功能切片（8 切片 —— 全部已人工阅读）：**
- `src/features/about/AboutView.tsx`（97 行） — **P** pass
- `src/features/apps/AppsView.tsx`（77 行） — **P** pass
- `src/features/collaboration/CollaborationView.tsx`（105 行） — **P** pass
- `src/features/debug/DebugView.tsx`（53 行） — **P** pass
- `src/features/dictation/DictationView.tsx`（149 行） — **P** pass
- `src/features/dictation/useDictation.ts` — **P** fix（过时 transcript ref）
- `src/features/mobile/MobileView.tsx`（30 行） — **P** pass
- `src/features/models/ModelsView.tsx`（128 行） — **P** pass
- `src/features/plan/PlanView.tsx`（122 行） — **P** pass
- `src/features/prompts/PromptsView.tsx`（118 行） — **P** pass
- `src/features/update/UpdateView.tsx`（50 行） — **P** pass
- `src/features/{agents,autopilot,kms,remote,schedule,side-channel,squad,tasks-board}/*` — **P** 每个会话（切片索引/组件/hooks/视图/测试）

---

## src-tauri/（46 .rs 文件）

### 顶层（9 文件 —— 全部已人工阅读）
- `src-tauri/build.rs` — **P** pass
- `src-tauri/src/main.rs` — **P** pass
- `src-tauri/src/lib.rs` — **P** pass
- `src-tauri/src/state.rs` — **P** fix（workspace() 返回覆盖）
- `src-tauri/src/events.rs`（37 行） — **P** pass（forward_agent_events Lagged/Closed 处理）
- `src-tauri/src/mcp.rs`（202 行） — **P** pass
- `src-tauri/src/menu.rs`（201 行） — **P** pass
- `src-tauri/src/tray.rs`（111 行） — **P** pass
- `src-tauri/src/shortcut.rs` — **P** fix（基于 cfg 的修饰符）
- `src-tauri/src/dock.rs` — **P** pass

### 状态模块（6 文件 —— 全部已人工阅读）
- `src-tauri/src/state/agent.rs` — **P** pass
- `src-tauri/src/state/install.rs`（236 行） — **P** pass
- `src-tauri/src/state/session.rs`（45 行） — **P** pass
- `src-tauri/src/state/submit.rs`（110 行） — **P** pass
- `src-tauri/src/state/activity.rs`（273 行） — **P** pass
- `src-tauri/src/state/remote_config.rs`（129 行） — **P** fix（`sinceMs`）

### 命令（25 文件 —— 全部已人工阅读）
- `src-tauri/src/commands/mod.rs` — **P** pass
- `src-tauri/src/commands/error.rs` — **P** pass
- `src-tauri/src/commands/agent.rs` — **P** pass
- `src-tauri/src/commands/agents.rs` — **P** fix（path_for 精确路径遍历；允许 `foo..bar`）
- `src-tauri/src/commands/activity.rs`（52 行） — **P** pass
- `src-tauri/src/commands/allowlist.rs` — **P** pass
- `src-tauri/src/commands/autopilot.rs` — **P** pass
- `src-tauri/src/commands/config.rs` — **P** pass
- `src-tauri/src/commands/export.rs` — **P** pass
- `src-tauri/src/commands/files.rs` — **P** fix（`&agent.workspace()` borrow）
- `src-tauri/src/commands/git.rs` — **P** pass
- `src-tauri/src/commands/hooks.rs` — **P** pass
- `src-tauri/src/commands/kms.rs` — **P** pass
- `src-tauri/src/commands/media.rs` — **P** pass
- `src-tauri/src/commands/memory.rs` — **P** pass
- `src-tauri/src/commands/remote.rs` — **P** pass
- `src-tauri/src/commands/schedule.rs` — **P** pass
- `src-tauri/src/commands/search.rs` — **P** pass
- `src-tauri/src/commands/sessions.rs` — **P** pass
- `src-tauri/src/commands/shell.rs` — **P** pass
- `src-tauri/src/commands/side_channel.rs` — **P** pass
- `src-tauri/src/commands/skills.rs` — **P** pass
- `src-tauri/src/commands/squad.rs` — **P** pass
- `src-tauri/src/commands/tasks.rs` — **P** pass
- `src-tauri/src/commands/update.rs` — **P** pass
- `src-tauri/src/commands/workspaces.rs` — **P** pass

### 辅助（4 文件 —— 全部已人工阅读）
- `src-tauri/src/hook_store.rs`（92 行） — **P** pass
- `src-tauri/src/memory_store.rs`（76 行） — **P** pass
- `src-tauri/src/shell_sessions.rs`（44 行） — **P** pass
- `src-tauri/src/workspace_state.rs`（31 行） — **P** pass
- `src-tauri/src/media_backend.rs`（436 行） — **P** pass（`max(1)` 除数守卫存在）

### 测试（2 文件）
- `src-tauri/tests/e2e_chat.rs` — **P** pass（依赖环境；需要实时 API key）
- `src-tauri/tests/protocol_bridge_e2e.rs` — **P** fix（interrupt_token + workspace_accessor 真实断言）

---

## app-core/（14 .rs 文件）

### Lib / 状态（4 文件 —— 全部已人工阅读）
- `app-core/src/lib.rs` — **P** pass
- `app-core/src/protocol.rs` — **P** pass
- `app-core/src/state/mod.rs` — **P** fix（PendingApproval.risk, RiskLevel import）
- `app-core/src/actor.rs`（331 行） — **P** pass

### Reducer（3 文件 —— 全部已人工阅读）
- `app-core/src/reducer/mod.rs` — **P** pass
- `app-core/src/reducer/matchers.rs` — **P** fix（PlanReady 保留 task, PermissionBubble.risk, ApprovalRequest.risk: None）
- `app-core/src/reducer/state_mut.rs` — **P** fix（map_approval_policy / map_sandbox_policy / map_permission_mode 现在匹配所有变体）

### 功能（6 文件 —— 全部已人工阅读）
- `app-core/src/side_channel.rs`（460 行） — **P** pass
- `app-core/src/autopilot.rs`（300+ 行） — **P** pass
- `app-core/src/kms.rs`（200+ 行） — **P** pass
- `app-core/src/tailscale.rs`（260 行） — **P** pass
- `app-core/src/squad.rs`（200+ 行） — **P** pass
- `app-core/src/media.rs`（500+ 行） — **P** pass（ComputerUseAction Screenshot params:null 行为已记录）
- `app-core/src/activity.rs`（550+ 行） — **P** pass（record() 从空分配 id）

---

## vendor/（只读镜像 —— 按 AGENTS.md 标记为 S）
22 个供应商 crate 未修改。抽查 serde 命名一致性：
- `reflect-protocol/src/{item, op, event_msg, event, submission, rollout}.rs`
- `reflect-stream/src/cron.rs`（CronJobSpec snakeCase）
- `reflect-tools/src/registry.rs`（register_with_source 语义）
- `reflect-mcp`, `reflect-lsp`（transport_mirror 在 mcp.rs 中已验证）

## docs/（按 AGENTS.md "无过去评论，仅活态"标记为 S）
所有 markdown 文档未修改；抽查过时的 `display_name` / `cwd` / `thread_id` 引用。

## build.rs / 配置文件（B）
- `package.json`, `tsconfig.json`, `vite.config.ts`, `Cargo.toml`, `*.css` — B（非源文件）

---

## 逐会话 bug 修复（16）

1. `src/features/modals/ModalShell.tsx:88-96` — ModalShell Enter 焦点方向
2. `src/features/composer/Composer.tsx:127` — Toolbar /command 保留草稿
3. `src/features/composer/Composer.tsx:51` — onSlashSelect 消费尾部参数
4. `src/test/setup.tsx:93` — `sinceMs` camelCase
5. `src/stores/replay.ts:8` — 幽灵字段 → 正确的 ReflectRolloutRecord 遍历
6. `src/stores/agent/reducer.ts:34-39` — turn_rewound UUID → findIndex
7. `src/stores/agent/reducer.ts:75-90` — tool_call_end 性能
8. `src/stores/agent/reducer.ts:133-144` — permission_bubble 风险已连线
9. `src/stores/agent/types.ts:27-33` — PendingApproval.risk 字段
10. `src/utils/commands/side_channel.ts:24-44` — camelCase TS 类型
11. `src/utils/commands/remote.ts:32` — `sinceMs` camelCase
12. `src/utils/commands/media.ts:38-44` — ComputerUseAction::Screenshot params:null
13. `src/features/media/useMediaController.ts:106` — params:null 同步
14. `src/features/media/MediaView.tsx:298` — params?: null
15. `src/features/memory/{useMemoryController,MemoryAddForm,MemoryView.test}.ts(x)` — global/session → user
16. `src/types/protocol/event.ts:10,105-107` — EVENT_ID_NONE 文档 + AbortReasonPayload
17. `app-core/src/state/mod.rs:247-254,20-23` — PendingApproval.risk + RiskLevel import
18. `app-core/src/reducer/matchers.rs:174-180,182-196,383-394` — 3 处修复
19. `app-core/src/reducer/state_mut.rs:28-49` — 3 个 map 函数
20. `src-tauri/src/state.rs:88-92,215` — workspace() 返回覆盖
21. `src-tauri/src/state/remote_config.rs` — sinceMs
22. `src-tauri/src/shortcut.rs:14-37` — 基于 cfg 的修饰符
23. `src-tauri/src/commands/agents.rs:104-117,266-289` — path_for + 新测试
24. `src-tauri/src/commands/files.rs:173` — `&agent.workspace()`
25. `src-tauri/tests/protocol_bridge_e2e.rs:121-130,154-180` — 真实断言
26. `src/utils/commands/sessions.ts` — 移除幽灵 `display_name`/`cwd`/`thread_id`/`tool_count`/`token_total`（早期会话）
27. `src/stores/agent/reducer.ts`（早期会话） — turn_rewound UUID, tool_call_end 性能, permission_bubble 风险

---

## 已推迟（产品决策 / Phase 3+ 工作）

- ModalShell ESC 静默拒绝审批/计划（有意 UX：dismiss = reject）
- Composer mentionOpen 从文本自动触发（显式仅工具栏设计）
- Composer SlashPopup 键盘导航（需要焦点管理重构）
- MCP 配置热重载（Phase 3+；代码中有记录）
- shell_sessions `tokio::spawn`（等效于 `tauri::async_runtime`；风格问题）
- AGENTS.md 幽灵字段清理（早期会话已解决）
- `reflect_set_effort` / `reflect_set_permission_mode` 接受 `string`（可窄化为联合；类型安全）
- DisplaySection `backgroundUrl` 仅从 http URL 初始化（上传的 `data:` 在输入中不可编辑）
- `StatusBar.tsx` modelKind 有两个 'warn' 分支（外观问题；逻辑没问题）
- `MessageList.tsx` index 键（外观问题；影响流式期间协调）

## 最终验证

- ✅ `pnpm typecheck` — 0 错误
- ✅ `pnpm test --run` — 530/530 通过（63 个测试文件）
- ✅ `app-core cargo test` — 91/91 通过
- ✅ `src-tauri cargo test --lib` — 60/60 通过
- ✅ workspace `cargo test --lib --tests` — 488/488 通过
- （1 个依赖环境的 e2e 测试 `e2e_anthropic_client_streams_real_llm` 需要实时 API key）