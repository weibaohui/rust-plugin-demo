# ============================================================================
# plugkit — Rust 通用插件管理框架
#
# 主框架仅编译 plugkit 库本身。新闻插件示例在 examples/news/ 下，独立构建。
# 本 Makefile 仅保留框架开发常用命令，示例构建请 cd examples/news 参考其 Makefile。
# ============================================================================

# ---------- 基础变量 ----------
REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# ---------- 主目标 ----------
.PHONY: all build check test clean help

all: frontend build ## 默认: 构建前端 + cargo build

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

# ---------- 示例构建 ----------
.PHONY: example-news

example-news: ## 构建新闻插件示例 (cd examples/news && make)
	@echo "==> 构建新闻插件示例 (examples/news)"
	@if [ -f examples/news/Makefile ]; then \
		$(MAKE) -C examples/news; \
	else \
		echo "提示: examples/news/Makefile 不存在，请自行 cd examples/news && cargo build"; \
	fi

# ---------- 运行示例 ----------
.PHONY: run-example-news

run-example-news: example-news ## 构建并运行新闻插件示例
	@echo "==> 启动 news_server 示例..."
	@cd examples/news && cargo run --bin news_server

# ---------- debug ----------
list: ## 列出项目结构
	@echo "plugkit 框架:"
	@echo "  src/          — 框架核心 (lib)"
	@echo "  examples/news/ — 新闻插件宿主示例"
	@echo "    news_api/     — 新闻 API crate (插件侧)"
	@echo "    news_server/  — 新闻宿主 (bin)"
	@echo "    plugins/      — 新闻插件 dylib"
	@echo "    frontend/     — 宿主前端 (Vite + qiankun)"
	@echo ""
	@echo "构建: make build"
	@echo "示例: cd examples/news && make"