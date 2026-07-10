# ============================================================================
# 独立插件构建示例 — afp_plugin / reuters_plugin
#
# 这些插件是真正独立的 dylib，只依赖 plugkit 框架，不依赖任何宿主。
# 任何宿主（如 `make run` 启动的 standalone）都可以在运行时加载它们。
# ============================================================================

REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
PLUGINS        := afp_plugin reuters_plugin

.PHONY: all build clean help

all: build ## 默认: 构建所有插件

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## 构建所有插件
	@echo "==> 构建插件"
	@for p in $(PLUGINS); do \
		echo "  -> plugins/$$p"; \
		cd plugins/$$p && cargo build && cd $(REPO_ROOT) || exit $$?; \
	done
	@echo "✓ 插件构建完成"

clean: ## 清理所有构建产物
	@for p in $(PLUGINS); do \
		cd plugins/$$p && cargo clean && cd $(REPO_ROOT) || exit $$?; \
	done
	@echo "✓ 清理完成"