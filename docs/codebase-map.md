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

1. Frontend callsite: `src/features/**` (hooks/components)
2. Frontend IPC API: `src/utils/tauri.ts`
3. Tauri command registration: `src-tauri/src/lib.rs` (`invoke_handler`)
4. Tauri command body: `src-tauri/src/commands/mod.rs`
5. App state (MinimalAgent stub → AgentThread): `src-tauri/src/state.rs`
6. Event forwarder: `src-tauri/src/events.rs`
7. Reflect protocol types: `vendor/reflect-protocol/src/{event,event_msg,op,item}.rs`

If behavior must run in headless (CLI/TUI) too, the change lives in `vendor/reflect-*` first.

---

## If You Need X, Edit Y

| Need | Primary files |
| --- | --- |
| Add a new feature slice / route | `src/router.tsx`, `src/features/<slice>/<View>.tsx`, `src/features/shell/AppShell.tsx` |
| Add/change Tauri command from frontend | `src/utils/tauri.ts`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (handler list) |
| Add/change event handler in UI | `src/services/agent.ts` (`handle_event`), `src/features/<slice>/hooks/*` |
| Change composer (input box) | `src/features/messages/Composer.tsx`, `src/features/composer/SlashPopup.tsx`, `src/features/composer/slashCommands.ts` |
| Change chat rendering | `src/features/messages/MessageList.tsx`, `src/features/messages/ChatView.tsx` |
| Change IDE shell (layout / activity bar / status bar) | `src/features/shell/{AppShell,ActivityBar,TitleBar,StatusBar,Inspector,PageShell}.tsx` |
| Change approval / question / plan modal | `src/features/modals/ModalShell.tsx`, `src/features/modals/index.tsx` |
| Change session list (sidebar) | `src/features/sessions/components/Sidebar.tsx`, `src/features/sessions/hooks/useSessions.ts` |
| Change settings persistence | `src/features/settings/SettingsView.tsx`, `src/utils/tauri.ts`, `src-tauri/src/commands/mod.rs` |
| Change theme / design tokens | `src/styles/tokens.css`, `src/features/design-system/DesignSystemView.tsx` |
| Add tray icon / menu / shortcut | `src-tauri/src/{tray,menu,shortcut}.rs`, `src-tauri/src/lib.rs` |
| macOS dock badge | `src-tauri/src/dock.rs` |
| Add a new vendor crate | `scripts/vendor-sync.sh` (CRATES list), `Cargo.toml` workspace |
| Add a new test fixture | `src/test/setup.ts`, `vitest.config.ts` |
| Change protocol types | `vendor/reflect-protocol/src/*.rs` (sync via `scripts/vendor-sync.sh`) |

---

## Frontend Navigation

- Composition root: `src/main.tsx`
- Router (TanStack Router): `src/router.tsx`
- App layout shell (IDE 5-pane): `src/features/shell/AppShell.tsx` (+ ActivityBar/TitleBar/StatusBar/Inspector/PageShell)
- Tauri IPC wrapper: `src/utils/tauri.ts`
- Agent hook (event fanout + submit): `src/services/agent.ts`
- Global Zustand store: `src/stores/agentStore.ts`
- Theme infra: `src/utils/theme.ts`
- Design tokens / base reset: `src/styles/{tokens,base}.css` + `typography.module.css`
- Shared types: `src/types/protocol.ts`

### Feature slices

| Slice | Files | Notes |
| --- | --- | --- |
| `home` | `features/home/HomeView.tsx` | Dashboard / quick actions |
| `messages` | `features/messages/{ChatView,MessageList,Composer,Collapsible}.tsx` | Chat scrollback + rows + input |
| `composer` | `features/composer/{SlashPopup}.tsx` + `slashCommands.ts` | `/` popup (Composer lives in messages/) |
| `shell` | `features/shell/{AppShell,ActivityBar,TitleBar,StatusBar,Inspector,PageShell}.tsx` | IDE 5-pane layout |
| `modals` | `features/modals/{ModalShell,index}.tsx` | Approval / Question / Plan / AskUser |
| `sessions` | `features/sessions/{components/Sidebar,hooks/useSessions}.{tsx,ts}` + test | Time-bucketed sidebar |
| `settings` | `features/settings/SettingsView.tsx` + test | Display / Editor / Provider |
| `models` | `features/models/ModelsView.tsx` | Model picker |
| `workspaces` | `features/workspaces/WorkspacesView.tsx` | Workspace picker (M3.x) |
| `git` | `features/git/GitView.tsx` | Git panel (M3.x) |
| `files` | `features/files/FilesView.tsx` | File tree (M3.x) |
| `plan` | `features/plan/PlanView.tsx` | Plan mode UI |
| `terminal` | `features/terminal/TerminalView.tsx` | Terminal dock (M2.9+) |
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

- Command registry (what frontend can invoke): `src-tauri/src/lib.rs`
- Command bodies: `src-tauri/src/commands/mod.rs` (19 `#[tauri::command]`s)
- App state: `src-tauri/src/state.rs` (`MinimalAgent` stub → `reflect_core::AgentThread`)
- Event forwarder: `src-tauri/src/events.rs` (`forward_agent_events`)
- Tray / menu / shortcut / dock: `src-tauri/src/{tray,menu,shortcut,dock}.rs`
- Tauri config: `src-tauri/tauri.conf.json`
- Capabilities: `src-tauri/capabilities/main.json`
- Cargo workspace: `Cargo.toml` (workspace root) + `src-tauri/Cargo.toml`

### Tauri IPC surface (current = M1.x + M2.x stub)

19 commands + `ping` + `reflect_set_dock_badge`:

| Category | Commands |
| --- | --- |
| Health | `ping` |
| Submission | `reflect_submit`, `reflect_interrupt`, `reflect_compact`, `reflect_rewind`, `reflect_shutdown` |
| Approvals | `reflect_tool_approval`, `reflect_hook_approval`, `reflect_plan_approval` |
| Plan mode | `reflect_enter_plan_mode`, `reflect_exit_plan_mode` |
| Effort / Permission | `reflect_set_effort`, `reflect_set_permission_mode`, `reflect_cycle_permission_mode` |
| Ask user | `reflect_ask_user_question_response`, `reflect_ask_user_input_response` |
| Sessions | `reflect_list_sessions`, `reflect_rename_session`, `reflect_delete_session`, `reflect_replay_session` |
| Dock (macOS) | `reflect_set_dock_badge` |

Push events (single channel, dispatched by `msg.type`):

- `reflect_event` — payload is `reflect_protocol::Event` (snake_case JSON)

---

## Vendor Crates

`vendor/` contains a read-only mirror of `reflect-*` crates from the upstream `Reflect-Agent` repo, kept in sync via `scripts/vendor-sync.sh`. Crates mirrored:

`reflect-protocol`, `reflect-core`, `reflect-app-core`, `reflect-config`, `reflect-rollout`, `reflect-skills`, `reflect-memory`, `reflect-permissions`, `reflect-tools`, `reflect-llm`, `reflect-hooks`, `reflect-task`, `reflect-mcp`, `reflect-discussion`, `reflect-lsp`, `reflect-pipeline`, `reflect-stream`, `reflect-exec`, `reflect-cli`, `reflect-tui`, `reflect`, `reflect-binary-derive`.

Excluded: none (full mirror). Edit upstream first; then `bash scripts/vendor-sync.sh /path/to/Reflect-Agent main`.

---

## Events Map (Backend → Frontend)

- Backend emits via `src-tauri/src/events.rs::forward_agent_events` → `app.emit("reflect_event", &event)`.
- Frontend fanout hub: `src/services/agent.ts::useAgent` + `handle_event`.
- Parser guards: `src/utils/tauri.ts::onReflectEvent`.
- Type contract: `vendor/reflect-protocol/src/event_msg.rs` (Rust) ↔ `src/types/protocol.ts` (TS).

If event payload format changes, regenerate `src/types/protocol.ts` from `reflect-protocol`'s schema dump (`cargo run -p reflect-protocol --example dump_schema` → `npx json2ts`).

---

## Type Contract Files

Keep Rust and TypeScript contracts in sync:

- Rust backend types: `vendor/reflect-protocol/src/{event,event_msg,op,item}.rs`
- Frontend types: `src/types/protocol.ts`
- Settings: `src/features/settings/SettingsView.tsx` ↔ `src-tauri/src/state.rs`

This is required for submissions, events, settings, and session payloads.

---

## Conventions

- **Feature-sliced design**: each feature is a folder under `src/features/<slice>/`. Components live flat or in `components/`; hooks in `hooks/`; utils in `utils/`; tests co-located as `*.test.ts(x)`.
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