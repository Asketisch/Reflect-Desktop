# Codebase Map (Task-Oriented)

Canonical navigation for ReflectDesktop. Use this as: **"if you need X, edit Y"**.

Related docs:

- Setup / build / release: [`README.md`](../README.md)
- Architecture: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- GUI design history: [`gui/00-index.md`](gui/00-index.md)
- IPC envelope: [`PROTOCOL_BRIDGE.md`](PROTOCOL_BRIDGE.md)
- Change log: [`CHANGELOG.md`](CHANGELOG.md)
- Agent contract: [`AGENTS.md`](../AGENTS.md)

---

## How Changes Flow

For backend behavior, follow this path in order:

1. Frontend callsite: `src/features/**` (hooks/controllers/components)
2. Frontend IPC API: `src/utils/commands/{domain}.ts` (compat re-exports live in `src/utils/tauri.ts` and `src/utils/commands.ts`)
3. Tauri command registration: `src-tauri/src/lib.rs` (`invoke_handler` enumerates each `reflect_*` command)
4. Tauri command body: `src-tauri/src/commands/mod.rs` re-exports from `src-tauri/src/commands/<domain>.rs`
5. App state (AgentThread host): `src-tauri/src/state.rs` (plus private helpers `hook_store.rs` / `memory_store.rs` / `shell_sessions.rs` / `workspace_state.rs`)
6. Event forwarder: `src-tauri/src/events.rs`
7. Reflect protocol types: `vendor/reflect-protocol/src/{event,event_msg,op,item}.rs`

If behavior must run in headless (CLI/TUI) too, the change lives in `vendor/reflect-*` first.

---

## If You Need X, Edit Y

| Need | Primary files |
| --- | --- |
| Add a new feature slice / route | `src/router.tsx`, `src/features/<slice>/<View>.tsx`, `src/features/shell/AppShell.tsx` |
| Add/change Tauri command from frontend | `src/utils/commands/<domain>.ts`, `src-tauri/src/commands/<domain>.rs`, `src-tauri/src/commands/mod.rs` (re-export), `src-tauri/src/lib.rs` (handler list) |
| Add/change event handler in UI | `src/services/agent.ts` (compat re-export) + `src/services/agentEventBus.ts` (refcounted fan-out), `src/stores/agent/reducer.ts`, `src/features/<slice>/use*Controller.ts` |
| Change composer (input box) | `src/features/composer/{Composer,SlashPopup}.tsx`, `src/features/composer/{slashCommands,slashEngine}.ts`, `src/features/composer/{useComposerInput,useComposerSubmission,useAttachments,usePromptHistory}.ts`. The legacy path `src/features/messages/Composer.tsx` is a thin re-export shim — do not edit. |
| Change chat rendering | `src/features/messages/{ChatView,MessageList,ToolCells,Collapsible}.tsx` |
| Change IDE shell (layout / activity bar / status bar) | `src/features/shell/{AppShell,ActivityBar,TitleBar,StatusBar,Inspector,PageShell}.tsx`. Stateful shell interactions live in `src/features/shell/hooks/{useCommandPaletteShortcut,usePaletteActions,useThemeCycle}.ts`. |
| Change approval / question / plan modal | `src/features/modals/{ModalShell,index}.tsx`, individual bodies at `src/features/modals/{ApprovalModal,QuestionModal,AskUserModal,PlanReadyModal,ApprovalHistory}.tsx` |
| Change session list (sidebar) | `src/features/sessions/components/Sidebar.tsx`, `src/features/sessions/hooks/useSessions.ts` |
| Change settings persistence | `src/features/settings/SettingsView.tsx`, `src/features/settings/ConfigForm.tsx`, `src/features/settings/config/{schema,toml}.ts`, `src/features/settings/components/{StructuredField,ComplexEditors}.tsx`, `src/features/settings/sections/{DisplaySection,NotificationsSection,UpdatesSection}.tsx`, `src/utils/commands/config.ts`, `src-tauri/src/commands/config.rs` |
| Change theme / design tokens | `src/styles/tokens.css`, `src/features/design-system/DesignSystemView.tsx` |
| Add tray icon / menu / shortcut | `src-tauri/src/{tray,menu,shortcut}.rs`, `src-tauri/src/lib.rs` |
| macOS dock badge | `src-tauri/src/dock.rs` |
| Memory view controller | `src/features/memory/MemoryView.tsx` + `src/features/memory/useMemoryController.ts` |
| Terminal view controller | `src/features/terminal/TerminalView.tsx` + `src/features/terminal/useTerminalController.ts` |
| Add a new vendor crate | `scripts/vendor-sync.sh` (CRATES list), `Cargo.toml` workspace |
| Add a new test fixture | `src/test/setup.ts`, `vitest.config.ts` |
| Change protocol types | `vendor/reflect-protocol/src/*.rs` (sync via `scripts/vendor-sync.sh`) |
| Change agent store reducer / actions | `src/stores/agent/reducer.ts` (canonical). `src/stores/agentStore.ts` is a compatibility re-export — do not add new code there. |

---

## Frontend Navigation

- Composition root: `src/main.tsx`
- Router (TanStack Router): `src/router.tsx`
- App layout shell (IDE 5-pane): `src/features/shell/AppShell.tsx` (+ `ActivityBar/TitleBar/StatusBar/Inspector/PageShell`)
- Tauri IPC low-level bridge: `src/utils/bridge.ts` (`invoke` / `listen` + fallback)
- Tauri IPC wrapper barrels (compat): `src/utils/tauri.ts`, `src/utils/commands.ts`
- Tauri IPC wrappers (canonical, per-domain): `src/utils/commands/{health,agent,approvals,plan,permissions,questions,config,sessions,events,workspaces,skills,memory,hooks,git,terminal,files,allowlist,updates,search}.ts`
- Agent hook (event fanout + submit): `src/services/agent.ts` (legacy compat re-export) + `src/services/agentEventBus.ts` (refcounted bus)
- Global Zustand store: `src/stores/agentStore.ts` (compat facade) → `src/stores/agent/` (canonical impl: `store.ts` + `reducer.ts` + `turns.ts` + `toast.ts` + `servers.ts` + `types.ts` + `useAgent.ts` + `index.ts`)
- Theme infra: `src/utils/theme.ts`
- Design tokens / base reset: `src/styles/{tokens,base}.css` + `typography.module.css`
- i18n runtime: `src/utils/i18n.ts` (compat barrel) → `src/utils/i18n/{context.tsx,locale.ts,interpolate.ts,lookup.ts,types.ts}` + `src/utils/i18n/strings/index.ts` merging domain catalogs under `src/utils/i18n/strings/{about,app,apps,chat,collaboration,common,composer,debug,design,dictation,files,git,home,inspector,memory,mobile,modal,models,notifications,palette,permissionMode,plan,prompts,settings,shell,sidebar,skills,slash,terminal,threads,toast,update,workspaces}.ts`
- Shared types: `src/types/protocol.ts`

### Feature slices

| Slice | Files | Notes |
| --- | --- | --- |
| `home` | `features/home/HomeView.tsx` | Dashboard / quick actions |
| `messages` | `features/messages/{ChatView,MessageList,ToolCells,Collapsible,Composer,Composer.module,MessageList.module,ChatView.module,ToolCells.module,Collapsible.module}.{tsx,css}` | Chat scrollback + rows; Composer is now a thin re-export shim → `features/composer/Composer` |
| `composer` | `features/composer/{Composer,SlashPopup,MentionPicker,AttachmentBar}.tsx` + `slashCommands.ts` + `slashEngine.ts` + `useComposerInput.ts` + `useComposerSubmission.ts` + `useAttachments.ts` + `usePromptHistory.ts` | Self-owned Composer + slash engine |
| `shell` | `features/shell/{AppShell,ActivityBar,TitleBar,StatusBar,Inspector,PageShell}.tsx` + `features/shell/hooks/{useCommandPaletteShortcut,usePaletteActions,useThemeCycle}.ts` | IDE 5-pane layout + stateful shell hooks |
| `modals` | `features/modals/{ModalShell,index}.tsx` + `{ApprovalModal,QuestionModal,AskUserModal,PlanReadyModal,ApprovalHistory}.tsx` | Approval / Question / Plan / AskUser + history |
| `sessions` | `features/sessions/{components/Sidebar,hooks/useSessions}.{tsx,ts}` + test | Time-bucketed sidebar |
| `settings` | `features/settings/{SettingsView,ConfigForm,configSchema}.tsx` + `features/settings/config/{schema,toml}.ts` + `features/settings/components/{StructuredField,ComplexEditors}.tsx` + `features/settings/sections/{DisplaySection,NotificationsSection,UpdatesSection}.tsx` | Display / Editor / Provider + structured config form |
| `models` | `features/models/ModelsView.tsx` | Model picker |
| `workspaces` | `features/workspaces/WorkspacesView.tsx` | Workspace picker (M3.x) |
| `git` | `features/git/GitView.tsx` | Git panel (M3.x) |
| `files` | `features/files/FilesView.tsx` | File tree (M3.x) |
| `plan` | `features/plan/PlanView.tsx` | Plan mode UI |
| `terminal` | `features/terminal/TerminalView.tsx` + `useTerminalController.ts` | Terminal dock + controller hook |
| `memory` | `features/memory/MemoryView.tsx` + `MemoryRow.tsx` + `MemoryAddForm.tsx` + `useMemoryController.ts` | Memory view + controller hook |
| `skills` | `features/skills/SkillsView.tsx` | Skills catalog |
| `apps` | `features/apps/AppsView.tsx` | MCP apps (M3.1+) |
| `prompts` | `features/prompts/PromptsView.tsx` | Custom prompts library |
| `threads` | `features/threads/ThreadsView.tsx` + test | Threads (M2.5 LRU) |
| `notifications` | `features/notifications/NotificationsView.tsx` | Toast center |
| `dictation` | `features/dictation/DictationView.tsx` | Hold-to-talk (M3.x) |
| `mobile` | `features/mobile/MobileView.tsx` | iOS layout |
| `update` | `features/update/UpdateView.tsx` | Auto-update UI |
| `debug` | `features/debug/DebugView.tsx` | Debug panel |
| `about` | `features/about/AboutView.tsx` | About / version |
| `design-system` | `features/design-system/{DesignSystemView,primitives/*}.tsx` | DS catalog + primitives (live) |

### Import Aliases

Use TS/Vite aliases:

- `@/*` → `src/*`

---

## Backend Navigation

- Command registry (what frontend can invoke): `src-tauri/src/lib.rs` (`tauri::generate_handler!` enumerates each `reflect_*` command)
- Command bodies (per domain): `src-tauri/src/commands/{agent,allowlist,config,export,files,git,hooks,memory,search,sessions,shell,skills,update,workspaces}.rs`; shared error helpers in `src-tauri/src/commands/error.rs` (`CommandError` / `CommandResult`)
- Command barrel: `src-tauri/src/commands/mod.rs` (thin re-export of each `<domain>.rs`)
- App state: `src-tauri/src/state.rs` (`MinimalAgent` stub → `reflect_core::AgentThread`)
- State support modules: `src-tauri/src/{hook_store,memory_store,shell_sessions,workspace_state}.rs`
- Event forwarder: `src-tauri/src/events.rs` (`forward_agent_events`)
- Tray / menu / shortcut / dock: `src-tauri/src/{tray,menu,shortcut,dock}.rs`
- MCP runtime: `src-tauri/src/mcp.rs`
- Tauri config: `src-tauri/tauri.conf.json`
- Capabilities: `src-tauri/capabilities/main.json`
- Cargo workspace: `Cargo.toml` (workspace root) + `src-tauri/Cargo.toml`

### Tauri IPC surface

Each `#[tauri::command]` lives in the domain module under `src-tauri/src/commands/<domain>.rs` and is registered in `src-tauri/src/lib.rs`. The full set spans:

| Domain | Sample commands (full set in `commands/<domain>.rs`) |
| --- | --- |
| `agent` | `ping`, `reflect_agent_status`, `reflect_submit` (see also: approval / question / plan / effort / permission / compaction / shutdown variants) |
| `config` | `reflect_get_config`, `reflect_save_config` |
| `sessions` | `reflect_list_sessions`, `reflect_rename_session`, `reflect_delete_session`, `reflect_replay_session`, `reflect_export_session`, `reflect_export_session_markdown` |
| `workspaces` | `reflect_list_workspaces`, `reflect_set_workspace`, `reflect_current_workspace` |
| `skills` | `reflect_list_skills` |
| `memory` | `reflect_list_memory`, `reflect_add_memory`, `reflect_remove_memory` |
| `hooks` | `reflect_list_hooks`, `reflect_toggle_hook` |
| `git` | `reflect_git_status`, `reflect_git_diff`, `reflect_git_log` |
| `shell` | `reflect_run_shell`, `reflect_kill_shell`, `reflect_list_shell_sessions` |
| `files` | `reflect_list_dir`, `reflect_read_file` |
| `search` | `reflect_search_files` |
| `allowlist` | `reflect_load_allowlist`, `reflect_save_allowlist`, `reflect_check_allowlist` |
| `update` | `reflect_check_update` |
| `tools` | `reflect_list_tools` (see `commands/agent.rs`) |
| Dock (macOS) | `reflect_set_dock_badge` |

Push events (single channel, dispatched by `msg.type`):

- `reflect_event` — payload is `reflect_protocol::Event` (snake_case JSON)
- `reflect_terminal_output` — payload is `ReflectShellOutputChunk` (B8-01 streaming shell)

---

## Vendor Crates

`vendor/` contains a read-only mirror of `reflect-*` crates from the upstream `Reflect-Agent` repo, kept in sync via `scripts/vendor-sync.sh`. The exact set of crates synced is controlled by the `CRATES` list in `scripts/vendor-sync.sh`; the live directory listing under `vendor/` (e.g. `vendor/reflect-protocol/`, `vendor/reflect-core/`, `vendor/reflect-config/`, `vendor/reflect-tools/`, etc.) reflects the current snapshot.

UI-agnostic reducer/state types used by both ReflectDesktop and the headless ReflectAgent tooling live in the `app-core/` Cargo crate at the workspace root (sibling to `vendor/`), not under `vendor/`. Edit upstream first; then `bash scripts/vendor-sync.sh /path/to/Reflect-Agent main`.

---

## Events Map (Backend → Frontend)

- Backend emits via `src-tauri/src/events.rs::forward_agent_events` → `app.emit("reflect_event", &event)`.
- Frontend fanout hub: `src/services/agentEventBus.ts` (refcounted single subscription) → `src/stores/agent/reducer.ts`. `src/services/agent.ts` is the legacy compat re-export that forwards to the store hook.
- Parser guards: `src/utils/tauri.ts::onReflectEvent`.
- Type contract: `vendor/reflect-protocol/src/event_msg.rs` (Rust) ↔ `src/types/protocol.ts` (TS).

If event payload format changes, regenerate `src/types/protocol.ts` from `reflect-protocol`'s schema dump (`cargo run -p reflect-protocol --example dump_schema` → `npx json2ts`).

---

## Type Contract Files

Keep Rust and TypeScript contracts in sync:

- Rust backend types: `vendor/reflect-protocol/src/{event,event_msg,op,item}.rs`
- Frontend types: `src/types/protocol.ts`
- Settings: `src/features/settings/config/schema.ts` ↔ `src-tauri/src/commands/config.rs`

This is required for submissions, events, settings, and session payloads.

---

## Conventions

- **Feature-sliced design**: each feature is a folder under `src/features/<slice>/`. Components live flat or in `components/`; hooks in `hooks/`; controllers (stateful view orchestration) live alongside the view as `use*Controller.ts`; tests co-located as `*.test.ts(x)`.
- **Co-location**: tests live next to source (`Foo.tsx` → `Foo.test.tsx`).
- **Naming**: PascalCase for components, camelCase for hooks/utils, ALL_CAPS for env-like constants.
- **State**: prefer local `useState`/`useReducer`; lift to Zustand store only when shared by ≥2 features.

---

## Quick Runbook

```bash
pnpm install                 # install JS deps
pnpm tauri dev               # dev (HMR)
pnpm test                    # vitest
pnpm typecheck               # tsc --noEmit
cd src-tauri && cargo check  # Rust types
pnpm tauri build             # release bundle
```

Vendor sync:

```bash
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
git diff --stat vendor/
git add vendor/ && git commit -m "sync vendor: <reason>"
```

Install (macOS):

```bash
pnpm tauri build --bundles app
bash scripts/install.sh
# → /usr/local/bin/reflect-desktop + ~/Applications/ReflectDesktop.app
```
