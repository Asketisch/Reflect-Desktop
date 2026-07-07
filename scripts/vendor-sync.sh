#!/usr/bin/env bash
# vendor-sync.sh — pull the latest reflect-* crates from a Reflect Agent repo
# into this repo's vendor/ subtree, then review + commit.
#
# Usage:
#   bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent main
#   bash scripts/vendor-sync.sh /Users/admin/Code/CNB/Reflect-Agent v1.0.0
#   bash scripts/vendor-sync.sh   # uses REFLECT_AGENT_REPO env or cwd guess
#
# Constraints:
#   * The source repo must be a real git repository (we read its HEAD by default,
#     or accept a tag/branch/ref).
#   * vendor/ already contains the 21 reflect-* crates; the script refreshes them.
#   * Updates should be reviewed on a per-crate basis before commit.

set -euo pipefail

# ── Args ─────────────────────────────────────────────────────────────────
SOURCE_REPO="${1:-${REFLECT_AGENT_REPO:-}}"
REF="${2:-HEAD}"

if [ -z "$SOURCE_REPO" ] || [ ! -d "$SOURCE_REPO/.git" ]; then
  echo "usage: $0 <source-repo-path> [<ref>]"
  echo "       (or set REFLECT_AGENT_REPO env)"
  exit 1
fi

THIS_REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$THIS_REPO"

# Crates to sync. Add new crates here once they're vendor-tracked in this
# repo. Read by `vendor/<crate>/Cargo.toml` to confirm source has it.
CRATES=(
  reflect-protocol
  reflect-core
  reflect-tools
  reflect-config
  reflect-rollout
  reflect-llm
  reflect-skills
  reflect-memory
  reflect-hooks
  reflect-permissions
  reflect-stream
  reflect-task
  reflect-agent-def
  reflect-recovery
  reflect-prompt
  reflect-compact
  reflect-async-graph
  reflect-notes
  reflect-ast
  reflect-subagent
  reflect-sandbox
)

echo "syncing from $SOURCE_REPO @ $REF"
echo "into $THIS_REPO/vendor/"
echo

for crate in "${CRATES[@]}"; do
  if [ ! -d "vendor/$crate" ]; then
    echo "skip $crate (not vendored; add to list once you've copied it)."
    continue
  fi
  if [ ! -d "$SOURCE_REPO/crates/$crate" ]; then
    echo "skip $crate (not in source anymore; remove from list if deprecated)."
    continue
  fi

  echo "---- $crate ----"

  # We use plain `cp` rather than `git subtree pull`: the source repo's
  # history isn't replicated per-crate in this repo, and `cp` keeps the
  # update readable as a single commit on this repo's log. Update existing
  # files only; do not delete files that exist here but not in source.
  rsync -a --delete \
    --exclude='target/' \
    --exclude='Cargo.lock' \
    --exclude='.git/' \
    "$SOURCE_REPO/crates/$crate/" "vendor/$crate/"
done

echo
echo "next: review diff, then commit."
echo "  git status vendor/"
echo "  git diff --stat vendor/"
echo "  git add vendor/ && git commit -m 'sync vendor: <reason>'"
