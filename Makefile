# ============================================================================
# rust-plugin-demo 顶层 Makefile
#
# 主框架 news_server 与各插件(plugins/*/Makefile)解耦:
#   - 每个插件可独立 `cd plugins/<name> && make <target>`
#   - 顶层 make 自动扫描 plugins/*/Makefile,把它们串起来
#
# 三类产物位置 (按"分别"程度递增):
#   1. workspace 级: target/release/lib<name>.<ext>     [cargo 默认]
#   2. 插件自包含: plugins/<name>/release/             [make release]
#                     └── lib<name>.<ext>           (UI 已编译期嵌入)
#   3. 集中发布: bin/plugin/<name>/                     [make install]
#                     └── lib<name>.<ext>           (UI 已编译期嵌入)
#
# 分发产物只是单个 dylib —— UI 通过 include_dir! 在编译期打进二进制内部,
# 运行时 host 从内存服务。不需要也不应额外的 ui/dist 文件件。
#
# 嵌入构建:
#   - 先 npm run build 出 ui/dist/
#   - 再 cargo build --release,产物 dylib 通过 include_dir! 把 ui/dist/ 编译期打包
#   - 校验:build 后用 strings+grep 在 dylib 中搜索 UI 特征(<!doctype html/<html),
#          不通过则报错,确保嵌入成功
#
# 启动: bin/news_server → 自动扫描 bin/plugin/<name>/*.dylib 并 dlopen
# ============================================================================

# ---------- 基础变量 ----------
REPO_ROOT      := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
BIN_DIR        := $(REPO_ROOT)/bin
SERVER_BIN     := $(BIN_DIR)/news_server
SERVER_CRATE   := news_server

# 自动扫描 plugins/*/Makefile,新增插件零修改
PLUGIN_MFILES  := $(wildcard $(REPO_ROOT)/plugins/*/Makefile)
PLUGINS        := $(patsubst $(REPO_ROOT)/plugins/%/Makefile,%,$(PLUGIN_MFILES))

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

# ---------- 主目标 ----------
.PHONY: all install release release-embedded plugins-embedded build server plugins \
        verify-embed run clean distclean list-plugins help

# 默认:嵌入式构建 + 自包含 release 包 + 安装到 ./bin/
all: install ## 默认: 每个插件生成自包含 release + 复制到 ./bin/

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
	/^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# ---------- 主框架 ----------
build: server ## 仅 cargo build --release 主框架

ui-frontend: ## 构建宿主前端 (产出 frontend/dist/,会被 include_dir! 嵌入 server)
	@if [ ! -d frontend ]; then \
	    echo "✗ frontend/ 不存在"; exit 1; \
	fi
	@if [ ! -d frontend/node_modules ]; then \
	    echo "==> frontend: npm install"; \
	    (cd frontend && npm install); \
	fi
	@echo "==> frontend: npm run build"
	@(cd frontend && npm run build)
	# 触碰 news_server/src/main.rs 的 mtime, 强制 cargo 重新编译 news_server
	# 否则 include_dir! 拿到的可能是上次嵌入的旧版 frontend/dist/
	@touch news_server/src/main.rs
	@echo "✓ built frontend/dist/ (touch news_server/src/main.rs 以触发重编)"

server: ui-frontend ## 构建主框架 release 产物 (自动跑 ui-frontend 先于 cargo build)
	cargo build --release -p $(SERVER_CRATE)
	@echo "✓ built $(REPO_ROOT)/target/release/$(SERVER_CRATE)"

# ---------- 插件嵌入构建 (workspace 级) ----------
# 串行调用每个插件 Makefile 的 `embed`(内部 ui 先、cargo 后,顺序保证嵌入)
# 接着用 verify-embed 校验每个 dylib 确实包含了 ui/dist 内容。
plugins-embedded: ## 嵌入构建所有插件 (产物在 target/release/,可被所有插件共享)
	@if [ -z "$(PLUGINS)" ]; then \
	    echo "✗ 未发现任何插件 (plugins/*/Makefile)"; exit 1; \
	fi
	@echo "==> 发现 $(words $(PLUGINS)) 个插件: $(PLUGINS)"
	@for p in $(PLUGINS); do \
	    echo ""; \
	    echo "==> [$$p] 嵌入式 release 构建"; \
	    $(MAKE) -C plugins/$$p embed || exit $$?; \
	done
	@echo ""
	@echo "==> 校验 dylib 嵌入"
	@$(MAKE) verify-embed

# release-embedded 暴露为单独 target,供只想构建不安装的场景
release-embedded: plugins-embedded ## 仅嵌入式构建 release,产物在 target/release/

# ---------- 校验 ----------
verify-embed: ## 校验 release dylib 是否真的把 ui/dist 嵌进去了
	@fail=0; \
	for p in $(PLUGINS); do \
	    dylib="$(REPO_ROOT)/target/release/$(DYLIB_PREFIX)$$p.$(DYLIB_EXT)"; \
	    if [ ! -f "$$dylib" ]; then \
	        echo "✗ [$$p] 缺少 dylib: $$dylib"; fail=1; continue; \
	    fi; \
	    if strings -a "$$dylib" 2>/dev/null | grep -qiE '<!doctype html|<html'; then \
	        echo "✓ [$$p] UI 已嵌入 -> $$dylib"; \
	    else \
	        echo "✗ [$$p] UI 未嵌入 (在 dylib 中找不到 <!doctype html / <html 特征): $$dylib"; \
	        echo "    排查:确认 plugins/$$p/ui/dist/index.html 存在且先于 cargo build 生成"; \
	        fail=1; \
	    fi; \
	done; \
	if [ $$fail -ne 0 ]; then exit 1; fi

# ---------- 每个插件分别生成 release 包 ----------
# 逐个调用 `cd plugins/<n> && make release`,每个插件独立产出
# plugins/<n>/release/{lib<n>.<ext>, ui/dist/}。
# 这是"分别生成"的入口 —— 每个插件的产物互不耦合,各自可直接拷贝分发。
release: ## 每个插件分别生成自包含 release 包 (plugins/<name>/release/)
	@if [ -z "$(PLUGINS)" ]; then \
	    echo "✗ 未发现任何插件 (plugins/*/Makefile)"; exit 1; \
	fi
	@echo "==> 发现 $(words $(PLUGINS)) 个插件: $(PLUGINS)"
	@fail=0; \
	for p in $(PLUGINS); do \
	    echo ""; \
	    echo "==> [$$p] 调用 plugins/$$p/Makefile 生成 release"; \
	    if ! $(MAKE) -C plugins/$$p release; then fail=1; break; fi; \
	done; \
	echo ""; \
	echo "============================================================"; \
	echo " 每个插件的自包含 release 包:"; \
	echo "============================================================"; \
	for p in $(PLUGINS); do \
	    d="plugins/$$p/release/$(DYLIB_PREFIX)$$p.$(DYLIB_EXT)"; \
	    if [ -f "$$d" ]; then \
	        sz=$$(stat -f%z "$$d" 2>/dev/null || stat -c%s "$$d" 2>/dev/null); \
	        echo "  ✓ $$d  ($$sz bytes)"; \
	    else \
	        echo "  ✗ $$d  未生成"; fail=1; \
	    fi; \
	done; \
	echo "============================================================"; \
	if [ $$fail -ne 0 ]; then exit $$fail; fi

# ---------- 安装 (复用 release 产物,仅复制到 ./bin/plugin/<name>/) ----------
# `release` 阶段已经出了每个插件的自包含包;这一步只负责把它们复制到集中发布目录。
# 为了避免重复 cargo/npm 构建,plugins/<n>/Makefile 的 install 应复用现有 target/release/<dylib>。
plugins: ## 复用各插件 release 产物,集中安装到 ./bin/plugin/<name>/
	@if [ -z "$(PLUGINS)" ]; then \
	    echo "✗ 未发现任何插件 (plugins/*/Makefile)"; exit 1; \
	fi
	@for p in $(PLUGINS); do \
	    echo ""; \
	    echo "==> [$$p] 集中安装到 ./bin/plugin/$$p/"; \
	    $(MAKE) -C plugins/$$p install-to-bin || exit $$?; \
	done
	@echo ""
	@echo "==> 校验 dylib 嵌入"
	@$(MAKE) verify-embed

install: server release plugins ## 主框架 + 每个插件 release + 集中安装到 ./bin/
	@mkdir -p $(BIN_DIR)
	@cp -f $(REPO_ROOT)/target/release/$(SERVER_CRATE) $(SERVER_BIN)
	@echo
	@echo "✓ installed server -> $(SERVER_BIN)"
	@echo
	@echo "============================================================"
	@echo " Build complete. Layout (发布物只是单个 dylib,UI 已嵌入):"
	@echo "============================================================"
	@find $(BIN_DIR) -maxdepth 4 -type f -print | sort | sed 's|^$(BIN_DIR)|  bin|'
	@echo
	@echo " 各插件自包含 release 包:"
	@for p in $(PLUGINS); do \
	    echo "  plugins/$$p/release/"; \
	    find plugins/$$p/release -maxdepth 2 -type f | sed 's|^|    |'; \
	done
	@echo "============================================================"
	@echo " Run:    bin/news_server"
	@echo " Frontend dev (separate): cd frontend && npm run dev"
	@echo "============================================================"

# ---------- 单插件快速入口 ----------
# 单独构建某一个插件:
#   make plugin/afp_plugin             => 等价于 cd plugins/afp_plugin && make release
#   make plugin/afp_plugin/embed       => 等价于 cd plugins/afp_plugin && make embed
#   make plugin/afp_plugin/install     => 等价于 cd plugins/afp_plugin && make install
#
# 实现:静态模式规则 + 双格式声明(短形式 /<name>,长形式 /<name>/<target>)。
# 使用 .PHONY: $(addprefix plugin/,$(PLUGINS))  让两个名字都能被 make 识别。
.SECONDEXPANSION:
.PHONY: $(addprefix plugin/,$(PLUGINS)) $(addsuffix /%,$(addprefix plugin/,$(PLUGINS)))
$(addprefix plugin/,$(PLUGINS)): ## 单插件构建:make plugin/<name> (默认 release)
	@name=$$(echo "$@" | awk -F'/' '{print $$2}'); \
	 echo "==> [$$name] 单插件构建 target=release"; \
	 $(MAKE) -C plugins/$$name release

$(addsuffix /%,$(addprefix plugin/,$(PLUGINS))): ## 单插件子 target:make plugin/<name>/<target>
	@name=$$(echo "$@" | awk -F'/' '{print $$2}'); \
	 subtarget=$$(echo "$@" | awk -F'/' '{print $$3}'); \
	 echo "==> [$$name] 单插件构建 target=$$subtarget"; \
	 $(MAKE) -C plugins/$$name $$subtarget

# ---------- 运行 ----------
run: install ## 构建并启动主框架 (Ctrl+C 终止)
	$(SERVER_BIN)

# ---------- 清理 ----------
clean: ## cargo clean (整个 workspace)
	cargo clean

distclean: clean ## 清理 ./bin/ 与所有 plugins/*/release/
	rm -rf $(BIN_DIR)
	@for p in $(PLUGINS); do rm -rf plugins/$$p/release; done

# debug:列出自动发现的插件
list-plugins: ## 打印自动发现的插件列表
	@echo "Plugins (扫描自 plugins/*/Makefile):"
	@for p in $(PLUGINS); do echo "  - $$p"; done