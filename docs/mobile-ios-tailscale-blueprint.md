# iOS + Tailscale Setup (TCP) — ReflectDesktop Remote Backend

> **Status: WIP / placeholder**. iOS builds are planned for M3.x. This document captures the intended setup so reviewers can follow along.

## Goal

Connect an iOS build of ReflectDesktop to a desktop-hosted daemon over a Tailscale tailnet, so the phone can drive a session without spawning `AgentThread` locally.

## Prereqs

1. Tailscale installed and signed in on both desktop and iPhone (same tailnet).
2. Desktop ReflectDesktop running with a known data dir (`settings.json` + `workspaces.json`).
3. iOS Xcode + Command Line Tools installed; Rust iOS targets installed:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
```

## Desktop setup

1. Open ReflectDesktop → Settings → Server.
2. Set a "Remote backend token" (random 32-byte hex).
3. Click **Start daemon** under "Mobile access daemon".
4. In **Tailscale helper** → **Detect Tailscale**, note the suggested host, e.g. `your-mac.your-tailnet.ts.net:4732`.

## iOS setup

1. Open ReflectDesktop (iOS) → Settings → Server.
2. Enter the desktop Tailscale host and the same token.
3. Tap **Connect & test**; confirm success.

## Headless daemon control

```bash
cd src-tauri
cargo build --bin reflect-desktop-daemon --bin reflect-desktop-daemonctl
./target/debug/reflect-desktop-daemonctl status
./target/debug/reflect-desktop-daemonctl start
./target/debug/reflect-desktop-daemonctl stop
./target/debug/reflect-desktop-daemonctl command-preview
./target/debug/reflect-desktop-daemonctl --listen 0.0.0.0:4732 --token <token> --data-dir /Users/admin/Library/Application\ Support/com.cnb.reflectdesktop.app start
```

Useful overrides:

- `--data-dir <path>`: app data dir containing `settings.json` / `workspaces.json`
- `--listen <addr>`: bind address override
- `--token <token>`: token override
- `--daemon-path <path>`: explicit `reflect-desktop-daemon` binary path
- `--json`: machine-readable output

## iOS Simulator

```bash
./scripts/build_run_ios.sh
# --simulator "<name>" | --target aarch64-sim | --skip-build | --no-clean
```

## iOS USB device

```bash
./scripts/build_run_ios_device.sh --list-devices
./scripts/build_run_ios_device.sh --device "<device name or identifier>" --team <TEAM_ID>
# --target aarch64 | --skip-build | --bundle-id <id>
```

First-time device setup:

1. iPhone unlocked and trusted with this Mac.
2. Developer Mode enabled on iPhone.
3. Pairing/signing approved in Xcode at least once.

If signing is not ready yet:

```bash
./scripts/build_run_ios_device.sh --open-xcode
```

## TestFlight release

```bash
./scripts/release_testflight_ios.sh
```

Auto-loads release metadata from `.testflight.local.env` (gitignored). Copy from `.testflight.local.env.example` first.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `connect refused` from iOS | Desktop daemon not running | Start daemon on desktop; confirm port 4732 is reachable |
| TLS handshake fails | Token mismatch | Re-enter token on both sides |
| `Lagged(N)` in logs | Slow iOS network | Reduce session event rate; increase broadcast buffer |
| Test fails with `no tailnet` | Tailscale off | Re-check Tailscale status on both devices |

## Notes

- Desktop daemon must stay running while iOS is connected.
- On iOS, terminal + dictation remain unavailable (planned for M3.x).