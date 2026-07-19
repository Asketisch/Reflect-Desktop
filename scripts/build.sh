#!/usr/bin/env bash
# build.sh — Cross-platform build entry for ReflectDesktop.
#
# Detects current OS, picks the right Tauri bundle targets, optionally produces
# a macOS Universal Binary, and prints a summary of artifacts.
#
# Usage:
#   bash scripts/build.sh                       # current OS, default bundles
#   bash scripts/build.sh --fast                # release-fast profile (no LTO, faster)
#   bash scripts/build.sh --universal           # macOS: arm64 + x86_64 universal binary
#   bash scripts/build.sh --bundles=app,dmg     # override bundle list
#   bash scripts/build.sh --target=x86_64-apple-darwin  # explicit rustc target
#   bash scripts/build.sh --no-frontend         # skip frontend rebuild
#   bash scripts/build.sh --dry-run             # print what would run, don't execute
#   bash scripts/build.sh --help
#
# Default bundle targets per OS:
#   macOS    → app,dmg        (universal mode → same, but universal binary inside)
#   Linux    → deb,appimage
#   Windows  → msi
#
# Environment overrides:
#   SIGNING_IDENTITY   macOS codesign identity (default: ad-hoc, no notarization)
#   NODE_INSTALLER     'pnpm' (default) | 'npm' | 'yarn'
#   TAURI_CLI_ARGS     extra args appended verbatim to `pnpm tauri build`

set -euo pipefail

# ── Repo root ────────────────────────────────────────────────────────────
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# ── Defaults ─────────────────────────────────────────────────────────────
DRY_RUN=false
FAST=false
UNIVERSAL=false
NO_FRONTEND=false
BUNDLES_OVERRIDE=""
TARGET_OVERRIDE=""
NODE_INSTALLER="${NODE_INSTALLER:-pnpm}"
SIGNING_IDENTITY="${SIGNING_IDENTITY:-}"

# ── Arg parsing ──────────────────────────────────────────────────────────
# Accept both `--flag value` and `--flag=value` forms for value-taking flags.
while [ $# -gt 0 ]; do
  arg="$1"
  case "$arg" in
    --fast)        FAST=true ;;
    --universal)   UNIVERSAL=true ;;
    --no-frontend) NO_FRONTEND=true ;;
    --dry-run)     DRY_RUN=true ;;
    --bundles)     BUNDLES_OVERRIDE="${2:?--bundles requires a value}"; shift ;;
    --bundles=*)   BUNDLES_OVERRIDE="${arg#--bundles=}" ;;
    --target)      TARGET_OVERRIDE="${2:?--target requires a value}"; shift ;;
    --target=*)    TARGET_OVERRIDE="${arg#--target=}" ;;
    -h|--help)
      cat <<EOF
Usage: bash scripts/build.sh [options]

Options:
  --fast               Use release-fast profile (no LTO, ~2x faster, larger binary)
  --universal          macOS only: build universal arm64+x86_64 binary
  --bundles=LIST       Override bundle list (e.g. app,dmg | deb,appimage | msi)
  --target=TRIPLE      Pass explicit rustc target to tauri build (advanced)
  --no-frontend        Skip 'pnpm build' (assume dist/ is current)
  --dry-run            Print commands without executing
  -h, --help           Show this help

Env:
  SIGNING_IDENTITY     macOS codesign identity (default: ad-hoc, unsigned)
  NODE_INSTALLER       pnpm (default) | npm | yarn
  TAURI_CLI_ARGS       Extra args appended to 'pnpm tauri build'

Per-OS default bundles:
  macOS   → app,dmg
  Linux   → deb,appimage
  Windows → msi
EOF
      exit 0 ;;
    *)
      echo "unknown arg: $arg (use --help)" >&2
      exit 1 ;;
  esac
  shift
done

# ── Run helper: echo + eval, respect DRY_RUN ─────────────────────────────
run() {
  echo "+ $*"
  if ! $DRY_RUN; then
    eval "$@"
  fi
}

# ── Trap: report abort on Ctrl-C ─────────────────────────────────────────
cleanup() {
  local code=$?
  if [ $code -ne 0 ]; then
    echo "build aborted (exit $code)" >&2
  fi
  exit $code
}
trap cleanup EXIT INT TERM

# ── OS detection ─────────────────────────────────────────────────────────
OS_NAME="$(uname -s)"
case "$OS_NAME" in
  Darwin)
    PLATFORM="macos"
    DEFAULT_BUNDLES="app,dmg"
    ;;
  Linux)
    PLATFORM="linux"
    DEFAULT_BUNDLES="deb,appimage"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    PLATFORM="windows"
    DEFAULT_BUNDLES="msi"
    ;;
  *)
    echo "unsupported OS: $OS_NAME" >&2
    echo "ReflectDesktop can be built on macOS, Linux, or Windows (Git Bash)." >&2
    exit 1
    ;;
esac

# --universal only meaningful on macOS.
if $UNIVERSAL && [ "$PLATFORM" != "macos" ]; then
  echo "WARN: --universal is macOS-only; ignoring on $PLATFORM" >&2
  UNIVERSAL=false
fi

BUNDLES="${BUNDLES_OVERRIDE:-$DEFAULT_BUNDLES}"

echo "── ReflectDesktop build ───────────────────────────────────"
echo "platform:      $PLATFORM ($OS_NAME)"
echo "bundles:       $BUNDLES"
echo "profile:       $([ $FAST = true ] && echo release-fast || echo release)"
echo "universal:     $UNIVERSAL"
echo "target:        ${TARGET_OVERRIDE:-<host>}"
echo "dry-run:       $DRY_RUN"
echo "───────────────────────────────────────────────────────────"

# ── Toolchain checks ─────────────────────────────────────────────────────
need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required tool: $1" >&2
    exit 1
  }
}
need_cmd "$NODE_INSTALLER"
need_cmd cargo
need_cmd rustc

# ── Frontend deps ────────────────────────────────────────────────────────
if [ ! -d node_modules ]; then
  echo "+ $NODE_INSTALLER install (node_modules missing)"
  $DRY_RUN || "$NODE_INSTALLER" install
fi

# ── Determine rustc target ───────────────────────────────────────────────
TARGET_ARGS=()
if [ -n "$TARGET_OVERRIDE" ]; then
  TARGET_ARGS=(--target "$TARGET_OVERRIDE")
elif $UNIVERSAL; then
  # Tauri honors universal-apple-darwin via the 'apple' universal target.
  # Requires both aarch64-apple-darwin + x86_64-apple-darwin toolchains installed.
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed 2>/dev/null | grep -q "^$t$"; then
      echo "+ rustup target add $t (required for universal binary)"
      $DRY_RUN || rustup target add "$t"
    fi
  done
  TARGET_ARGS=(--target universal-apple-darwin)
fi

# ── Profile arg ──────────────────────────────────────────────────────────
PROFILE_ARGS=()
if $FAST; then
  PROFILE_ARGS=(-- --profile release-fast)
fi

# ── Bundles arg ──────────────────────────────────────────────────────────
BUNDLES_ARGS=(--bundles "$BUNDLES")

# ── macOS signing hook ───────────────────────────────────────────────────
if [ "$PLATFORM" = "macos" ] && [ -n "$SIGNING_IDENTITY" ] && ! $DRY_RUN; then
  export APPLE_SIGNING_IDENTITY="$SIGNING_IDENTITY"
fi

# ── Frontend build ───────────────────────────────────────────────────────
if ! $NO_FRONTEND; then
  run "$NODE_INSTALLER run build"
fi

# ── Assemble final command ───────────────────────────────────────────────
# Note: under `set -u`, expanding an empty array errors out. The `${arr[@]+"${arr[@]}"}`
# idiom expands to nothing when the array is empty and to its elements otherwise.
# shellcheck disable=SC2206,SC2086
CMD=("$NODE_INSTALLER" tauri build
  ${TARGET_ARGS[@]+"${TARGET_ARGS[@]}"}
  ${BUNDLES_ARGS[@]+"${BUNDLES_ARGS[@]}"}
  ${PROFILE_ARGS[@]+"${PROFILE_ARGS[@]}"}
  ${TAURI_CLI_ARGS:-})

# ── Run the build ────────────────────────────────────────────────────────
run "${CMD[*]}"

# ── Resolve bundle output dir for summary ────────────────────────────────
PROFILE_DIR="release"
if $FAST; then
  PROFILE_DIR="release-fast"
fi

# When --target is used, Tauri nests output under target/<target>/<profile>/bundle.
# When universal-apple-darwin is used, output is under target/universal-apple-darwin/<profile>/bundle.
TARGET_SUBDIR=""
if [ -n "$TARGET_OVERRIDE" ]; then
  TARGET_SUBDIR="$TARGET_OVERRIDE/"
elif $UNIVERSAL; then
  TARGET_SUBDIR="universal-apple-darwin/"
fi

case "$PLATFORM" in
  macos)   BUNDLE_DIR="target/${TARGET_SUBDIR}${PROFILE_DIR}/bundle/macos" ;;
  linux)   BUNDLE_DIR="target/${TARGET_SUBDIR}${PROFILE_DIR}/bundle/deb target/${TARGET_SUBDIR}${PROFILE_DIR}/bundle/appimage" ;;
  windows) BUNDLE_DIR="target/${TARGET_SUBDIR}${PROFILE_DIR}/bundle/msi" ;;
esac

# ── Artifact summary ─────────────────────────────────────────────────────
# Human-readable size helper: macOS `du -h` and Linux `du -h` agree; fallback
# to raw bytes if numfmt/du unavailable.
human_size() {
  local path="$1"
  if command -v du >/dev/null 2>&1; then
    # -sh: summary, human-readable. -L not needed; we want bundle dir size.
    du -sh "$path" 2>/dev/null | cut -f1
  else
    echo "?"
  fi
}

echo ""
echo "── artifacts ──────────────────────────────────────────────"
if $DRY_RUN; then
  echo "(dry-run: skipping artifact scan of $BUNDLE_DIR)"
else
  FOUND=0
  for dir in $BUNDLE_DIR; do
    if [ -d "$dir" ]; then
      # List top-level entries (skip . ..). Bundles like .app/.dmg/.deb/.msi/
      # .AppImage all appear as entries directly under the bundle/<type>/ dir.
      for entry in "$dir"/*; do
        [ -e "$entry" ] || continue   # glob didn't match
        base="$(basename "$entry")"
        size="$(human_size "$entry")"
        printf "  %8s  %s\n" "$size" "$entry"
      done
      FOUND=1
    fi
  done
  if [ "$FOUND" -eq 0 ]; then
    echo "WARN: no artifacts found under expected paths ($BUNDLE_DIR)" >&2
    echo "      check target/${TARGET_SUBDIR}${PROFILE_DIR}/bundle/ for actual output" >&2
  fi
fi
echo "───────────────────────────────────────────────────────────"
echo "done."
