REPO_ROOT := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

.PHONY: all build build-ui clean help

all: build ## 默认: 构建前端 + 后端

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^#^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: build-ui ## 构建前端 + 后端
	@echo "==> 构建后端 dylib"
	@cargo build
	@echo "✓ 构建完成 → target/debug/libdata_plugin.dylib"

build-ui: ## 构建前端 (ui/dist/)
	@echo "==> 构建前端"
	@cd ui && \
		[ -d node_modules ] || npm install; \
		(node node_modules/typescript/lib/tsc.js -b 2>/dev/null || true); \
		node node_modules/vite/bin/vite.js build
	@echo "✓ 前端构建完成 → ui/dist/"

clean: ## 清理构建产物
	@cargo clean
	@rm -rf ui/dist ui/node_modules
	@echo "✓ 清理完成"
