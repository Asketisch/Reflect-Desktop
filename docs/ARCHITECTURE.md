# ReflectDesktop — Architecture

> Tauri 2 + React 19 desktop GUI for Reflect Agent. This document
> supersedes the older `Reflect-Agent/docs/gui/03-architecture.md`.

## 1. Top-level layout

```
ReflectDesktop/
├── vendor/                  # git subtree: 21 reflect-* crates mirrored from
│                            # Reflect-Agent/. rsync-managed via scripts/.
├── app-core/                # shared UI-agnostic reducer + state
├── src/                     # React 19 + Vite + feature slices
│   ├── features/
│   │   ├── shell/           # IDE 5-pane layout (AppShell, ActivityBar,
│   │   │                    #   TitleBar, StatusBar, Inspector, PageShell) +
│   │   │                    #   hooks/ (useCommandPaletteShortcut,
│   │   │                    #   usePaletteActions, useThemeCycle)
│   │   ├── messages/        # ChatView, MessageList, ToolCells, Collapsible
│   │   │                    # (Composer is a thin re-export shim →
│   │   │                    #   features/composer/Composer)
│   │   ├── composer/        # Composer, SlashPopup, MentionPicker, AttachmentBar
│   │   │                    #   + slashCommands / slashEngine / useComposerInput
│   │   │                    #   / useComposerSubmission / useAttachments /
│   │   │                    #   usePromptHistory
│   │   ├── sessions/        # Sidebar, BucketGroup, SessionItem, useSessions
│   │   ├── modals/          # ModalShell + Approval/Question/AskUser/PlanReady
│   │   │                    #   + ApprovalHistory (each in its own .tsx)
│   │   ├── terminal/        # TerminalView + useTerminalController
│   │   ├── memory/          # MemoryView + MemoryRow + MemoryAddForm +
│   │   │                    #   useMemoryController
│   │   ├── design-system/   # primitives (Button, Icon, Input, Card, ...) +
│   │   │                    #   DesignSystemView catalog
│   │   ├── settings/        # SettingsView + ConfigForm + sections/
│   │   │                    #   (DisplaySection / NotificationsSection /
│   │   │                    #   UpdatesSection) + components/ (StructuredField /
│   │   │                    #   ComplexEditors) + config/ (schema, toml)
│   │   └── {about,collaboration,debug,dictation,files,git,home,mobile,
│   │            models,notifications,plan,prompts,skills,threads,update,workspaces}/
│   ├── components/          # cross-slice atoms (e.g. Markdown)
│   ├── stores/
│   │   ├── agent/           # canonical impl: store.ts, reducer.ts, turns.ts,
│   │   │                    #   toast.ts, servers.ts, types.ts, useAgent.ts,
│   │   │                    #   index.ts (re-exports)
│   │   └── agentStore.ts    # compatibility facade → ./agent
│   ├── services/            # agent.ts (compat re-export) + agentEventBus.ts
│   ├── styles/              # tokens.css + base.css + typography.module.css
│   ├── utils/
│   │   ├── bridge.ts        # low-level invoke / listen + fallback
│   │   ├── tauri.ts         # compat barrel
│   │   ├── commands.ts      # compat barrel
│   │   ├── commands/        # per-domain IPC wrappers
│   │   ├── i18n.ts          # compat barrel
│   │   ├── i18n/            # runtime split (context.tsx, locale.ts,
│   │   │                    #   interpolate.ts, lookup.ts, types.ts)
│   │   └── i18n/strings/    # per-domain catalogs merged into STRINGS
│   └── types/               # generated from reflect-protocol's schema dump
├── src-tauri/               # Rust backend (Tauri 2)
│   ├── Cargo.toml           # bin name: reflect-desktop
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/main.json
│   ├── icons/               # 32/128/256/512 PNG + Windows .ico
│   └── src/
│       ├── main.rs          # binary entry
│       ├── lib.rs           # Tauri builder + setup + invoke_handler!
│       ├── state.rs         # MinimalAgent (M1.x stub) → reflect-core::AgentThread (M2.x)
│       ├── events.rs        # AgentThread → Tauri emit("reflect_event")
│       ├── mcp.rs           # MCP / LSP runtime (M3.x connection managers)
│       ├── commands/
│       │   ├── mod.rs       # thin barrel re-exporting each <domain>.rs
│       │   ├── error.rs     # CommandError / CommandResult
│       │   └── {agent,allowlist,config,export,files,git,hooks,memory,
│       │         search,sessions,shell,skills,update,workspaces}.rs
│       ├── {hook_store,memory_store,shell_sessions,workspace_state}.rs
│       │                    # private state helpers
│       ├── tray.rs          # M1.x stub; M2.x real impl
│       ├── menu.rs          # M1.x stub; M2.x real impl
│       ├── shortcut.rs      # M1.x stub; M2.x real impl
│       └── dock.rs          # M1.x stub; M2.x real impl
├── Cargo.toml               # workspace root
├── package.json             # frontend manifest (vite + tauri)
├── tsconfig.json / tsconfig.node.json
├── vite.config.ts           # @/ → src/ alias; dev server :5173
├── index.html               # SPA entry
├── docs/                    # USER_GUIDE + ARCHITECTURE + codebase-map +
│                              #   PROTOCOL_BRIDGE + CHANGELOG + gui/ (history)
└── scripts/
    ├── install.sh           # dual-binary install (script + .app)
    ├── vendor-sync.sh       # one-way rsync from Reflect-Agent/crates/* → vendor/
    └── build.sh             # cross-platform bundling (app, dmg, deb, appimage, msi)
```

## 2. IPC contract

See `docs/PROTOCOL_BRIDGE.md` for the request envelope and event
formats. The wire format is identical to `reflect-protocol::Event` /
`Submission` (snake_case JSON), avoiding custom serializers.

The full `#[tauri::command]` surface is registered in `src-tauri/src/lib.rs::invoke_handler` and defined per-domain in `src-tauri/src/commands/<domain>.rs`. The thin `src-tauri/src/commands/mod.rs` re-exports each domain module plus the shared `error` helpers (`CommandError` / `CommandResult`). One push event (`reflect_event`) is emitted from `events::forward_agent_events`; a terminal-output event (`reflect_terminal_output`) is emitted from `shell` command bodies.

- Health: `ping`
- Submission (Op dispatch): `reflect_submit`, `reflect_interrupt`, `reflect_compact`, `reflect_rewind`, `reflect_shutdown`, `reflect_tool_approval`, `reflect_hook_approval`, `reflect_plan_approval`, `reflect_enter_plan_mode`, `reflect_exit_plan_mode`, `reflect_set_effort`, `reflect_set_permission_mode`, `reflect_cycle_permission_mode`, `reflect_ask_user_question_response`, `reflect_ask_user_input_response`
- Sessions / export: `reflect_list_sessions`, `reflect_rename_session`, `reflect_delete_session`, `reflect_replay_session`, `reflect_export_session`, `reflect_export_session_markdown`
- Diagnostic / config / tools: `reflect_agent_status`, `reflect_get_config`, `reflect_save_config`, `reflect_list_tools`
- Domain management: `reflect_list_workspaces`, `reflect_set_workspace`, `reflect_current_workspace`, `reflect_list_skills`, `reflect_list_memory`, `reflect_add_memory`, `reflect_remove_memory`, `reflect_list_hooks`, `reflect_toggle_hook`
- Git / shell / files / search / allowlist / update: `reflect_git_status`, `reflect_git_diff`, `reflect_git_log`, `reflect_run_shell`, `reflect_kill_shell`, `reflect_list_shell_sessions`, `reflect_list_dir`, `reflect_read_file`, `reflect_search_files`, `reflect_load_allowlist`, `reflect_save_allowlist`, `reflect_check_allowlist`, `reflect_check_update`
- Dock (macOS): `reflect_set_dock_badge`

## 3. Build pipeline

```text
                pnpm tauri dev/build
                       │
        ┌──────────────┴──────────────┐
        ▼                              ▼
   frontend                        src-tauri/
   (Vite + tsc)                   (Tauri 2 + cargo build)
        │                              │
        │  bundle into dist/           │  outputs binary + .app/.msi/.deb
        └──────────────┬───────────────┘
                       ▼
            target/release/...
            ├── reflect-desktop (Mach-O)
            ├── bundle/macos/ReflectDesktop.app
            └── bundle/{dmg,msi,deb,appimage}/...
```

`scripts/install.sh` consumes the binary + bundle. `scripts/vendor-sync.sh`
mirrors changes from `Reflect-Agent/crates/*` into `vendor/*`. Both can
run independently — there is no build-time dependency between them.

## 4. State flow

```text
┌───────────────── React 19 (single Zustand store) ──────────────────┐
│ features/shell/AppShell (ActivityBar / Sidebar / Main / Inspector) │
│   ↑                                                                   │
│   │ shell/hooks/{useCommandPaletteShortcut, usePaletteActions,       │
│   │   useThemeCycle}                                                  │
│ features/messages/ChatView (MessageList + Composer re-export)        │
│   ↑                                                                   │
│   features/composer/{Composer, SlashPopup, useComposerInput,         │
│   │   useComposerSubmission, useAttachments, usePromptHistory}       │
│ features/memory/MemoryView ← useMemoryController                     │
│ features/terminal/TerminalView ← useTerminalController               │
│ features/modals/ModalStack (Approval/Question/AskUser/PlanReady)     │
│   ↑                                                                   │
│   │ useAgentStore (stores/agent/store.ts) — single source of truth    │
│   │   dispatcher: stores/agent/reducer.ts                            │
│   │ useAgent (services/agent.ts) — compat re-export                  │
│   │ event fan-out: services/agentEventBus.ts (refcounted)             │
│   ▼                                                                   │
│ onReflectEvent → agentEventBus → reducer (agent/reducer.ts)          │
└─────────────────────────┬─────────────────────────────────────────┘
                          │ Tauri 2 IPC (invoke / listen)
┌─────────────────────────▼─────────────────────────────────────────┐
│ src-tauri/                                                          │
│   bridge: src/utils/bridge.ts (invoke/listen + fallback)            │
│   commands barrel: src-tauri/src/commands/mod.rs                    │
│     └─ per-domain bodies under src-tauri/src/commands/<domain>.rs   │
│   state::MinimalAgent (M2.x)                                        │
│     ├─ mpsc Sender<Submission>                                       │
│     ├─ broadcast Receiver<Event> fan-out                             │
│     ├─ shell_sessions / hook_store / memory_store / workspace_state │
│     └─ MinimalAgentInner                                            │
│                                                                       │
│      ↔ reflect_core::AgentThread                                     │
│     + 16 builtin tools (Bash/Read/Write/Edit/...)                     │
│     + ModelRegistry                                                  │
│     + ConfigWatcher                                                  │
│     + reflect-rollout::JsonlRolloutWriter                            │
└────────────────────────────────────────────────────────────────────┘
```

## 5. Vendor mirror

`vendor/` contains a **read-only** mirror of the reflect-* crates,
maintained by `scripts/vendor-sync.sh`. The mirror pattern lets the GUI
keep its own release cadence without depending on the agent repo's CI,
while still benefiting from bug fixes shipped into reflect-protocol /
reflect-core / etc.

When updating:

```bash
# Pull latest from a fork or local clone of ReflectAgent
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
git diff --stat vendor/             # review
git add vendor/
git commit -m "sync vendor: <reason>"
```

The mirror intentionally excludes `reflect-tui`, `reflect-exec`,
`reflect-cli`, `reflect-mcp`, `reflect-lsp`, `reflect-discussion`, the
top-level `reflect` binary, and `reflect-pipeline` (GUI does not depend
on these). See the CRATES list in `scripts/vendor-sync.sh` for the exact
set kept in sync.
