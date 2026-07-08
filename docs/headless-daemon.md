# Headless Daemon Management

> **Status: planned for M3.x**. ReflectDesktop will ship an optional daemon binary that runs `reflect_core::AgentThread` headlessly, accepting JSON-RPC connections from the Tauri shim (and from iOS clients over Tailscale).
>
> This document captures the intended interface so contributors know what to build toward.

## Why a daemon?

For remote/iOS scenarios, running `AgentThread` on a different host than the UI requires:

1. A long-lived process holding the thread.
2. JSON-RPC transport (over stdio or TLS).
3. Lifecycle control without the Tauri window.

The daemon is the host; the Tauri app becomes a thin RPC client when remote mode is selected.

## Binaries

Two binaries, both built from `src-tauri/`:

```bash
cd src-tauri
cargo build --bin reflect-desktop-daemon
cargo build --bin reflect-desktop-daemonctl
```

- `reflect-desktop-daemon` — headless server. Reads `settings.json` + `workspaces.json` from the app data dir; serves JSON-RPC over stdio (local) or TLS (remote).
- `reflect-desktop-daemonctl` — lifecycle CLI: `status` / `start` / `stop` / `command-preview`.

## Daemon lifecycle

```bash
# Show current daemon status
./target/debug/reflect-desktop-daemonctl status

# Start daemon using host/token from settings.json
./target/debug/reflect-desktop-daemonctl start

# Stop daemon
./target/debug/reflect-desktop-daemonctl stop

# Print equivalent daemon start command (for systemd / launchd)
./target/debug/reflect-desktop-daemonctl command-preview

# Machine-readable
./target/debug/reflect-desktop-daemonctl --json status
```

## Useful overrides

| Flag | Purpose |
|---|---|
| `--data-dir <path>` | App data dir containing `settings.json` / `workspaces.json` |
| `--listen <addr>` | Bind address override (default: `127.0.0.1:4732`) |
| `--token <token>` | Token override (otherwise read from `settings.json`) |
| `--daemon-path <path>` | Explicit daemon binary path |
| `--json` | Machine-readable output |

## JSON-RPC surface (planned)

Method names mirror Tauri commands in `src-tauri/src/lib.rs` (e.g. `reflect_submit`, `reflect_list_sessions`). Domain handlers live in `src-tauri/src/bin/reflect-desktop-daemon/rpc/*` mirroring the Tauri command files.

For the full planned surface, see [`codebase-map.md` Daemon Navigation](codebase-map.md#backend-navigation) (added when the daemon lands).

## Settings.json keys (planned)

```jsonc
{
  "remote": {
    "enabled": false,            // local vs remote mode (set per workspace)
    "host": "your-mac.tail.ts.net",
    "port": 4732,
    "token": "..."               // 32-byte hex
  }
}
```

## Startup (launchd on macOS, systemd on Linux)

The daemon is intended to be managed by the OS service manager. `command-preview` prints the exact command needed:

```bash
./target/debug/reflect-desktop-daemonctl command-preview
# reflect-desktop-daemon \
#   --data-dir /Users/admin/Library/Application\ Support/com.cnb.reflectdesktop.app \
#   --listen 0.0.0.0:4732 \
#   --token $(cat /Users/admin/Library/Application\ Support/com.cnb.reflectdesktop.app/remote.token)
```

## See also

- iOS + Tailscale blueprint: [`mobile-ios-tailscale-blueprint.md`](mobile-ios-tailscale-blueprint.md)
- Protocol envelope: [`PROTOCOL_BRIDGE.md`](PROTOCOL_BRIDGE.md)
- Architecture: [`ARCHITECTURE.md`](ARCHITECTURE.md)