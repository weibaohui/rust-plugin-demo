# ============================================================================
# plugkit — Rust 通用插件管理框架
#
# 主框架: make all        → 构建 plugkit 库 + 通用前端
# 示例:   make run        → 构建/运行新闻插件示例 (examples/news/)
#         单独构建示例: cd examples/news && make
# ============================================================================

# ---------- 基础变量 ----------
REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# ---------- 主目标 ----------
.PHONY: all build check test clean frontend run

all: frontend build ## 默认: 构建通用前端 + plugkit 库

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## cargo build (主框架)
	cargo build

check: ## cargo check
	cargo check

test: ## cargo test
	cargo test

clean: ## cargo clean
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

# ---------- 示例 ----------
run: frontend ## 构建/运行新闻插件示例 (框架 + 示例全量)
	@echo "==> 构建新闻插件示例"
	$(MAKE) -C examples/news build
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║  news_server 启动中...                                   ║"
	@echo "║  后端 API:  http://localhost:3000/api                   ║"
	@echo "║  前端 UI:  http://localhost:3000/                       ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@cd examples/news && cargo run --manifest-path news_server/Cargo.toml

# ---------- debug ----------
list: ## 列出项目结构
	@echo "plugkit 框架:"
	@echo "  src/                — 框架核心 (lib)"
	@echo "  frontend/           — 通用插件管理前端"
	@echo ""
	@echo "示例 (examples/news/):"
	@echo "  news_server/        — 新闻宿主 (lib + bin)"
	@echo "  plugins/            — 新闻插件 dylib"
	@echo "  frontend/           — 新闻宿主前端"
	@echo ""
	@echo "构建框架: make"
	@echo "运行示例: make run"