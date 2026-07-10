# ----------------------------------------------------------------------------
# afp_plugin Makefile
#
# 该插件作为独立 crate 自负责自己的构建:release build 后,把产物复制到
# 仓库根的 bin/plugin/afp_plugin/ 下,供主框架 news_server 运行时扫描加载。
#
# 产物布局:
#   <repo_root>/bin/plugin/afp_plugin/
#   ├── libafp_plugin.dylib        (macOS) / .so (Linux) / .dll (Windows)
#   └── ui/
#       └── dist/                  (前端静态资源,作为 dylib 之外的 fallback)
# ----------------------------------------------------------------------------

# ---------- 基础变量 ----------
PLUGIN_NAME      := afp_plugin
PLUGIN_DIR       := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
REPO_ROOT        := $(abspath $(PLUGIN_DIR)../..)
BIN_PLUGIN_DIR   := $(REPO_ROOT)/bin/plugin/$(PLUGIN_NAME)
# 插件自包含的 release 产物:plugins/<name>/release/
SELF_RELEASE_DIR := $(PLUGIN_DIR)release
UI_SRC_DIR       := $(PLUGIN_DIR)ui
UI_DIST_DIR      := $(UI_SRC_DIR)/dist

# 自动识别平台 dylib 扩展名与前缀
UNAME_S          := $(shell uname -s)
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

DYLIB_NAME       := $(DYLIB_PREFIX)$(PLUGIN_NAME).$(DYLIB_EXT)
SRC_DYLIB        := $(REPO_ROOT)/target/release/$(DYLIB_NAME)
DST_DYLIB        := $(BIN_PLUGIN_DIR)/$(DYLIB_NAME)

# ---------- 主目标 ----------
.PHONY: all build embed release install install-to-bin ui ui-install clean distclean help

all: install ## 默认: 构建并安装 release 产物到 bin/plugin/$(PLUGIN_NAME)/

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
	/^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# ---------- 构建 ----------
build: ## cargo build --release (产物落在 target/release/)
	cargo build --release -p $(PLUGIN_NAME)
	@echo "✓ built $(SRC_DYLIB)"

# ---------- 前端 UI ----------
ui-install: ## 在 ui/ 下安装前端依赖 (npm install)
	cd $(UI_SRC_DIR) && npm install

ui: ui-install ## 构建前端 UI (产出 ui/dist/)
	cd $(UI_SRC_DIR) && npm run build
	@echo "✓ built $(UI_DIST_DIR)"

# ---------- 嵌入式 release 构建 (UI 先、cargo 后) ----------
# `include_dir!` 依赖 ui/dist 必须在 cargo build 前存在,因此顺序重要。
embed: ui build ## 嵌入式 release 构建 (UI 编译期打入 dylib),不复制到 bin/

# ---------- 插件自包含 release 包 ----------
# 产物:plugins/$(PLUGIN_NAME)/release/lib<name>.<ext>
# 该文件是插件独享的可分发二进制 —— UI 通过 include_dir! 编译期嵌入内部,
# 运行时由 host 从内存服务,不需要也不应额外附带 ui/dist/ 文件夹。
release: embed ## 生成插件自包含 release 包 (仅一个 dylib,UI 已嵌入)
	@mkdir -p $(SELF_RELEASE_DIR)
	@cp -f $(SRC_DYLIB) $(SELF_RELEASE_DIR)/$(DYLIB_NAME)
	@echo "✓ [$(PLUGIN_NAME)] self-release ready -> $(SELF_RELEASE_DIR)/$(DYLIB_NAME)"
	@ls -la $(SELF_RELEASE_DIR)

# ---------- 安装到 bin/plugin/<name>/ ----------
# 完整流程:嵌入式构建 + 复制到 bin/plugin/<name>/。独立运行请用此 target。
install: embed ## 嵌入式构建后,仅复制 dylib 到 bin/plugin/$(PLUGIN_NAME)/
	@mkdir -p $(BIN_PLUGIN_DIR)
	@cp -f $(SRC_DYLIB) $(DST_DYLIB)
	@echo "✓ installed $(PLUGIN_NAME) -> $(DST_DYLIB)"

# 复用现成的 target/release/lib<name>.<ext> 仅复制到 bin/plugin/<n>/
# 顶层 make all 在调过 `make release` 后调此 target,避免重复 cargo 构建。
install-to-bin: ## 复用 release 产物,仅复制 dylib 到 bin/plugin/$(PLUGIN_NAME)/ (不重新构建)
	@mkdir -p $(BIN_PLUGIN_DIR)
	@if [ ! -f $(SRC_DYLIB) ]; then \
	    echo "✗ 缺少 $(SRC_DYLIB),请先 make release 或 make embed"; exit 1; \
	fi
	@cp -f $(SRC_DYLIB) $(DST_DYLIB)
	@echo "✓ installed $(PLUGIN_NAME) -> $(DST_DYLIB)"

# ---------- 清理 ----------
clean: ## cargo clean
	cargo clean -p $(PLUGIN_NAME)

distclean: clean ## 同时清理 ui/node_modules 与 ui/dist
	rm -rf $(UI_SRC_DIR)/node_modules $(UI_SRC_DIR)/dist $(UI_SRC_DIR)/*.tsbuildinfo $(UI_SRC_DIR)/.vite
	rm -rf $(BIN_PLUGIN_DIR)