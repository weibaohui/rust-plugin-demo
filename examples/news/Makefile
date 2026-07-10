# ============================================================================
# news example — 新闻插件演示示例的构建与运行
# ============================================================================

REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# 自动识别目标三元组
UNAME_S        := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    DYLIB_EXT    := dylib
    DYLIB_PREFIX := lib
else ifeq ($(UNAME_S),Linux)
    DYLIB_EXT    := so
    DYLIB_PREFIX := lib
else
    DYLIB_EXT    := dll
    DYLIB_PREFIX :=
endif

PLUGINS        := afp_plugin reuters_plugin

.PHONY: all build build-plugins run clean help

all: build ## 默认: 构建宿主 + 所有插件

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# ---------- 构建宿主 ----------
build: ## 构建宿主 + 所有插件
	@echo "==> 构建 news_server (宿主)"
	@cd news_server && cargo build
	@echo "==> 构建所有插件"
	@for p in $(PLUGINS); do \
		echo "  -> plugins/$$p"; \
		cd plugins/$$p && cargo build && cd $(REPO_ROOT) || exit $$?; \
	done
	@echo "✓ 全部构建完成"

# ---------- 单插件构建 ----------
build-plugins: ## 仅构建插件(不构建宿主)
	@for p in $(PLUGINS); do \
		echo "  -> plugins/$$p"; \
		cd plugins/$$p && cargo build && cd $(REPO_ROOT) || exit $$?; \
	done
	@echo "✓ 插件构建完成"

# ---------- 运行 ----------
run: ## 构建并运行宿主 (需先 make build 构建插件)
	@echo "==> 启动 news_server..."
	@cd news_server && cargo run

# ---------- 清理 ----------
clean: ## 清理所有构建产物
	cd news_server && cargo clean
	@for p in $(PLUGINS); do \
		cd plugins/$$p && cargo clean && cd $(REPO_ROOT) || exit $$?; \
	done
	@echo "✓ 清理完成"