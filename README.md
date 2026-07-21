# Crate plugkit

**Rust 通用插件管理框架** — 基于动态库加载，支持生命周期状态机、cron 调度、UI 嵌入与菜单聚合，可开箱即用地构建插件式宿主应用。

![MIT License](https://img.shields.io/badge/license-mit-118811.svg)
![Minimum Rust Version](https://img.shields.io/badge/Min%20Rust-1.70-green.svg)

-----

## 设计理念

`plugkit` 是一个**插件管理框架**，本身不做业务。它提供：

### 核心能力（`plugkit`）

| 模块 | 能力 |
|------|------|
| `plugin` | `Plugin` trait + 完整生命周期状态机（Loaded → Enabled → Running） |
| `manager` | `PluginManager` — 动态库加载/卸载、依赖拓扑排序、状态转换 |
| `database` | `DatabaseExt` + `SqliteDatabase` 实现，向插件安全暴露数据库 |
| `metadata` | `PluginMetadata` — 声明式元信息（名称/版本/依赖/菜单/cron） |
| `config` | `PluginManagerConfiguration` — 从配置文件初始化插件管理器 |
| `error` | 统一错误类型（404/409/500 映射） |
| `auth` | JWT 登录认证、声明式路由权限、`RequestCtx` 上下文 |

### 宿主能力（`plugkit::host`）

| 功能 | 说明 |
|------|------|
| HTTP API | 开箱即用的 axum Router：库扫描/加载/卸载、插件 CRUD、状态机操作 |
| cron 调度 | `start` 时自动注册 `tokio::spawn` 定时任务，`stop` 时注销 |
| UI 托盘 | 从编译期嵌入的 `include_dir!` 内存服务插件 UI |
| CORS | 默认 `permissive`，方便开发调试 |

### 依赖

`plugkit` 核心：

- `libloading` — 动态库加载
- `rusqlite` — SQLite 数据库
- `search_path` — 库路径解析
- `parking_lot` — 高效同步原语

`plugkit::host` 额外：

- `axum` + `tokio` — HTTP 服务器
- `include_dir` — 编译期嵌入 UI 目录
- `walkdir` — 扫描 dylib 文件
- `mime_guess` — MIME 类型推断
- `chrono` — 时间格式化

-----

## 快速开始

```rust
use plugkit::host::{host_router, HostApp, serve_frontend_handler};
use plugkit::plugin::Plugin;
use std::sync::Arc;

#[derive(Debug)] struct MyPlugin { id: String }
impl Plugin for MyPlugin {
    fn plugin_id(&self) -> &String { &self.id }
}

#[tokio::main]
async fn main() {
    let app = HostApp::new();
    let state = Arc::new(std::sync::RwLock::new(app));
    let router = host_router()
        .fallback(serve_frontend_handler)
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
```

## 认证

启用 JWT 认证：

```rust
use plugkit::database::SqliteDatabase;
use std::sync::Arc;

let db = Arc::new(SqliteDatabase::open("plugkit.db").unwrap());
let app = HostApp::new()
    .with_database(db)
    .with_auth();  // 启用认证
```

- 默认管理员：`admin` / 环境变量 `PLUGKIT_ADMIN_PASSWORD` 或随机生成
- 登录端点：`POST /auth/login`
- 插件声明式权限：`AuthRequirement::Public` / `Authenticated` / `Permission("xxx")`

详见 [`docs/auth.md`](docs/auth.md)。

## 完整示例

参见 [`examples/news/`](examples/news/) — 一个基于 `plugkit` 构建的新闻机构插件宿主：

- `news_api` — 新闻插件 API crate（定义 `NewsAgencyPlugin`）
- `news_server` — 新闻宿主（二进制），仅含发布业务，管理能力全部来自 `plugkit::host`
- `plugins/afp_plugin` — 法新社插件（dylib，编译期嵌入 UI）
- `plugins/reuters_plugin` — 路透社插件（dylib）

```bash
cd examples/news && make
```

-----

## 版本

**0.2.0** — 从 `dygpi` 升级为 `plugkit`，合并通用宿主能力。

-----

## License

MIT