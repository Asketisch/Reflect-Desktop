# ── ReflectDesktop Makefile ─────────────────────────────────────────
# Tauri 2 桌面应用 —— 转调现有 pnpm / scripts/build.sh,不重写逻辑。
# 核心层由 reflect-agent submodule 提供。
.PHONY: install dev build build-fast build-universal dist test test-native check clean gc help

# ── 前端 ──────────────────────────────────────────────────────────────
NODE_INSTALLER ?= pnpm

## install: 安装前端依赖(pnpm install)
install:
	$(NODE_INSTALLER) install

## dev: 启动 Vite 开发服务器(前端热重载,需另开 tauri dev)
dev:
	$(NODE_INSTALLER) dev

## build: 构建前端(tsc + vite build,产物到 dist/)
build:
	$(NODE_INSTALLER) run build

## build-fast: 构建前端(typecheck only,跳过 vite build,快速校验)
build-fast:
	$(NODE_INSTALLER) run typecheck

# ── 原生打包(转调 scripts/build.sh)─────────────────────────────────
# scripts/build.sh 已支持跨平台(mac/linux/win)、--fast、--universal、
# --bundles、--dry-run 等,这里只做便捷封装。

## dist: 构建当前平台原生分发包(app/dmg | deb,appimage | msi)
dist:
	bash scripts/build.sh

## dist-fast: 快速构建(release-fast profile,无 LTO,约 2x 快)
dist-fast:
	bash scripts/build.sh --fast

## dist-universal: macOS 通用二进制(arm64 + x86_64)
dist-universal:
	bash scripts/build.sh --universal

## dist-dry: 预览打包命令(不实际执行)
dist-dry:
	bash scripts/build.sh --dry-run

# ── 测试 ──────────────────────────────────────────────────────────────
## test: 前端单元测试(vitest)
test:
	$(NODE_INSTALLER) run test

## test-native: 后端 cargo 测试(含 reflect-agent 核心 crate)
test-native:
	cargo nextest run --no-fail-fast

# ── 检查与清理 ────────────────────────────────────────────────────────
## check: 前端 typecheck + 后端 cargo check
check: build-fast
	cargo check --workspace

## clean: 清理前端 dist + 后端 target
clean:
	rm -rf dist
	cargo clean

## gc: 清掉 cargo debug 里不会自动 GC 的旧中间产物
gc:
	rm -rf target/debug/incremental target/debug/.fingerprint
	rm -rf target/debug/examples
	@echo "✅ removed target/debug/{incremental,.fingerprint,examples}"

## help: Show this message
help:
	@echo "ReflectDesktop targets:"
	@echo ""
	@echo "  前端:"
	@echo "    install        - pnpm install"
	@echo "    dev            - Vite 开发服务器"
	@echo "    build          - 前端构建(tsc + vite build)"
	@echo "    build-fast     - 前端 typecheck(快速)"
	@echo ""
	@echo "  原生打包(转调 scripts/build.sh):"
	@echo "    dist           - 构建当前平台原生包(app/dmg | deb,appimage | msi)"
	@echo "    dist-fast      - release-fast profile(无 LTO,快)"
	@echo "    dist-universal - macOS 通用二进制(arm64 + x86_64)"
	@echo "    dist-dry       - 预览打包命令(不实际执行)"
	@echo ""
	@echo "  测试:"
	@echo "    test           - 前端 vitest"
	@echo "    test-native    - 后端 cargo nextest"
	@echo ""
	@echo "  其它:"
	@echo "    check          - 前端 typecheck + 后端 cargo check"
	@echo "    clean          - 清理 dist + target"
	@echo "    gc             - 清 target/debug 旧中间产物"
