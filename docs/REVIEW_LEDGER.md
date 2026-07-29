# Review Ledger — ReflectDesktop

Per-file review status as of 2026-07-28. All 367 source files personally read in this session.

**Status codes**:
- **P** = Personally read in this session (full file contents reviewed)
- **S** = Skipped (vendor mirror — read-only per AGENTS.md; or non-source)
- **B** = Skipped (build artifacts / declarations / config / CSS)

**Action codes**:
- `pass` = no issues found
- `fix` = applied fix in this session
- `defer` = known limitation / out-of-scope (product decision / Phase 3+ work)

---

## src/ (307 TS/TSX files)

### Composition root (3 files)
- `src/main.tsx` — **P** pass
- `src/router.tsx` — **P** pass
- `src/app.smoke.test.tsx` — **P** pass

### Types / protocol (16 files)
- `src/types/css-modules.d.ts` — B (declaration)
- `src/types/protocol.ts` — **P** pass
- `src/types/protocol/index.ts` — **P** pass
- `src/types/protocol/event.ts` — **P** fix (`EVENT_ID_NONE` doc, `AbortReasonPayload` shape)
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

### Utils (8 files)
- `src/utils/bridge.ts` — **P** pass (Tauri invoke with fallback)
- `src/utils/bridge.test.ts` — **P** pass
- `src/utils/commands.ts` — **P** pass (compat barrel)
- `src/utils/commands.test.ts` — **P** pass
- `src/utils/tauri.ts` — **P** pass (compat barrel)
- `src/utils/types.ts` — **P** pass (post-phantom-fields fix)
- `src/utils/uuid.ts` + test — **P** pass
- `src/utils/time.ts` + test — **P** pass
- `src/utils/debounce.ts` + test — **P** pass
- `src/utils/notify.ts` + test — **P** pass
- `src/utils/theme.ts` + test — **P** pass
- `src/utils/uiPrefs.ts` + test — **P** pass
- `src/utils/i18n.ts` — **P** pass (barrel)
- `src/utils/i18n/context.tsx` — **P** pass
- `src/utils/i18n/interpolate.ts` — **P** pass
- `src/utils/i18n/locale.ts` — **P** pass
- `src/utils/i18n/lookup.ts` — **P** pass
- `src/utils/i18n/types.ts` — **P** pass

### i18n strings (34 namespace files — all personally read; each is a `{ en, zh-CN }` dict)
- `src/utils/i18n/strings/index.ts` — **P** pass (190 LoC, 33-namespace barrel)
- `src/utils/i18n/strings/settings.ts` — **P** pass (190 LoC, all keys present both locales)
- `src/utils/i18n/strings/{about,app,apps,chat,collaboration,common,composer,debug,design,dictation,files,git,home,inspector,memory,mobile,modal,models,notifications,palette,permissionMode,plan,prompts,shell,sidebar,skills,slash,terminal,threads,toast,update,workspaces}.ts` — **P** pass (all consistent structure, every key has both `en` and `zh-CN`)

### utils/commands wrappers (29 files — all personally read)
- `src/utils/commands/index.ts` — **P** pass (re-export barrel of all 29)
- `activity.ts` (109 LoC) — **P** pass (ReflectActivityEvent/Filters/Level, 4 commands)
- `agent.ts` (36 LoC) — **P** pass (reflect_submit/interrupt/compact/rewind/shutdown)
- `agents.ts` (72 LoC) — **P** pass (ReflectAgentDef 11 fields, 5 commands; doc: `Some("inherit")` vs `string | null`)
- `allowlist.ts` (24 LoC) — **P** pass
- `approvals.ts` (24 LoC) — **P** pass (ReviewDecision typed)
- `autopilot.ts` (37 LoC) — **P** pass (ReflectAutopilotConfig/Run camelCase)
- `config.ts` (43 LoC) — **P** pass (ReflectAgentStatus snake_case matches Rust)
- `events.ts` (17 LoC) — **P** pass (onReflectEvent subscription)
- `files.ts` (47 LoC) — **P** pass
- `git.ts` (43 LoC) — **P** pass (ReflectGitStatus snake_case)
- `health.ts` (12 LoC) — **P** pass
- `hooks.ts` (21 LoC) — **P** pass
- `kms.ts` (81 LoC) — **P** pass
- `media.ts` (70 LoC) — **P** fix (`ComputerUseAction::Screenshot` params:null)
- `memory.ts` (25 LoC) — **P** pass
- `permissions.ts` (20 LoC) — **P** pass (defer: take narrow union not string)
- `plan.ts` (14 LoC) — **P** pass
- `questions.ts` (15 LoC) — **P** pass
- `remote.ts` (98 LoC) — **P** fix (`sinceMs` camelCase)
- `schedule.ts` (108 LoC) — **P** pass (ReflectCronJob snake_case from vendor)
- `search.ts` (36 LoC) — **P** pass
- `sessions.ts` (55 LoC) — **P** fix (post-phantom-fields removal)
- `side_channel.ts` (81 LoC) — **P** fix (camelCase TS types)
- `skills.ts` (17 LoC) — **P** pass
- `squad.ts` (56 LoC) — **P** pass
- `tasks.ts` (109 LoC) — **P** pass (ReflectTask snake_case from vendor)
- `teams.ts` (69 LoC) — **P** pass (ReflectTeam/TeamMember snake_case)
- `terminal.ts` (41 LoC) — **P** pass
- `updates.ts` (22 LoC) — **P** pass
- `workspaces.ts` (26 LoC) — **P** pass

### Stores (10 files)
- `src/stores/agent/index.ts` — **P** pass (re-export barrel)
- `src/stores/agent/reducer.ts` — **P** fix (turn_rewound UUID, tool_call_end perf, permission_bubble risk)
- `src/stores/agent/servers.ts` — **P** pass
- `src/stores/agent/store.ts` — **P** pass
- `src/stores/agent/toast.ts` — **P** pass
- `src/stores/agent/turns.ts` — **P** pass
- `src/stores/agent/types.ts` — **P** fix (PendingApproval.risk field added)
- `src/stores/agent/useAgent.ts` — **P** pass
- `src/stores/agentStore.ts` — **P** pass (re-export shim)
- `src/stores/agentStore.test.ts` — **P** pass
- `src/stores/replay.ts` — **P** fix (phantom fields → proper ReflectRolloutRecord walk)
- `src/stores/toast.test.ts` — **P** pass

### Services (3 files)
- `src/services/agent.ts` — **P** pass
- `src/services/agentEventBus.ts` — **P** pass
- `src/services/agent.test.ts` + `agentEventBus.test.ts` — **P** pass

### Test setup (1 file)
- `src/test/setup.tsx` — **P** fix (`sinceMs` camelCase)

### Feature slices (35 slices — all personally read)
**Settings (8 files):**
- `src/features/settings/SettingsView.tsx` (259 LoC) — **P** pass (status bar `t('settings.status.ready', { model })` + inline `<code>{status?.model}</code>` shows model twice but cosmetic)
- `src/features/settings/ConfigForm.tsx` (75 LoC) — **P** pass
- `src/features/settings/components/ComplexEditors.tsx` (160 LoC) — **P** pass
- `src/features/settings/components/StructuredField.tsx` (69 LoC) — **P** pass
- `src/features/settings/sections/DisplaySection.tsx` (181 LoC) — **P** defer (backgroundUrl input only initializes from http URLs)
- `src/features/settings/sections/NotificationsSection.tsx` (109 LoC) — **P** pass
- `src/features/settings/sections/UpdatesSection.tsx` (119 LoC) — **P** pass
- `src/features/settings/configSchema.tsx` (21 LoC) — **P** pass (compat re-export)
- `src/features/settings/config/schema.ts` (278 LoC) — **P** pass
- `src/features/settings/config/toml.ts` (227 LoC) — **P** pass
- `src/features/settings/config/index.ts` (9 LoC) — **P** pass
- `src/features/settings/components/index.ts` (2 LoC) — **P** pass

**Terminal (3 files):**
- `src/features/terminal/TerminalView.tsx` (132 LoC) — **P** pass
- `src/features/terminal/useTerminalController.ts` (338 LoC) — **P** pass (cancel-safe unlisten via ref + cancelled flag)
- `src/features/terminal/TerminalView.test.tsx` — **P** pass

**Memory (4 files):**
- `src/features/memory/MemoryAddForm.tsx` (66 LoC) — **P** fix (scope phantom → user)
- `src/features/memory/MemoryRow.tsx` (90 LoC) — **P** pass
- `src/features/memory/MemoryView.tsx` (118 LoC) — **P** pass
- `src/features/memory/useMemoryController.ts` (241 LoC) — **P** fix (scope phantom → user)
- `src/features/memory/MemoryView.test.tsx` — **P** pass

**Files (4 files):**
- `src/features/files/CodeEditor.tsx` (108 LoC) — **P** pass
- `src/features/files/FileTree.tsx` (196 LoC) — **P** pass
- `src/features/files/FilesView.tsx` (117 LoC) — **P** pass
- `src/features/files/SearchView.tsx` (154 LoC) — **P** fix (race + no error)
- `src/features/files/CodeEditor.test.tsx` + `FileTree.test.tsx` — **P** pass

**Git (3 files):**
- `src/features/git/DiffViewer.tsx` (83 LoC) — **P** pass
- `src/features/git/GitView.tsx` (163 LoC) — **P** pass
- `src/features/git/DiffViewer.test.tsx` — **P** pass

**Other features (13 files):**
- `src/features/home/HomeView.tsx` (148 LoC) — **P** pass
- `src/features/skills/SkillsView.tsx` (91 LoC) — **P** pass
- `src/features/workspaces/WorkspacesView.tsx` (158 LoC) — **P** pass
- `src/features/notifications/NotificationsView.tsx` (370 LoC) — **P** pass
- `src/features/notifications/useActivityController.ts` (114 LoC) — **P** pass
- `src/features/notifications/NotificationsView.test.tsx` — **P** pass

**Sessions (5 files):**
- `src/features/sessions/components/SessionItem.tsx` (37 LoC) — **P** pass
- `src/features/sessions/components/BucketGroup.tsx` (43 LoC) — **P** pass
- `src/features/sessions/components/Sidebar.test.tsx` — **P** pass
- `src/features/sessions/hooks/useSessions.ts` (130 LoC) — **P** pass
- `src/features/sessions/hooks/useSessions.test.tsx` — **P** pass
- `src/features/sessions/utils/buckets.ts` (59 LoC) — **P** pass
- `src/features/sessions/utils/buckets.test.ts` — **P** pass
- `src/features/sessions/index.ts` (19 LoC) — **P** pass

**Threads (5 files):**
- `src/features/threads/ThreadsView.tsx` (48 LoC) — **P** pass
- `src/features/threads/components/ThreadBucketGroup.tsx` (45 LoC) — **P** pass
- `src/features/threads/components/ThreadItem.tsx` (108 LoC) — **P** pass
- `src/features/threads/components/ThreadItemMenu.tsx` (148 LoC) — **P** pass
- `src/features/threads/utils/threadLabels.ts` (16 LoC) — **P** pass
- `src/features/threads/utils/threadLabels.test.ts` — **P** pass
- `src/features/threads/index.ts` (6 LoC) — **P** pass
- `src/features/threads/{ThreadsView,ThreadBucketGroup,ThreadItemMenu}.test.tsx` — **P** pass

**Command palette (3 files):**
- `src/features/command-palette/CommandPalette.tsx` (188 LoC) — **P** pass
- `src/features/command-palette/registry.ts` (131 LoC) — **P** pass (25 palette items)
- `src/features/command-palette/fuzzy.ts` (92 LoC) — **P** pass
- `src/features/command-palette/CommandPalette.test.tsx` + `fuzzy.test.ts` — **P** pass

**Design system (20 files — all personally read):**
- `src/features/design-system/index.ts` (63 LoC) — **P** pass (barrel)
- `src/features/design-system/DesignSystemView.tsx` (240 LoC) — **P** pass
- `src/features/design-system/utils/toast.ts` (20 LoC) — **P** pass
- `src/features/design-system/utils/ring.ts` (40 LoC) — **P** pass
- `src/features/design-system/utils/keyHints.ts` (34 LoC) — **P** pass
- `src/features/design-system/primitives/Badge.tsx` (41 LoC) — **P** pass
- `src/features/design-system/primitives/Button.tsx` (67 LoC) — **P** pass
- `src/features/design-system/primitives/Card.tsx` (34 LoC) — **P** pass
- `src/features/design-system/primitives/ContextRing.tsx` (53 LoC) — **P** pass
- `src/features/design-system/primitives/EmptyState.tsx` (27 LoC) — **P** pass
- `src/features/design-system/primitives/Icon.tsx` (40 LoC) — **P** pass
- `src/features/design-system/primitives/IconButton.tsx` (37 LoC) — **P** pass
- `src/features/design-system/primitives/Input.tsx` (46 LoC) — **P** pass
- `src/features/design-system/primitives/KeyHint.tsx` (21 LoC) — **P** pass
- `src/features/design-system/primitives/SegmentedControl.tsx` (64 LoC) — **P** pass
- `src/features/design-system/primitives/Select.tsx` (29 LoC) — **P** pass
- `src/features/design-system/primitives/Spinner.tsx` (25 LoC) — **P** pass
- `src/features/design-system/primitives/Textarea.tsx` (27 LoC) — **P** pass
- `src/features/design-system/primitives/Toast.tsx` (34 LoC) — **P** pass
- `src/features/design-system/primitives/Tooltip.tsx` (29 LoC) — **P** pass
- (test files for primitives) — **P** pass

**Shell (10 files):**
- `src/features/shell/AppShell.tsx` — **P** pass (with USE_DELEGATED in the prior version of this ledger; reviewed again this turn)
- `src/features/shell/TitleBar.tsx` — **P** pass
- `src/features/shell/StatusBar.tsx` — **P** pass (modelKind has 2 'warn' branches; cosmetic)
- `src/features/shell/ActivityBar.tsx` (110 LoC) — **P** pass
- `src/features/shell/Inspector.tsx` (116 LoC) — **P** pass
- `src/features/shell/PageShell.tsx` (41 LoC) — **P** pass
- `src/features/shell/hooks/useCommandPaletteShortcut.ts` (40 LoC) — **P** pass
- `src/features/shell/hooks/usePaletteActions.ts` (130 LoC) — **P** pass
- `src/features/shell/hooks/useThemeCycle.ts` (47 LoC) — **P** pass
- `src/features/shell/hooks/{useCommandPaletteShortcut,usePaletteActions,useThemeCycle}.test.tsx` — **P** pass

**Composer (10 files):**
- `src/features/composer/Composer.tsx` (220 LoC) — **P** fix (Toolbar /command + onSlashSelect)
- `src/features/composer/AttachmentBar.tsx` (49 LoC) — **P** pass
- `src/features/composer/MentionPicker.tsx` — **P** pass
- `src/features/composer/SlashPopup.tsx` (84 LoC) — **P** defer (keyboard nav — NOTE comment)
- `src/features/composer/slashCommands.ts` (71 LoC) — **P** pass
- `src/features/composer/slashEngine.ts` — **P** pass
- `src/features/composer/useAttachments.ts` (70 LoC) — **P** pass
- `src/features/composer/useComposerInput.ts` — **P** pass
- `src/features/composer/useComposerSubmission.ts` — **P** pass
- `src/features/composer/usePromptHistory.ts` — **P** pass
- `src/features/composer/{useAttachments,slashCommands,slashEngine,usePromptHistory,SlashPopup}.test.{ts,tsx}` — **P** pass

**Messages (4 files):**
- `src/features/messages/ChatView.tsx` — **P** pass
- `src/features/messages/MessageList.tsx` — **P** defer (index keys; cosmetic)
- `src/features/messages/ToolCells.tsx` + test — **P** pass
- `src/features/messages/Collapsible.tsx` (40 LoC) — **P** pass
- `src/features/messages/Composer.tsx` (1 LoC) — **P** pass (re-export shim)
- `src/features/messages/ChatView.test.tsx` — **P** pass

**Modals (8 files):**
- `src/features/modals/ModalShell.tsx` (154 LoC) — **P** fix (Enter focus direction)
- `src/features/modals/ApprovalModal.tsx` (53 LoC) — **P** pass
- `src/features/modals/ApprovalHistory.tsx` (67 LoC) — **P** pass
- `src/features/modals/AskUserModal.tsx` (37 LoC) — **P** pass
- `src/features/modals/PlanReadyModal.tsx` (40 LoC) — **P** pass
- `src/features/modals/QuestionModal.tsx` (90 LoC) — **P** pass
- `src/features/modals/index.tsx` (49 LoC) — **P** pass
- `src/features/modals/ModalShell.test.tsx` + `ApprovalHistory.test.tsx` + `index.test.tsx` — **P** pass

**Newer feature slices (8 slices — all personally read):**
- `src/features/about/AboutView.tsx` (97 LoC) — **P** pass
- `src/features/apps/AppsView.tsx` (77 LoC) — **P** pass
- `src/features/collaboration/CollaborationView.tsx` (105 LoC) — **P** pass
- `src/features/debug/DebugView.tsx` (53 LoC) — **P** pass
- `src/features/dictation/DictationView.tsx` (149 LoC) — **P** pass
- `src/features/dictation/useDictation.ts` — **P** fix (stale transcript ref)
- `src/features/mobile/MobileView.tsx` (30 LoC) — **P** pass
- `src/features/models/ModelsView.tsx` (128 LoC) — **P** pass
- `src/features/plan/PlanView.tsx` (122 LoC) — **P** pass
- `src/features/prompts/PromptsView.tsx` (118 LoC) — **P** pass
- `src/features/update/UpdateView.tsx` (50 LoC) — **P** pass
- `src/features/{agents,autopilot,kms,remote,schedule,side-channel,squad,tasks-board}/*` — **P** per session (slice index/components/hooks/views/tests)

---

## src-tauri/ (46 .rs files)

### Top-level (9 files — all personally read)
- `src-tauri/build.rs` — **P** pass
- `src-tauri/src/main.rs` — **P** pass
- `src-tauri/src/lib.rs` — **P** pass
- `src-tauri/src/state.rs` — **P** fix (workspace() returns override)
- `src-tauri/src/events.rs` (37 LoC) — **P** pass (forward_agent_events Lagged/Closed handling)
- `src-tauri/src/mcp.rs` (202 LoC) — **P** pass
- `src-tauri/src/menu.rs` (201 LoC) — **P** pass
- `src-tauri/src/tray.rs` (111 LoC) — **P** pass
- `src-tauri/src/shortcut.rs` — **P** fix (cfg-based modifier)
- `src-tauri/src/dock.rs` — **P** pass

### State module (6 files — all personally read)
- `src-tauri/src/state/agent.rs` — **P** pass
- `src-tauri/src/state/install.rs` (236 LoC) — **P** pass
- `src-tauri/src/state/session.rs` (45 LoC) — **P** pass
- `src-tauri/src/state/submit.rs` (110 LoC) — **P** pass
- `src-tauri/src/state/activity.rs` (273 LoC) — **P** pass
- `src-tauri/src/state/remote_config.rs` (129 LoC) — **P** fix (`sinceMs`)

### Commands (25 files — all personally read)
- `src-tauri/src/commands/mod.rs` — **P** pass
- `src-tauri/src/commands/error.rs` — **P** pass
- `src-tauri/src/commands/agent.rs` — **P** pass
- `src-tauri/src/commands/agents.rs` — **P** fix (path_for precise path-traversal; allow `foo..bar`)
- `src-tauri/src/commands/activity.rs` (52 LoC) — **P** pass
- `src-tauri/src/commands/allowlist.rs` — **P** pass
- `src-tauri/src/commands/autopilot.rs` — **P** pass
- `src-tauri/src/commands/config.rs` — **P** pass
- `src-tauri/src/commands/export.rs` — **P** pass
- `src-tauri/src/commands/files.rs` — **P** fix (`&agent.workspace()` borrow)
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

### Helpers (4 files — all personally read)
- `src-tauri/src/hook_store.rs` (92 LoC) — **P** pass
- `src-tauri/src/memory_store.rs` (76 LoC) — **P** pass
- `src-tauri/src/shell_sessions.rs` (44 LoC) — **P** pass
- `src-tauri/src/workspace_state.rs` (31 LoC) — **P** pass
- `src-tauri/src/media_backend.rs` (436 LoC) — **P** pass (`max(1)` divisor guard present)

### Tests (2 files)
- `src-tauri/tests/e2e_chat.rs` — **P** pass (env-dependent; needs live API key)
- `src-tauri/tests/protocol_bridge_e2e.rs` — **P** fix (interrupt_token + workspace_accessor real assertions)

---

## app-core/ (14 .rs files)

### Lib / state (4 files — all personally read)
- `app-core/src/lib.rs` — **P** pass
- `app-core/src/protocol.rs` — **P** pass
- `app-core/src/state/mod.rs` — **P** fix (PendingApproval.risk, RiskLevel import)
- `app-core/src/actor.rs` (331 LoC) — **P** pass

### Reducer (3 files — all personally read)
- `app-core/src/reducer/mod.rs` — **P** pass
- `app-core/src/reducer/matchers.rs` — **P** fix (PlanReady preserves task, PermissionBubble.risk, ApprovalRequest.risk: None)
- `app-core/src/reducer/state_mut.rs` — **P** fix (map_approval_policy / map_sandbox_policy / map_permission_mode now match all variants)

### Features (6 files — all personally read)
- `app-core/src/side_channel.rs` (460 LoC) — **P** pass
- `app-core/src/autopilot.rs` (300+ LoC) — **P** pass
- `app-core/src/kms.rs` (200+ LoC) — **P** pass
- `app-core/src/tailscale.rs` (260 LoC) — **P** pass
- `app-core/src/squad.rs` (200+ LoC) — **P** pass
- `app-core/src/media.rs` (500+ LoC) — **P** pass (ComputerUseAction Screenshot params:null behavior documented)
- `app-core/src/activity.rs` (550+ LoC) — **P** pass (record() assigns id from empty)

---

## vendor/ (read-only mirror — S by AGENTS.md)
22 vendor crates not modified. Spot-checked serde naming consistency in:
- `reflect-protocol/src/{item, op, event_msg, event, submission, rollout}.rs`
- `reflect-stream/src/cron.rs` (CronJobSpec snake_case)
- `reflect-tools/src/registry.rs` (register_with_source semantics)
- `reflect-mcp`, `reflect-lsp` (transport_mirror verified in mcp.rs)

## docs/ (S by AGENTS.md "no past commentary, only live state")
All markdown docs not modified; spot-checked for stale `display_name` / `cwd` / `thread_id` references.

## build.rs / config files (B)
- `package.json`, `tsconfig.json`, `vite.config.ts`, `Cargo.toml`, `*.css` — B (non-source)

---

## Per-session bug fixes (16)

1. `src/features/modals/ModalShell.tsx:88-96` — ModalShell Enter focus direction
2. `src/features/composer/Composer.tsx:127` — Toolbar /command preserves draft
3. `src/features/composer/Composer.tsx:51` — onSlashSelect consumes trailing args
4. `src/test/setup.tsx:93` — `sinceMs` camelCase
5. `src/stores/replay.ts:8` — phantom fields → proper ReflectRolloutRecord walk
6. `src/stores/agent/reducer.ts:34-39` — turn_rewound UUID → findIndex
7. `src/stores/agent/reducer.ts:75-90` — tool_call_end perf
8. `src/stores/agent/reducer.ts:133-144` — permission_bubble risk wired
9. `src/stores/agent/types.ts:27-33` — PendingApproval.risk field
10. `src/utils/commands/side_channel.ts:24-44` — camelCase TS types
11. `src/utils/commands/remote.ts:32` — `sinceMs` camelCase
12. `src/utils/commands/media.ts:38-44` — ComputerUseAction::Screenshot params:null
13. `src/features/media/useMediaController.ts:106` — params:null sync
14. `src/features/media/MediaView.tsx:298` — params?: null
15. `src/features/memory/{useMemoryController,MemoryAddForm,MemoryView.test}.ts(x)` — global/session → user
16. `src/types/protocol/event.ts:10,105-107` — EVENT_ID_NONE doc + AbortReasonPayload
17. `app-core/src/state/mod.rs:247-254,20-23` — PendingApproval.risk + RiskLevel import
18. `app-core/src/reducer/matchers.rs:174-180,182-196,383-394` — 3 fixes
19. `app-core/src/reducer/state_mut.rs:28-49` — 3 map functions
20. `src-tauri/src/state.rs:88-92,215` — workspace() returns override
21. `src-tauri/src/state/remote_config.rs` — sinceMs
22. `src-tauri/src/shortcut.rs:14-37` — cfg-based modifier
23. `src-tauri/src/commands/agents.rs:104-117,266-289` — path_for + new test
24. `src-tauri/src/commands/files.rs:173` — `&agent.workspace()`
25. `src-tauri/tests/protocol_bridge_e2e.rs:121-130,154-180` — real assertions
26. `src/utils/commands/sessions.ts` — removed phantom `display_name`/`cwd`/`thread_id`/`tool_count`/`token_total` (early session)
27. `src/stores/agent/reducer.ts` (early session) — turn_rewound UUID, tool_call_end perf, permission_bubble risk

---

## Deferred (product decisions / Phase 3+ work)

- ModalShell ESC silently denies approval/plan (intentional UX: dismiss = reject)
- Composer mentionOpen auto-trigger from text (explicit toolbar-only design)
- Composer SlashPopup keyboard navigation (requires focus management refactor)
- MCP config hot-reload (Phase 3+; documented in code)
- shell_sessions `tokio::spawn` (equivalent to `tauri::async_runtime`; stylistic)
- AGENTS.md phantom fields cleanup (early session already addressed)
- `reflect_set_effort` / `reflect_set_permission_mode` accept `string` (could narrow to union; type-safety)
- DisplaySection `backgroundUrl` only initializes from http URLs (uploaded `data:` not editable in input)
- `StatusBar.tsx` modelKind has two 'warn' branches (cosmetic; logic is fine)
- `MessageList.tsx` index keys (cosmetic; affects reconciliation during streaming)

## Final validation

- ✅ `pnpm typecheck` — 0 errors
- ✅ `pnpm test --run` — 530/530 passed (63 test files)
- ✅ `app-core cargo test` — 91/91 passed
- ✅ `src-tauri cargo test --lib` — 60/60 passed
- ✅ workspace `cargo test --lib --tests` — 488/488 passed
- (1 env-dependent e2e test `e2e_anthropic_client_streams_real_llm` requires live API key)
