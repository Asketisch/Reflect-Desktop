# ReflectDesktop Agent Guide

All docs must be canonical, with no past commentary, only live state.

## Scope

This file is the agent contract for how to work in this repo.
Detailed navigation/runbooks live in:

- `docs/codebase-map.md` (task-oriented file map: "if you need X, edit Y")
- `docs/PROTOCOL_BRIDGE.md` (Tauri ↔ reflect-protocol envelope spec)
- `README.md` (setup, build, release, and broader project docs)
- `docs/CHANGELOG.md` (release notes)

## Project Snapshot

ReflectDesktop is a Tauri 2 + React 19 desktop GUI for the Reflect Agent.

- Frontend: React 19 + Vite + TanStack Router/Query (`src/`)
- Backend app: Tauri Rust process (`src-tauri/src/lib.rs`)
- Submodule: `reflect-*` 核心 crate 通过 git submodule 复用(见 `reflect-agent/`),升级流程见 [`SUBMODULE.md`](SUBMODULE.md)
- Shared app-core: `app-core/` (UI-agnostic reducer/state, shared with future Tauri shims)

## Non-Negotiable Architecture Rules

1. Put shared/domain backend logic in `app-core/` and `reflect-agent/crates/reflect-*` first.
2. The Tauri app is a thin adapter around `reflect_core::AgentThread`.
3. Do not duplicate logic between TUI and GUI; both consume the same `reflect-protocol` envelope.
4. Keep Tauri command names and payload shapes stable unless intentionally changing contracts.
5. Keep frontend IPC contracts in sync with backend command surfaces (`src/utils/commands/` ↔ `src-tauri/src/commands/mod.rs`).

## Backend Routing Rules

For backend behavior changes, follow this order:

1. Core crates (`reflect-agent/crates/reflect-*`,via submodule) — preferred for cross-runtime logic. If the change belongs upstream,改 Reflect-Agent 仓库并升级 submodule(见 `SUBMODULE.md`)。
2. App adapter and Tauri command surface (`src-tauri/src/lib.rs` + `commands/mod.rs`).
3. Frontend IPC wrapper (`src/utils/commands/{domain}.ts`).
4. If you add a backend command, update all relevant layers + tests + `docs/PROTOCOL_BRIDGE.md` + `docs/CHANGELOG.md`.

## Frontend Routing Rules

- Keep `src/main.tsx` as the composition root.
- Keep `src/router.tsx` as the route table.
- Move stateful orchestration into `src/features/<slice>/hooks/*` or `<slice>/use*Controller.ts` files co-located with the view.
- Keep presentational UI in feature components (`src/features/<slice>/<View>.tsx`).
- Keep Tauri calls in `src/utils/commands/` only (per-domain files); `src/utils/tauri.ts` and `src/utils/commands.ts` are compatibility barrels — do not add new wrappers there.
- Keep event subscription fanout in `src/services/agent.ts` and `src/services/agentEventBus.ts`.

## Import Aliases

Use project aliases for frontend imports (defined in `tsconfig.json` + `vite.config.ts`):

- `@/*` → `src/*`

## Key File Anchors

- Frontend composition root: `src/main.tsx`
- Frontend router: `src/router.tsx`
- IDE app shell: `src/features/shell/AppShell.tsx`
- Frontend IPC barrel (compat): `src/utils/tauri.ts` / `src/utils/commands.ts`
- Frontend IPC low-level bridge: `src/utils/bridge.ts` (`invoke` / `listen` with fallback)
- Frontend IPC wrappers (per-domain): `src/utils/commands/{domain}.ts` (e.g. `agent`, `sessions`, `memory`, `terminal`, `git`, `files`, `skills`, `hooks`, `workspaces`, `config`, `permissions`, `approvals`, `questions`, `plan`, `events`, `health`, `updates`, `allowlist`, `search`)
- Frontend agent store: `src/stores/agentStore.ts` (compat facade) → `src/stores/agent/` (implementation: `store.ts`, `reducer.ts`, `turns.ts`, `toast.ts`, `servers.ts`, `types.ts`, `useAgent.ts`, `index.ts`)
- Frontend agent hook (legacy re-export): `src/services/agent.ts`
- App command registry: `src-tauri/src/lib.rs`
- App state (AgentThread host): `src-tauri/src/state.rs`
- App state support modules (kept under `src-tauri/src/`): `state.rs`, `dock.rs`, `events.rs`, `mcp.rs`, `menu.rs`, `tray.rs`, `shortcut.rs`, plus private helpers `hook_store.rs`, `memory_store.rs`, `shell_sessions.rs`, `workspace_state.rs`.
- Tauri command surface: `src-tauri/src/commands/mod.rs` — thin barrel that re-exports the per-domain modules under `src-tauri/src/commands/{agent,allowlist,config,export,files,git,hooks,memory,search,sessions,shell,skills,update,workspaces}.rs` and the shared `error.rs` (defines `CommandError` / `CommandResult`).
- Cargo workspace: `Cargo.toml` (root) + `src-tauri/Cargo.toml`
- Submodule 核心: `reflect-agent/crates/reflect-*/` (只读镜像,不要直接改 —— 改 Reflect-Agent 仓库并升级 submodule)

For broader path maps, use `docs/codebase-map.md`.

## Protocol Invariants

- Wire format is `reflect_protocol::Event` / `Submission` (snake_case JSON).
- All `Op` variants → Tauri commands (see `docs/PROTOCOL_BRIDGE.md` §2).
- All `EventMsg` variants → single `reflect_event` channel; frontend dispatches by `msg.type`.
- Submission id ↔ Event id pairing for stream correlation.
- `EVENT_ID_NONE = ""` for lifecycle events (no matching submission).

## AgentThread Host State Invariants

- `MinimalAgent` (M2.x) is the wrapper that holds `Arc<AgentThread>` + `tokio::sync::broadcast` for session event fan-out.
- `install_agent_thread()` is called from Tauri `setup()` (post runtime init) — never from `manage()` (sync phase).
- Slow subscribers trigger `RecvError::Lagged(n)` — logged but task keeps running.
- macOS close button → hide to tray (do not exit) via `on_window_event`.

## Session Hierarchy Invariants

- `useSessions` (`src/features/sessions/hooks/useSessions.ts`) is the canonical hook for session list.
- Switching sessions → `reflect_replay_session(id)` then hydrate local state.
- Time bucketing (`Now/Today/Yesterday/ThisWeek/Older`) lives in the hook, not the sidebar component.

## Follow-up Behavior Map

For Queue vs Steer follow-up behavior, start here:

- Settings model + defaults: `src/features/settings/SettingsView.tsx` (uses `ConfigForm` + per-section components)
- Composer runtime behavior: `src/features/composer/Composer.tsx` (the canonical location); `src/features/messages/Composer.tsx` is now a thin re-export shim that re-exports from `features/composer/Composer`.
- Send intent routing: `src/stores/agent/index.ts::useAgentStore` (canonical) — `src/stores/agentStore.ts` is a compatibility facade.
- App/layout wiring: `src/features/shell/AppShell.tsx`, `src/router.tsx`

## App State Sync Checklist

When changing settings/persistence that affects both backend and frontend:

1. Backend (`src-tauri/src/state.rs` + the relevant `commands/<domain>.rs` / `commands/mod.rs`) updated.
2. Frontend IPC (`src/utils/commands/{domain}.ts`) updated.
3. Feature settings UI (`src/features/settings/SettingsView.tsx`, `ConfigForm.tsx`, `sections/DisplaySection.tsx`, `sections/NotificationsSection.tsx`, `sections/UpdatesSection.tsx`, `components/StructuredField.tsx`, `components/ComplexEditors.tsx`, `config/schema.ts`) updated.
4. Test coverage added.
5. `docs/PROTOCOL_BRIDGE.md` updated if wire format changed.
6. `docs/CHANGELOG.md` entry added.

## Design System Rule (High-Level)

Use existing design tokens (`src/styles/tokens.css`, dark-first with light/system themes) and primitives (`src/features/design-system/primitives/*`) for shared shell chrome. Do not reintroduce duplicated modal/toast/panel/popover shell styling in feature CSS. All views consume `token` via CSS Modules (no inline style hardcoded colors).

(See existing DS files and `DesignSystemView` catalog for implementation details.)

## Safety and Git Behavior

- Prefer safe git operations (`status`, `diff`, `log`).
- Do not reset/revert unrelated user changes.
- If unrelated changes appear, continue focusing on owned files unless they block correctness.
- If conflicts impact correctness, call them out and choose the safest path.
- Fix root cause, not band-aids.
- **Submodule 核心**: 不要直接改 `reflect-agent/crates/reflect-*`;改 Reflect-Agent 仓库后用 `git submodule update --remote` 升级(见 `SUBMODULE.md`)。

## Validation Matrix

Run validations based on touched areas:

- Always: `pnpm typecheck`
- Frontend behavior/state/hooks/components: `pnpm test`
- Rust backend changes: `cd src-tauri && cargo check`
- Use targeted tests for touched modules before full-suite runs when iterating.

## Quick Runbook

Core local commands (keep these inline for daily use):

```bash
pnpm install
pnpm tauri dev                # dev mode (HMR)
pnpm test                     # vitest
pnpm typecheck                # tsc --noEmit
cd src-tauri && cargo check   # Rust types
```

Release build:

```bash
pnpm tauri build              # full multi-platform
pnpm tauri build --bundles app  # macOS .app only
bash scripts/install.sh       # → /usr/local/bin/reflect-desktop + ~/Applications/ReflectDesktop.app
```

Submodule 升级(当 Reflect-Agent 发布新版本时,详见 [`SUBMODULE.md`](SUBMODULE.md)):

```bash
git submodule update --remote reflect-agent
git diff --submodule reflect-agent
git add reflect-agent && git commit -m "chore: bump reflect-agent submodule"
```

> 历史:本仓库曾用 `vendor/` + `scripts/vendor-sync.sh`(rsync 手动同步)复用核心 crate,
> 已迁移为 git submodule。旧的 vendor runbook 不再适用。

Focused test runs:

```bash
pnpm test -- src/features/settings/SettingsView.test.tsx
```

## Hotspots

Use extra care in high-churn/high-complexity files:

- `src-tauri/src/lib.rs` — Tauri builder + command handler registration (`invoke_handler` lists every public `reflect_*` command exported from `src-tauri/src/commands/mod.rs`).
- `src-tauri/src/state.rs` — AgentThread host; install timing matters.

## Oversized File Exceptions (>=500 LoC rationale)

The structural-refactor target is `<= 500 LoC` for first-party production files. Currently no production entry file in the codebase exceeds this threshold.

Sub-modules of the structural split are intentionally allowed to be small without an exception clause here; the principal unit of size discipline is the public entry point per domain (`commands/<domain>.rs`, `stores/agent/index.ts`, etc.).

If a future production entry needs to exceed 500 LoC, document the rationale here and prefer moving logic across crates (e.g. lifting reducer entries into `app-core::reducer::*`, lifting host logic into `app-core::host`) over mechanical slices inside the same file.
- `src-tauri/src/commands/mod.rs` — thin barrel that re-exports per-domain command bodies in `commands/{agent,allowlist,config,export,files,git,hooks,memory,search,sessions,shell,skills,update,workspaces}.rs` (the actual command bodies live here; the sub-`mod.rs` keeps module wiring only).
- `src-tauri/src/events.rs` — event forwarder, single channel.
- `src/stores/agent/store.ts` — the live Zustand store; `src/stores/agent/index.ts` re-exports `useAgentStore` / `reduceEvent` / `useAgent` and the type union; the user-facing compat entry is `src/stores/agentStore.ts`, which re-exports from `./agent`.
- `src/features/shell/AppShell.tsx` — IDE 5-pane layout; sidebar/inspector toggle, session routing. Stateful shell interactions are extracted into `src/features/shell/hooks/useCommandPaletteShortcut.ts`, `usePaletteActions.ts`, `useThemeCycle.ts`.
- `src/features/messages/MessageList.tsx` — chat rendering + Collapsible items (Composer now lives at `src/features/composer/Composer.tsx`; `src/features/messages/Composer.tsx` is a thin re-export shim).
- `src/utils/bridge.ts` — low-level `invoke` / `listen` with fallback. `src/utils/tauri.ts` and `src/utils/commands.ts` are compatibility barrels; new wrappers go into `src/utils/commands/{domain}.ts`.
- `src/utils/i18n.ts` — compatibility barrel; runtime split into `src/utils/i18n/{context.tsx,locale.ts,interpolate.ts,lookup.ts,types.ts}` and `src/utils/i18n/strings/index.ts` merging domain catalogs under `src/utils/i18n/strings/{about,app,apps,chat,collaboration,common,composer,debug,design,dictation,files,git,home,inspector,memory,mobile,modal,models,notifications,palette,permissionMode,plan,prompts,settings,shell,sidebar,skills,slash,terminal,threads,toast,update,workspaces}.ts`.
- `src/router.tsx` — TanStack Router route table.
- `src/styles/tokens.css` — design tokens; dark/light/system themes — single source of truth.
- `scripts/vendor-sync.sh` — **已移除**(vendor 模式已迁移为 submodule)。核心升级见 `SUBMODULE.md`。
- View/controller split: `src/features/terminal/TerminalView.tsx` reads from `src/features/terminal/useTerminalController.ts`; `src/features/memory/MemoryView.tsx` reads from `src/features/memory/useMemoryController.ts`; `src/features/modals/index.tsx` orchestrates `ApprovalModal` / `QuestionModal` / `AskUserModal` / `PlanReadyModal` whose bodies live alongside it.

## Canonical References

- Task-oriented code map: `docs/codebase-map.md`
- Protocol envelope: `docs/PROTOCOL_BRIDGE.md`
- Architecture: `docs/ARCHITECTURE.md`
- Setup/build/release/test commands: `README.md`
- Change log: `docs/CHANGELOG.md`
- GUI design history: `docs/gui/00-index.md`
