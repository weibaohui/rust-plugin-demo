REPO_ROOT := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

.PHONY: all build clean help

all: build ## 默认: 构建后端 dylib

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## 构建后端 dylib（无 UI）
	@echo "==> 构建后端 dylib"
	@cargo build
	@echo "✓ 构建完成 → target/debug/libhello_plugin.dylib"

clean: ## 清理构建产物
	@cargo clean
	@echo "✓ 清理完成"