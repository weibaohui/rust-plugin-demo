# ============================================================================
# plugkit — Rust 通用插件管理框架
#
# make        → 构建框架 + 前端
# make run    → 构建一切 + 安装到 bin/ + 启动宿主
# ============================================================================

REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
BIN_DIR         = $(REPO_ROOT)bin
PLUGIN_DIR      = $(BIN_DIR)/plugins

# ---------- 主目标 ----------
.PHONY: all build check test clean frontend run install

all: frontend build ## 默认: 构建通用前端 + plugkit 框架

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## cargo build (主框架)
	cargo build

check: ## cargo check
	cargo check

test: ## cargo test
	cargo test

clean: ## cargo clean + 清理 bin/
	rm -rf $(BIN_DIR)
	cargo clean

frontend: ## 构建通用前端 (frontend/dist/)
	@if [ ! -d frontend/node_modules ]; then \
		echo "==> frontend: npm install"; \
		(cd frontend && npm install); \
	fi
	@echo "==> frontend: npm run build"
	@(cd frontend && npm run build)
	# 强制 cargo 重新编译 plugkit (include_dir! 静态嵌入)
	@touch src/host.rs
	@echo "✓ built frontend/dist/ (touch src/host.rs 以触发重编)"

# ---------- 安装到 bin/ ----------
install: frontend ## 构建框架 + 插件 + 安装到 bin/
	@echo "==> 构建 plugkit 框架..."
	@cargo build --release 2>/dev/null || cargo build
	@echo "==> 构建插件..."
	$(MAKE) -C examples/news build 2>/dev/null || true
	@echo "==> 安装到 $(BIN_DIR)..."
	@mkdir -p $(PLUGIN_DIR)
	# 主框架
	@cp "$(REPO_ROOT)target/debug/plugkit" "$(BIN_DIR)/plugkit" 2>/dev/null || true
	@cp "$(REPO_ROOT)target/release/plugkit" "$(BIN_DIR)/plugkit" 2>/dev/null || true
	# 插件 dylib
	@for p in afp_plugin reuters_plugin; do \
		dylib="lib$$p.dylib"; \
		src="$(REPO_ROOT)examples/news/plugins/$$p/target/debug/$$dylib"; \
		[ -f "$$src" ] && cp "$$src" "$(PLUGIN_DIR)/" && echo "  ✓ $$dylib"; \
	done
	@echo "✓ 安装完成:"
	@echo "  $(BIN_DIR)/plugkit"
	@ls $(PLUGIN_DIR)/ 2>/dev/null | sed 's/^/  /'

run: install ## 安装所有组件 + 启动插件后台 — http://localhost:3000
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  plugkit 通用插件管理后台启动中...                       ║"
	@echo "║  后端 API:  http://localhost:3000/api                   ║"
	@echo "║  前端 UI:   http://localhost:3000/                      ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@cargo run

# ---------- debug ----------
list: ## 列出项目结构
	@echo "plugkit 框架 (lib + bin):"
	@echo "  src/                — 框架核心 (lib)"
	@echo "  src/main.rs         — 通用插件宿主入口 (bin)"
	@echo "  frontend/           — 通用插件管理前端"
	@echo "  bin/                — 构建产物输出目录"
	@echo "    plugkit           — 宿主可执行文件"
	@echo "    plugins/          — 插件 dylib"
	@echo ""
	@echo "独立插件示例 (examples/news/plugins/):"
	@echo "  afp_plugin/         — 法新社插件（仅依赖 plugkit）"
	@echo "  reuters_plugin/     — 路透社插件（仅依赖 plugkit）"
	@echo ""
	@echo "构建框架:     make"
	@echo "安装到 bin/:  make install"
	@echo "运行:         make run"