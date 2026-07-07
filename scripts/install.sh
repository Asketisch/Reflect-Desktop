#!/usr/bin/env bash
# install.sh — install ReflectDesktop binary and macOS .app
# Usage:
#   bash scripts/install.sh                  # default: /usr/local/bin, ~/Applications
#   bash scripts/install.sh --dry-run      # print actions without executing
#   bash scripts/install.sh --prefix=$HOME/.local

set -euo pipefail

PREFIX="${PREFIX:-/usr/local/bin}"
APP_DIR="${APP_DIR:-$HOME/Applications}"
DRY_RUN=false

for arg in "$@"; do
  case "$arg" in
    --dry-run)  DRY_RUN=true ;;
    --prefix=*) PREFIX="${arg#--prefix=}" ;;
    --app-dir=*) APP_DIR="${arg#--app-dir=}" ;;
    -h|--help)
      cat <<EOF
Usage: install.sh [--dry-run] [--prefix=PATH] [--app-dir=PATH]

  --prefix=PATH   Where to copy the binary (default: /usr/local/bin)
  --app-dir=PATH  Where to copy the .app bundle on macOS (default: ~/Applications)
  --dry-run       Print what would happen without modifying the system
  -h, --help      Show this help

EOF
      exit 0 ;;
    *) echo "unknown arg: $arg" >&2; exit 1 ;;
  esac
done

# Resolve repository root (parent of scripts/).
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

BIN_SRC="$REPO_ROOT/target/release/reflect-desktop"
APP_SRC="$REPO_ROOT/target/release/bundle/macos/ReflectDesktop.app"

run() {
  echo "+ $*"
  $DRY_RUN || eval "$@"
}

if [ ! -f "$BIN_SRC" ]; then
  echo "binary not found: $BIN_SRC"
  echo "build first: pnpm tauri build --bundles app"
  exit 1
fi

run "install -m 0755 '$BIN_SRC' '$PREFIX/reflect-desktop'"

if [ "$(uname)" = "Darwin" ] && [ -d "$APP_SRC" ]; then
  run "mkdir -p '$APP_DIR'"
  run "rm -rf '$APP_DIR/ReflectDesktop.app'"
  run "cp -R '$APP_SRC' '$APP_DIR/ReflectDesktop.app'"
  echo "installed .app → $APP_DIR/ReflectDesktop.app"
fi

echo "done. launch with: $PREFIX/reflect-desktop"
echo "                  or open $APP_DIR/ReflectDesktop.app on macOS"
