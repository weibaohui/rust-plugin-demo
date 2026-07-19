# ============================================================================
# plugkit — Rust 通用插件管理框架
#
# make        → 构建框架 + 前端
# make run    → 构建一切 + 安装到 bin/ + 启动宿主
# ============================================================================

REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
BIN_DIR         = $(REPO_ROOT)bin
PLUGIN_DIR      = $(BIN_DIR)/plugins

# 自动发现所有插件目录（包含 Cargo.toml 且 crate-type 含 dylib 的）
PLUGIN_DIRS    := $(shell find "$(REPO_ROOT)examples" -name "Cargo.toml" -exec grep -l "dylib" {} \; | xargs -n1 dirname 2>/dev/null || echo "")

# ---------- 主目标 ----------
.PHONY: all build check test clean frontend run install plugins build-ui stop

all: frontend build

help:
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build:
	cargo build

check:
	cargo check

test:
	cargo test

clean:
	rm -rf $(BIN_DIR)
	cargo clean

frontend:
	@if [ ! -d frontend/node_modules ]; then \
		echo "==> frontend: npm install"; \
		(cd frontend && npm install); \
	fi
	@echo "==> frontend: npm run build"
	@(cd frontend && npm run build)
	@touch src/host.rs
	@echo "✓ built frontend/dist/ (touch src/host.rs to trigger recompile)"

# ---------- 插件构建 ----------
build-ui:
	@echo "==> 构建插件前端"
	@for dir in $(PLUGIN_DIRS); do \
		if [ -f "$$dir/Makefile" ]; then \
			echo "  -> $$dir"; \
			$(MAKE) -C "$$dir" build-ui 2>/dev/null || echo "  ⚠️  $$dir 前端构建跳过"; \
		fi \
	done
	@echo "✓ 插件前端构建完成"

plugins: build-ui
	@echo "==> 构建插件后端"
	@for dir in $(PLUGIN_DIRS); do \
		echo "  -> $$dir"; \
		$(MAKE) -C "$$dir" build 2>/dev/null || echo "  ⚠️  $$dir 后端构建跳过"; \
	done
	@echo "✓ 插件构建完成"

# ---------- 安装到 bin/ ----------
install: frontend plugins
	@echo "==> 构建 plugkit 框架..."
	@cargo build --release 2>/dev/null || cargo build
	@echo "==> 安装到 $(BIN_DIR)..."
	@mkdir -p $(PLUGIN_DIR)
	@cp "$(REPO_ROOT)target/debug/plugkit" "$(BIN_DIR)/plugkit" 2>/dev/null || true
	@cp "$(REPO_ROOT)target/release/plugkit" "$(BIN_DIR)/plugkit" 2>/dev/null || true
	@for dir in $(PLUGIN_DIRS); do \
		dylib="$$(ls "$$dir/target/debug/"*.dylib 2>/dev/null | head -1)"; \
		if [ -n "$$dylib" ]; then \
			cp "$$dylib" "$(PLUGIN_DIR)/" && echo "  ✓ $$(basename $$dylib)"; \
		fi \
	done
	@echo "✓ 安装完成:"
	@echo "  $(BIN_DIR)/plugkit"
	@ls $(PLUGIN_DIR)/ 2>/dev/null | sed 's/^/  /'

run: install
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  plugkit 通用插件管理后台启动中...                       ║"
	@echo "║  后端 API:  http://localhost:3000/api                   ║"
	@echo "║  前端 UI:   http://localhost:3000/                      ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@cargo run

stop: ## 停止占用 3000 端口的 plugkit 服务
	@PID=$$(lsof -ti :3000 2>/dev/null); \
	if [ -z "$$PID" ]; then \
		echo "✓ 端口 3000 未被占用"; \
	else \
		echo "==> 终止占用 3000 端口的进程: $$PID"; \
		kill $$PID && sleep 1 && \
			( lsof -ti :3000 >/dev/null 2>&1 && kill -9 $$PID 2>/dev/null ) || true; \
		if lsof -ti :3000 >/dev/null 2>&1; then \
			echo "✗ 端口 3000 仍被占用"; exit 1; \
		else \
			echo "✓ 已停止"; \
		fi \
	fi

# ---------- debug ----------
list:
	@echo "plugkit 框架 (lib + bin):"
	@echo "  src/                — 框架核心 (lib)"
	@echo "  src/main.rs         — 通用插件宿主入口 (bin)"
	@echo "  frontend/           — 通用插件管理前端"
	@echo "  bin/                — 构建产物输出目录"
	@echo "    plugkit           — 宿主可执行文件"
	@echo "    plugins/          — 插件 dylib"
	@echo ""
	@echo "自动发现的插件目录:"
	@for dir in $(PLUGIN_DIRS); do echo "  $$dir"; done
	@echo ""
	@echo "构建框架:     make"
	@echo "安装到 bin/:  make install"
	@echo "运行:         make run"
