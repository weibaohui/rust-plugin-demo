# plugkit Agent 指南

> 给 AI Agent（Claude Code / Cursor / Codex / Continue）的项目速查手册。
> 涵盖：插件开发流程、agent-browser E2E 验证、项目约定与陷阱。

---

## 0. 项目一句话

`plugkit` 是 Rust 通用插件管理框架。插件是 `crate-type = ["dylib"]` 的动态库，宿主通过 `libloading` 在运行时加载，插件通过实现 `Plugin` trait 声明生命周期、数据库、UI、路由、定时任务。

- 框架核心：`src/`（plugin / manager / metadata / database / host / event_bus）
- 宿主二进制：`bin/plugkit`（已构建产物路径）
- 插件样例：`examples/hello_plugin`（最小）、`examples/data_plugin`（完整：UI + DB + cron + routes）、`examples/news/plugins/{afp,reuters}_plugin`（业务型）

---

## 1. 插件开发全流程

### 1.1 五分钟插件（最小骨架 hello_plugin）

参考 `examples/hello_plugin/src/lib.rs`。

**Cargo.toml** 必须三件套：

```toml
[package]
name = "hello_plugin"
version = "0.1.0"
edition = "2021"

[workspace]                        # ⚠️ 必须有，让插件独立编译，不污染根 workspace

[lib]
crate-type = ["dylib"]             # ⚠️ 必须，生成 .dylib 而非 rlib

[dependencies]
plugkit = { version = "0.2", path = "../.." }   # 指向根 Cargo.toml
```

**lib.rs 骨架**：

```rust
use plugkit::database::DatabaseExt;
use plugkit::metadata::PluginMetadata;
use plugkit::plugin::{Plugin, PluginRegistrar};
use std::sync::Arc;

#[derive(Debug)]
pub struct HelloPlugin { id: String }

impl Plugin for HelloPlugin {
    fn plugin_id(&self) -> &String { &self.id }
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(&self.id, "Hello Plugin", env!("CARGO_PKG_VERSION"))
            .with_icon("👋")
            .with_description("...")
            .with_author("...")
    }
    // 钩子全部默认 no-op，按需覆盖
}

// FFI 注册入口（符号名固定为 register_plugins）
#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar) {
    registrar.register(Arc::new(HelloPlugin::new(PLUGIN_ID)));
}

const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"), "::", module_path!(), "::HelloPlugin"
);
```

### 1.2 完整能力插件（data_plugin 风格）

参考 `examples/data_plugin/src/` 目录结构：

```
src/
├── lib.rs        # FFI 注册入口，初始化运行时，嵌入 UI
├── plugin.rs     # #[derive(Debug)] struct + 构造器 + with_ui_dist
├── trait_impl.rs # impl Plugin for XxxPlugin { 生命周期 + UI + routes }
├── metadata.rs   # 元数据（名称/图标/菜单/cron/tables_owned）
├── routes.rs     # HTTP 路由声明
├── handler/      # 每个端点一个 pub fn
├── service.rs    # 业务逻辑（调用 SeaORM / sqlx）
├── model.rs      # 数据结构（无业务逻辑）
└── db.rs         # 独立 runtime + SeaORM 连接管理
```

**关键约定**：

| 约定 | 说明 |
|------|------|
| 模块拆分 | 钩子、路由、handler、service、model、metadata、db 各自分文件 |
| `PLUGIN_ID` | 必须是 `"<crate>::<module>::<StructName>"` 三段式 |
| `lib.rs` 只做注册 | 不放业务代码；只调 `crate::db::init_runtime()` 和 `register_plugins` |
| `metadata.rs` 作为配置中心 | 名称/图标/菜单/cron 集中声明，路由声明独立在 `routes.rs` |
| `with_tables_owned` | 把本插件拥有的表名声明给框架，卸载时按表清理 |

### 1.3 状态机：四个钩子的语义

```
Loaded ──enable──▶ Enabled ──start──▶ Running ──stop──▶ Enabled
   │                  │                  │
   └──disable─────────┘                  │
   │                                     │
   └──unload──▶ (库卸载)                 │
                                         │
   cron_specs 声明定时任务，start 后由宿主按 interval_secs 调度
```

| 钩子 | 何时调用 | 典型用途 |
|------|---------|---------|
| `on_load` | dylib 注册后 | 打印日志 |
| `on_install` | 首次 load | 建表（`CREATE TABLE IF NOT EXISTS`），幂等 |
| `on_uninstall(keep_data)` | 卸载 | `keep_data=false` 时 drop 表 |
| `on_enable` | `Loaded → Enabled` | 菜单可见 |
| `on_start` | `Enabled → Running` | 启动后台任务，cron 开始调度 |
| `on_stop` | `Running → Enabled` | 停止后台任务 |
| `on_cron(name)` | 定时触发 | 执行定时任务 |
| `on_event(event)` | 事件总线广播 | 跨插件通信 |

### 1.4 嵌入 UI（React/Vue/任意）

**Rust 侧**（lib.rs）：

```rust
pub static UI_DIST: include_dir::Dir<'static> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/ui/dist");
```

**plugin.rs** 构造器链：

```rust
pub fn with_ui_dist(mut self, base_dir: &str, dist: &'static Dir<'static>) -> Self {
    self.ui_base_dir = Some(base_dir.to_string());
    self.ui_dist = Some(dist);
    self
}
```

**trait_impl.rs**：

```rust
fn ui_base_dir(&self) -> Option<&str> { self.ui_base_dir.as_deref() }
fn has_ui(&self) -> bool { self.ui_dist.is_some() }
fn ui_dist(&self) -> Option<&'static Dir<'static>> { self.ui_dist }
```

**注册时绑定**（lib.rs 的 `register_plugins`）：

```rust
registrar.register(Arc::new(
    plugin::DataPlugin::new(PLUGIN_ID)
        .with_ui_dist("data_plugin/ui", &UI_DIST),  // base_dir 与 include_dir 路径一致
));
```

**前端侧约定**（qiankun 子应用）：

```
ui/
├── package.json         # vite + qiankun + antd/react 等
├── src/
│   ├── Panel.tsx        # 主组件
│   └── main.tsx         # 导出 bootstrap/mount/update/unmount
├── index.html
└── dist/                # 编译产物，include_dir! 嵌入这里
```

**main.tsx 必须导出 qiankun 生命周期**（参考 `examples/data_plugin/ui/src/main.tsx`）：

```typescript
export async function bootstrap() { /* no-op */ }
export async function mount(props) { render(props) }
export async function update(props) { render(props) }
export async function unmount() { destroy() }

// 手动注入 vitemount 等（vite-plugin-qiankun 1.0.15 + vite 6 兼容）
const QIANKUN_APP_NAME = 'my-plugin';
window.moudleQiankunAppLifeCycles = window.moudleQiankunAppLifeCycles ?? {};
window.moudleQiankunAppLifeCycles[QIANKUN_APP_NAME] = { bootstrap, mount, update, unmount };
```

### 1.5 添加 HTTP 路由

**routes/mod.rs**：

```rust
use http::Method;
use plugkit::plugin::{PluginRoute, PluginRouteHandler};

pub fn routes() -> Vec<PluginRoute> {
    vec![
        PluginRoute { method: Method::GET,  path: "/items".into(),     handler: handler::handle_list },
        PluginRoute { method: Method::POST, path: "/items".into(),     handler: handler::handle_create },
        PluginRoute { method: Method::PUT,  path: "/items".into(),     handler: handler::handle_update },
        PluginRoute { method: Method::DELETE, path: "/items".into(),   handler: handler::handle_delete },
    ]
}
```

**前端调用**：`fetch('/plugin-api/<plugin-id>/items')`

### 1.6 数据库（直接用宿主 db）

`Plugin::on_install(db: &dyn DatabaseExt, ...)` 中：

```rust
db.validate_table_name("my_table")?;                    // ⚠️ 必须先校验，防注入
db.execute("CREATE TABLE IF NOT EXISTS my_table (...)")?;
```

**Plugin trait 之外**，如果要在 handler/cron 里访问数据库，需要自己持有连接（参考 data_plugin 的 `db.rs` 用独立 runtime + SeaORM）。原因是 `Plugin` trait 钩子之外的路径没有 `&dyn DatabaseExt`。

### 1.7 元数据与菜单

```rust
PluginMetadata::new(PLUGIN_ID, "Display Name", env!("CARGO_PKG_VERSION"))
    .with_icon("🗄️")
    .with_description("...")
    .with_author("...")
    .with_license("MIT")
    .with_tables_owned(vec!["data_items".into()])
    .with_menus(vec![PluginMenu {
        key: "my_panel".into(),
        title: "我的面板".into(),
        icon: Some("🗄️".into()),
        route: Some(format!("/plugin/{}", PLUGIN_ID)),
        order: 200,
        children: vec![],
    }])
```

---

## 2. 构建与部署

### 2.1 根目录 Makefile 速查

```bash
make                # 构建框架 + 前端
make frontend       # 构建宿主前端（frontend/dist → src/host.rs 嵌入）
make plugins        # 构建所有插件
make install        # 构建一切并安装到 bin/
make run            # 构建 + 安装 + 启动 bin/plugkit（端口 3000）
make stop           # 杀掉占用 3000 端口的进程（kill → sleep 1 → kill -9）
```

### 2.2 单个插件构建（关键路径）

```bash
cd examples/data_plugin
make                # = build-ui + cargo build
make build-ui       # 仅前端
make clean          # 清理 target/ 和 ui/dist/
```

构建产物：
- 后端：`target/debug/lib<name>.dylib`
- 前端：`ui/dist/index.html` + `ui/dist/assets/`

### 2.3 让宿主识别新插件

宿主通过 `bin/plugins/*.dylib` 加载插件（`make install` 步骤会自动复制）。手动复制：

```bash
cp examples/data_plugin/target/debug/libdata_plugin.dylib bin/plugins/
make stop && ./bin/plugkit
```

**重要**：宿主通过 `include_dir!` 把插件 `ui/dist` 嵌入 dylib，所以**前端变更后必须重跑 `cargo build`**（仅 vite build 不够）。

### 2.4 重启宿主的标准流程

```bash
make stop && ./bin/plugkit
```

或：

```bash
PID=$(lsof -ti :3000) && kill $PID && sleep 1 && ./bin/plugkit &
```

---

## 3. 核心 API（宿主 HTTP）

`host_router()` 默认提供的端点：

| Method | Path | 用途 |
|--------|------|------|
| GET    | `/api/libraries` | 扫描可用插件库 |
| POST   | `/api/libraries/:name/load` | 加载插件库 |
| POST   | `/api/plugins/install` | 上传 .plugin 文件安装（最大 50MB） |
| GET    | `/api/plugins` | 列出所有已加载插件 |
| GET    | `/api/plugins/:id` | 获取单个插件信息 |
| DELETE | `/api/plugins/:id` | 卸载插件 |
| POST   | `/api/plugins/:id/enable` | 启用（Loaded → Enabled） |
| POST   | `/api/plugins/:id/disable` | 禁用 |
| POST   | `/api/plugins/:id/start` | 启动（注册 cron） |
| POST   | `/api/plugins/:id/stop` | 停止 |
| GET    | `/api/plugins/:id/cron` | 列出定时任务 |
| POST   | `/api/plugins/:id/cron/run` | 手动触发 cron |
| GET    | `/plugin-files/*path` | 插件 UI 静态文件 |
| ANY    | `/plugin-api/:plugin_id/*route` | 插件自定义路由 |

**注意**：UI 嵌入依赖 `p.status === 'Enabled' || p.status === 'Running'`（见 `frontend/src/micro.ts:36`）。如果新插件页面空着，先 POST `/api/plugins/:id/enable`。

---

## 4. agent-browser E2E 验证

### 4.1 为什么用 agent-browser 而不是 playwright-cli

| 维度 | playwright-cli | agent-browser |
|------|----------------|---------------|
| 底层 | Node + Playwright | 原生 Rust + CDP |
| 启动 | ~3s | ~1.4s |
| snapshot 引用 | `e1` | `@e1`（更明确） |
| 语义定位 | 仅 ref | `find role/text/label/testid/nth/first` |
| 标注截图 | 无 | `screenshot --annotate`（杀手特性） |
| 网络 mock | 基础 | `network route "**/api" --body='...'` |
| MCP | 无 | `agent-browser mcp` 内置 |
| 多模态友好度 | 截图 + 文字不对齐 | 标注截图与 snapshot 编号一一对应 |

### 4.2 核心循环

```bash
agent-browser open <url>          # 启动浏览器（持久化，复用会话）
agent-browser snapshot -i         # 获取交互元素（带 @eN 引用）
agent-browser click @e3           # 用引用点击
agent-browser snapshot -i         # 页面变化后必须重新 snapshot（ref 会失效）
```

### 4.3 常用命令速查

```bash
# 导航
agent-browser open <url>
agent-browser goto <url>
agent-browser reload
agent-browser close
agent-browser tab                  # 列出所有标签

# 读取
agent-browser snapshot -i           # 交互元素（首选）
agent-browser snapshot -i -c        # 紧凑（去掉空结构节点）
agent-browser snapshot -s "#main"   # 限定 CSS 选择器范围
agent-browser read                  # 渲染 DOM 全文
agent-browser get text @e5          # 元素可见文本
agent-browser get attr @e5 href     # 任意属性
agent-browser get url
agent-browser get title

# 交互
agent-browser click @e3
agent-browser fill @e4 "hello"      # 清空再输入
agent-browser type @e4 "world"      # 追加
agent-browser press Enter
agent-browser select @e5 "option"
agent-browser check @e6
agent-browser scroll down 500

# 语义定位（无 snapshot 也可用）
agent-browser find role button click --name "新增"
agent-browser find text "保存" click
agent-browser find label "Email" fill "user@test.com"
agent-browser find placeholder "搜索" type "query"
agent-browser find first ".card" click

# 等待（避免裸 wait 2000）
agent-browser wait @e1                       # 等元素出现
agent-browser wait --text "Success"          # 等文本
agent-browser wait --url "**/dashboard"      # 等 URL 变化（glob）
agent-browser wait --load networkidle        # 等网络空闲（SPA 导航）
agent-browser wait --fn "window.appReady"    # 等 JS 条件

# 截图
agent-browser screenshot page.png
agent-browser screenshot --full full.png
agent-browser screenshot --annotate map.png # 数字标签 + 图例（多模态神器）

# JS 执行
agent-browser eval "document.title"
agent-browser eval --stdin <<'EOF'           # heredoc 避免引号问题
const rows = document.querySelectorAll('table tbody tr');
JSON.stringify(Array.from(rows).length);
EOF

# 网络
agent-browser network requests              # 查看已发请求
agent-browser network route "**/api/x" --body='{"mock":true}'  # mock
agent-browser network har start              # 录制 HAR
agent-browser network har stop /tmp/trace.har
```

### 4.4 并行会话

```bash
agent-browser --session a open https://example.com
agent-browser --session b open https://example.com
agent-browser session list
agent-browser --session a close

# 或
AGENT_BROWSER_SESSION=myapp agent-browser open https://...
```

### 4.5 状态持久化

```bash
SESSION=$(agent-browser session id --scope worktree --prefix my-app)
agent-browser --session "$SESSION" --restore open https://app.example.com
agent-browser --session "$SESSION" --restore-check-text Dashboard open ...
```

`--restore-save auto`（默认）保证失败的 restore 不会覆盖已知良好状态。

### 4.6 排错

```bash
agent-browser doctor               # 全面诊断（环境/Chrome/守护进程/网络）
agent-browser doctor --fix         # 尝试自动修复
```

**最常见错误**：
- "Ref not found @eN" → 页面已变，重新 `snapshot -i`
- 点击无反应 → 可能有 modal/cookie banner 遮挡，先关闭覆盖元素
- `fill` 不生效 → 试试 `focus @e1` + `keyboard inserttext "..."`
- 等待卡死 → 用 `wait --fn "..."` 替代 `wait 2000`

---

## 5. 项目约定与陷阱

### 5.1 Cargo workspace 隔离

**每个插件 crate 必须有 `[workspace]`**，否则根 workspace 会把插件包含进去，编译时间爆炸且版本冲突。

### 5.2 crate-type = ["dylib"]

**没有这一行，宿主无法加载**（libloading 找的是 .dylib，不是 .rlib）。

### 5.3 include_dir! 路径

```rust
include_dir::include_dir!("$CARGO_MANIFEST_DIR/ui/dist")
```

- 必须是相对 `$CARGO_MANIFEST_DIR` 的路径
- `ui/dist` 必须在 `cargo build` 时存在；前端先 `vite build` 出来
- 变更前端后必须重跑 `cargo build`（仅 vite build 不重新嵌入）

### 5.4 插件 ID 格式

```
<CARGO_PKG_NAME>::<module_path!>::<StructName>
```

例：`"data_plugin::data_plugin::plugin::DataPlugin"` 或简化版 `"data_plugin.DataPlugin"`（data_plugin 用）。

**唯一性约束**：插件 ID 全局唯一，重名会被 framework 拒绝。

### 5.5 数据库操作安全

- 表名必须先 `db.validate_table_name("xxx")?` 校验
- SQL 必须用参数化（`db.query` / `db.execute_with`），禁止字符串拼接
- `on_install` 必须是**幂等**的（用 `CREATE TABLE IF NOT EXISTS`）

### 5.6 UI 在 qiankun 下必须导出生命周期

漏掉 `bootstrap/mount/update/unmount` 导出，agent-browser 看到的是空 `#plugin-mount`。

### 5.7 插件状态 ≠ UI 可见

`p.status === 'Loaded'` 时菜单可见但 UI 不渲染（`frontend/src/micro.ts:36` 过滤了）。需要 UI 嵌入必须 enable。**测试新插件时第一步永远是 enable**。

### 5.8 修改前端后必须三步走

```bash
cd examples/<plugin>/ui && npm run build
cd examples/<plugin> && cargo build
cp target/debug/lib<plugin>.dylib ../../bin/plugins/
make stop && ./bin/plugkit
```

### 5.9 修改宿主前端后两步走

```bash
cd frontend && npm run build      # 编译到 frontend/dist/
touch src/host.rs && cargo build  # 让 include_str! 重新嵌入
cp target/debug/plugkit bin/plugkit
make stop && ./bin/plugkit
```

### 5.10 pre-commit hook

提交时自动跑 `cargo test`（含 14 个单元测试 + 13 个文档测试）和 `cargo fmt --check`。**全部通过才能提交**。

### 5.11 不提交测试产物

`.playwright-cli/`、`frontend/.playwright-cli/`、`<plugin>/ui/.playwright-cli/` 都是浏览器测试产物，**不要 `git add`**。gitignore 还没收录，加：

```gitignore
.playwright-cli/
**/.playwright-cli/
```

### 5.12 调试时查看运行日志

宿主 stdout 包含 `eprintln!` 输出和 qiankun 的 `[qiankun] before-load <app>` 日志。agent-browser 的 `console debug` / `network requests` 配合 `eval` 是排查前端问题三件套。

---

## 6. 典型工作流

### 6.1 新建一个插件

1. 复制 `examples/hello_plugin/` 到 `examples/my_plugin/`
2. 改 `Cargo.toml`（name / dependencies）
3. 改 `src/lib.rs` 的 `PLUGIN_ID` 和 `register_plugins`
4. 实现 `Plugin` trait 钩子
5. （可选）加 `ui/` 目录 + vite 配置
6. `cd examples/my_plugin && make`
7. `cp target/debug/libmy_plugin.dylib ../../bin/plugins/`
8. `make stop && ./bin/plugkit`
9. `curl -X POST http://localhost:3000/api/plugins/<id>/enable`
10. 用 agent-browser 打开 `http://localhost:3000/plugin/<id>` 验证

### 6.2 调试 UI 不显示

```bash
# 1. 检查状态
curl -s http://localhost:3000/api/plugins | python3 -m json.tool | grep -A1 "<id>"

# 2. 启用 + 启动
curl -X POST http://localhost:3000/api/plugins/<id>/enable
curl -X POST http://localhost:3000/api/plugins/<id>/start

# 3. 用 agent-browser 排查
agent-browser open http://localhost:3000/plugin/<id>
agent-browser snapshot -i                    # 看 #plugin-mount 是否有子元素
agent-browser console debug                  # 看是否有 JS 错误
agent-browser network requests               # 看是否加载了 dist/assets/*.js
```

### 6.3 验证 cron 触发

```bash
# 1. 看注册的 cron
curl http://localhost:3000/api/plugins/<id>/cron

# 2. 手动触发（不等 interval）
curl -X POST http://localhost:3000/api/plugins/<id>/cron/run -H "content-type: application/json" -d '{"name":"<cron_name>"}'
```

### 6.4 E2E 验证 data_plugin 完整 CRUD

```bash
agent-browser open http://localhost:3000/plugin/data_plugin.DataPlugin
agent-browser snapshot -i                          # 找到「新增」按钮
agent-browser find role button click --name "新增" # 语义定位点击
agent-browser fill @e14 "测试标题"
agent-browser fill @e15 "测试内容"
agent-browser click @e13                           # 保存
agent-browser snapshot -i                          # 验证新行入库
agent-browser screenshot --annotate /tmp/result.png
```

---

## 7. 重要参考

| 主题 | 位置 |
|------|------|
| Plugin trait 完整定义 | `src/plugin.rs:155` |
| PluginMetadata builder | `src/metadata.rs:145` |
| DatabaseExt 安全 API | `src/database.rs:68` |
| host_router 路由清单 | `src/host.rs:1403` |
| qiankun 注册逻辑 | `frontend/src/micro.ts:36` |
| Makefile 全目标 | `Makefile` |
| 最小插件 | `examples/hello_plugin/src/lib.rs` |
| 完整插件（UI+DB+cron+routes） | `examples/data_plugin/src/` |
| AFP 业务插件 | `examples/news/plugins/afp_plugin/src/lib.rs` |
| agent-browser 核心 skill | `agent-browser skills get core` |