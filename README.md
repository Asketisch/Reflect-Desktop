# ReflectDesktop

> Standalone Tauri 2 + React 19 desktop GUI for [Reflect Agent](https://github.com/CNB/Reflect-Agent).
> Zero-modification integration: this repo consumes `reflect-*` crates via
> `vendor/` mirror + a small IPC adapter on top of `reflect-protocol`.

## Why this repo exists

ReflectAgent's terminal-mode frontend (`reflect tui`) and the headless exec path
(`reflect exec "..."`) carry the same `reflect-core::AgentThread`. A desktop
GUI wants the same threads but in a Tauri-rendered window. Rather than
shipping a Tauri shim inside ReflectAgent (which would couple Tauri 2 build
artifacts into a headless agent tool), ReflectDesktop keeps all GUI code here
and only **consumes** the agent through `vendor/reflect-*` snapshots pulled
on demand.

## Layout

```
ReflectDesktop/
├── vendor/         # git subtree mirror of reflect-* crates
│                    # updated via bash scripts/vendor-sync.sh
├── app-core/       # shared UI-agnostic reducer & state (was reflect-app-core)
├── src/            # React 19 + Vite + 24 feature-sliced components
├── src-tauri/      # Tauri 2 Rust backend (binary name: reflect-desktop)
├── Cargo.toml      # workspace root for vendor + app-core + src-tauri
└── scripts/        # install.sh · vendor-sync.sh · doctor.sh
```

## Build

```bash
pnpm install
pnpm tauri build --bundles app      # macOS .app
pnpm tauri build                    # full multi-platform target set
```

The build yields:

- `target/release/reflect-desktop` (Mach-O / ELF / PE)
- `target/release/bundle/macos/ReflectDesktop.app` on macOS
- `target/release/bundle/{dmg,msi,deb,appimage}/...` cross-platform targets

## Install

```bash
bash scripts/install.sh
# installs to /usr/local/bin/reflect-desktop
# copies .app to ~/Applications/ on macOS
```

## Sync agent sources

When Reflect-Agent exports a new release:

```bash
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
# OR supply a tag:
bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent v1.0.0
```

This is the only sanctioned way to refresh vendor. Editing files under
`vendor/` directly is not allowed (next sync will overwrite).

## Roadmap

See `docs/USER_GUIDE.md` for the feature inventory. Implementation timeline:

| Phase | Slice | Status |
|---|---|---|
| MVP | scaffold + protocol bridge + 3-pane layout | done in M1.x (carried over) |
| B1..B6 | layout, threads, messages, composer, modals, statusbar+settings | in flight |
| B7..B10 | models, prompts, plan, skills, files, git, workspaces, about, notifications, update, debug, collaboration | after B6 |
| C1..C4 | tray, global hotkey, dock + badge, dual-binary install | polish phase |
