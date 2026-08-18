#!/usr/bin/env bash
# dump-ts-types.sh —— 从 Rust 端 reflect-protocol 类型经 JSON Schema 重新生成 `src/types/protocol.ts`。
#
# 流水线：
#   1. cargo run -p reflect-protocol --example dump_schema
#        → 生成 JSON Schema（基于协议类型上的 schemars JsonSchema derive）
#   2. npx json2ts /tmp/reflect-schema.json
#        → 生成 src/types/protocol.ts
#
# 为什么需要它（B1-01）：
#   此前 protocol.ts 是手写的 33 行摘要，每次 `Op` / `EventMsg` 新增
#   都会与 Rust 端漂移。现在 reflect-agent/crates/protocol/reflect-protocol/src/op.rs 中
#   任何新 `Op::Foo` 变体，运行本脚本后都会自动出现在 TS 类型中。
#
# 用法：
#   bash scripts/dump-ts-types.sh
#   pnpm run types:gen
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SCHEMA_TMP="/tmp/reflect-schema.json"
TS_OUT="src/types/protocol.generated.ts"

echo "→ 正在从 reflect-protocol 生成 JSON Schema..."
cargo run --quiet -p reflect-protocol --example dump_schema 2>/dev/null > "$SCHEMA_TMP"
echo "  schema: $SCHEMA_TMP ($(wc -l < "$SCHEMA_TMP") lines, $(stat -f%z "$SCHEMA_TMP" 2>/dev/null || stat -c%s "$SCHEMA_TMP") bytes)"

# json2ts 需要网络。若不可用，则保留手写的 src/types/protocol.ts；
# 本脚本只是重新生成辅助工具，不是构建依赖。
if ! command -v npx >/dev/null 2>&1; then
  echo "  ⚠ 未找到 npx；跳过 json2ts（重新生成需要 Node.js）"
  exit 0
fi

echo "→ 运行 json2ts → $TS_OUT..."
npx --yes json2ts --cwd "$REPO_ROOT" "$SCHEMA_TMP" > "$TS_OUT" 2>/dev/null || {
  echo "  ⚠ json2ts 失败（离线或注册表问题）；$TS_OUT 未写入"
  exit 0
}

# 写入头部横幅，明确标记生成文件。
cat > "${TS_OUT}.bak" <<'BANNER'
/**
 * 自动生成 —— 请勿手改。
 *
 * 来源：reflect-agent/crates/protocol/reflect-protocol 类型，经 `cargo run --example dump_schema` 生成
 * 重新生成：`bash scripts/dump-ts-types.sh` 或 `pnpm run types:gen`
 */
BANNER
cat "$TS_OUT" >> "${TS_OUT}.bak"
mv "${TS_OUT}.bak" "$TS_OUT"

echo "  → 已写入 $TS_OUT"
echo "✓ 完成"
