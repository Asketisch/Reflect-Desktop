# ReflectDesktop — Architecture

> Tauri 2 + React 19 desktop GUI for Reflect Agent. This document
> supersedes the older `Reflect-Agent/docs/gui/03-architecture.md`.

## 1. Top-level layout

```
ReflectDesktop/
├── vendor/                  # git subtree: 21 reflect-* crates mirrored from
│                            # Reflect-Agent/. rsync-managed via scripts/.
├── app-core/                # shared UI-agnostic reducer + state
├── src/                     # React 19 + Vite + 24 feature slices
│   ├── features/{about, app, collaboration, composer, debug, design-system,
│   │             dictation, files, git, home, layout, messages, mobile,
│   │             models, modals, notifications, plan, prompts, settings,
│   │             sessions, shared, skills, terminal, threads,
│   │             update, workspaces}/
│   ├── components/          # cross-slice atoms (e.g. Markdown, ModalShell)
│   ├── widgets/             # composite widgets (Topbar, BottomBar)
│   ├── stores/              # global Zustand stores (threadStore, agentStore,
│   │                        # uiStore, tauriStore)
│   ├── services/            # IPC wrappers (tauri.ts, agent.ts)
│   ├── utils/               # tauri invoke/listen glue + debounce / i18n
│   └── types/               # generated from reflect-protocol's schema dump
├── src-tauri/               # Rust backend (Tauri 2)
│   ├── Cargo.toml           # bin name: reflect-desktop
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/main.json
│   ├── icons/               # 32/128/256/512 PNG + Windows .ico
│   └── src/
│       ├── main.rs          # binary entry
│       ├── lib.rs           # Tauri builder + setup
│       ├── state.rs         # MinimalAgent (M1.x stub) → reflect-core::AgentThread (M2.x)
│       ├── events.rs        # AgentThread → Tauri emit("reflect_event")
│       ├── commands/{mod.rs}  # 19 #[tauri::command] s
│       ├── tray.rs          # M1.x stub; M2.x real impl
│       ├── menu.rs          # M1.x stub; M2.x real impl
│       ├── shortcut.rs      # M1.x stub; M2.x real impl
│       └── dock.rs          # M1.x stub; M2.x real impl
├── Cargo.toml               # workspace root
├── package.json             # frontend manifest (vite + tauri)
├── tsconfig.json / tsconfig.node.json
├── vite.config.ts           # @/ → src/ alias; dev server :5173
├── index.html               # SPA entry
├── docs/                    # USER_GUIDE + ARCHITECTURE + (planned)
│                              #   PRODUCT_LINKAGES + PROTOCOL_BRIDGE
└── scripts/
    ├── install.sh           # dual-binary install (script + .app)
    └── vendor-sync.sh       # one-way rsync from Reflect-Agent/crates/* → vendor/
```

## 2. IPC contract

See `docs/PROTOCOL_BRIDGE.md` (planned) for the request envelope and event
formats. The wire format is identical to `reflect-protocol::Event` /
`Submission` (snake_case JSON), avoiding custom serializers.

M1.x exposes 19 `#[tauri::command]`s and 1 push event (`reflect_event`):

- `ping`, `reflect_submit`, `reflect_interrupt`,
- `reflect_compact`, `reflect_rewind`, `reflect_shutdown`,
- `reflect_tool_approval`, `reflect_hook_approval`,
- `reflect_enter_plan_mode`, `reflect_exit_plan_mode`, `reflect_plan_approval`,
- `reflect_set_effort`, `reflect_set_permission_mode`, `reflect_cycle_permission_mode`,
- `reflect_ask_user_question_response`, `reflect_ask_user_input_response`,
- `reflect_list_sessions`, `reflect_rename_session`,
- `reflect_delete_session`, `reflect_replay_session`

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

## 4. State flow (M1.x stub → M2.x real)

```text
┌─────────────── React 19 ───────────────┐
│ features/messages/ChatView            │
│ features/composer/Composer            │
│ features/sessions/Sidebar (B2)        │
│ features/modals/{Approval|Question}…  │  ← modal-driven, B5
│   ↑                                    │
│   │ useAgent (services/agent.ts)       │
│   ▼                                    │
│ reflect_event (Tauri listen)          │
└─────────────┬──────────────────────────┘
              │ Tauri 2 IPC
┌─────────────▼──────────────────────────┐
│ src-tauri/                              │
│   events::forward_agent_events(...)    │
│   commands::{reflect_submit, ...}      │
│   state::MinimalAgent (M1.x)           │
│     ├─ mpsc Sender<Submission>         │
│     ├─ mpsc Receiver<Event> fan-out     │
│     └─ MinimalAgentInner               │
│                                          │
│      ↕ future (M2.x):                   │
│     reflect_core::AgentThread           │
│     + 22 builtin tools                  │
│     + ModelRegistry                     │
│     + ConfigWatcher                     │
│     + reflect-rollout::JsonlRolloutWriter│
└────────────────────────────────────────┘
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
