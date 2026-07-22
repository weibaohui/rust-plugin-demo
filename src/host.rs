/*!
通用插件宿主模块。

提供开箱即用的 axum HTTP 路由，用于管理插件的加载/卸载、生命周期状态机
（Loaded → Enabled → Running）、cron 调度以及插件 UI 资源服务。

二开者可直接使用 [`host_router`] 获得完整的管理 API，再补充自己的业务路由。
宿主前端 SPA 需通过 [`serve_frontend_handler`] 注册为 fallback handler。

# 快速开始

```rust,no_run
use plugkit::host::{host_router, HostApp, serve_frontend_handler};
use std::sync::Arc;

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
*/

use crate::auth::ctx::RequestCtx;
use crate::auth::routes::{handle_login, handle_logout, handle_me, handle_revoke_all};
use crate::auth::service::{AuthService, AuthServiceImpl};
use crate::database::DatabaseExt;
use crate::error::ErrorKind;
use crate::manager::{PluginManager, PLATFORM_DYLIB_EXTENSION, PLATFORM_DYLIB_PREFIX};
use crate::metadata::PluginMenu;
use crate::plugin::{Plugin, PluginStatus};

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Method, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{any, delete, get, post},
    Json, Router,
};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tracing::info;

// ------------------------------------------------------------------------------------------------
// HostContext trait（通用版本，不含业务字段）
// ------------------------------------------------------------------------------------------------

/// 宿主上下文 trait。
///
/// 插件可以通过此 trait 从宿主获取信息并调用宿主方法，实现宿主 ↔ 插件的双向通信。
/// 宿主需实现此 trait，在调用插件业务方法时传入 `&dyn HostContext`。
pub trait HostContext: Send + Sync {
    /// 宿主服务器名称。
    fn server_name(&self) -> &str;
    /// 宿主框架版本。
    fn server_version(&self) -> &str;
    /// 向宿主日志记录一条消息。
    fn log_message(&self, msg: &str);
    /// 按级别记录日志。
    fn log(&self, level: &str, msg: &str) {
        let formatted = format!("[{}] {}", level, msg);
        self.log_message(&formatted);
    }
    /// 获取当前服务器时间（格式化的时间字符串）。
    fn server_time(&self) -> String;
    /// 当前已加载的插件数量。
    fn plugin_count(&self) -> usize;
    /// 发布事件到事件总线，其他插件可通过 `on_event` 接收。
    fn emit(&self, topic: &str, payload: serde_json::Value);
    /// 获取宿主数据库操作接口（若可用）。
    fn database(&self) -> Option<Arc<dyn DatabaseExt>> {
        None
    }
}

/// 宿主的默认 `HostContext` 实现。
///
/// 持有 `SharedState` 引用，插件通过 `ctx.emit()` 发布事件时，
/// 此实现会锁定状态、将事件推入总线、并广播给所有已启用/运行中的插件。
#[derive(Clone)]
pub struct HostContextImpl {
    state: SharedState,
    plugin_id: String,
    server_name: String,
    server_version: String,
}

impl HostContextImpl {
    pub fn new(
        state: SharedState,
        plugin_id: &str,
        server_name: &str,
        server_version: &str,
    ) -> Self {
        Self {
            state,
            plugin_id: plugin_id.to_string(),
            server_name: server_name.to_string(),
            server_version: server_version.to_string(),
        }
    }
}

impl HostContext for HostContextImpl {
    fn server_name(&self) -> &str {
        &self.server_name
    }
    fn server_version(&self) -> &str {
        &self.server_version
    }
    fn log_message(&self, msg: &str) {
        tracing::info!("[plugin:{}] {}", self.plugin_id, msg);
    }
    fn server_time(&self) -> String {
        use chrono::Local;
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }
    fn plugin_count(&self) -> usize {
        self.state.read().unwrap().manager.plugins().len()
    }
    fn database(&self) -> Option<Arc<dyn DatabaseExt>> {
        let ctx = self.state.read().unwrap();
        ctx.manager.database().clone()
    }
    fn emit(&self, topic: &str, payload: serde_json::Value) {
        let event = crate::event_bus::Event {
            topic: topic.to_string(),
            payload,
            source: self.plugin_id.clone(),
        };
        let ctx = self.state.write().unwrap();
        // 推入事件总线
        ctx.event_bus.publish(event.clone());
        // 广播给所有已启用/运行中的插件
        let plugins = ctx.manager.plugins();
        let plugin_ids: Vec<String> = plugins.iter().map(|p| p.plugin_id().clone()).collect();
        // 释放读锁，避免死锁
        drop(plugins);
        for id in &plugin_ids {
            if id == &self.plugin_id {
                continue; // 不给自己发
            }
            let status = ctx.manager.plugin_status(id);
            if matches!(
                status,
                Some(PluginStatus::Enabled) | Some(PluginStatus::Running)
            ) {
                if let Some(p) = ctx.manager.get(id) {
                    let _ = p.on_event(&event);
                }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// 宿主应用上下文
// ------------------------------------------------------------------------------------------------

/// 通用插件宿主应用上下文。
///
/// 封装了 [`PluginManager`]、库加载记录、cron 调度状态和插件 UI 嵌入目录。
#[derive(Debug)]
pub struct HostApp {
    /// 插件管理器（核心）。
    pub manager: PluginManager,
    /// 记录每个库路径 → 它贡献的插件 ID 列表。
    pub library_plugins: HashMap<PathBuf, Vec<String>>,
    /// 每个插件的 cron 取消标志（start 时注册，stop 时置 true）。
    pub cron_flags: HashMap<String, Vec<Arc<AtomicBool>>>,
    /// 插件 UI 嵌入目录映射：base_dir（如 `"afp_plugin/ui"`）→ 编译期嵌入的 `Dir`。
    pub plugin_ui_dirs: HashMap<String, &'static Dir<'static>>,
    /// 插件 dylib 搜索目录（宿主配置）。`find_dylib_paths()` 会合并这些目录与默认启发式。
    pub plugin_search_dirs: Vec<PathBuf>,
    /// 插件间事件总线。
    pub event_bus: crate::event_bus::EventBus,
    /// 已激活插件的路由注册表 — enable 时注入，disable 时移除。
    pub active_plugin_routes: HashMap<String, Vec<crate::plugin::PluginRoute>>,
    /// 认证服务（若启用认证）。
    pub auth_service: Option<Arc<dyn AuthService>>,
}

impl Default for HostApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HostApp {
    /// 创建一个新的宿主上下文。
    pub fn new() -> Self {
        Self {
            manager: PluginManager::default(),
            library_plugins: HashMap::new(),
            cron_flags: HashMap::new(),
            plugin_ui_dirs: HashMap::new(),
            plugin_search_dirs: Vec::new(),
            event_bus: crate::event_bus::EventBus::new(),
            active_plugin_routes: HashMap::new(),
            auth_service: None,
        }
    }

    /// 追加插件 dylib 搜索目录（builder 风格，可链式多次调用）。
    ///
    /// `find_dylib_paths()` 会合并这些目录与默认启发式（cwd/target、exe 同目录等）。
    /// 宿主通常指向 `plugins/` 目录或各插件 crate 的 `target/debug`。
    pub fn with_plugin_search_dir(mut self, dir: impl AsRef<std::path::Path>) -> Self {
        self.plugin_search_dirs.push(dir.as_ref().to_path_buf());
        self
    }

    /// 自动扫描 `plugin_search_dirs` 并加载所有 dylib 插件。
    /// 加载失败的插件会打印警告日志，不影响宿主启动。
    pub fn auto_load(mut self) -> Self {
        let dylibs = find_dylib_paths(&self.plugin_search_dirs);
        for path in &dylibs {
            if let Err(e) = self.manager.load_plugins_from(path) {
                eprintln!("  ⚠️  自动加载插件失败: {:?} — {}", path, e);
                continue;
            }
            // 记录此库贡献的插件 ID 并注册 UI
            let plugins = self.manager.plugins();
            let plugin_ids: Vec<String> = plugins.iter().map(|p| p.plugin_id().clone()).collect();
            let _ = self
                .library_plugins
                .insert(path.clone(), plugin_ids.clone());
            for id in &plugin_ids {
                if let Some(p) = self.manager.get(id) {
                    if let (Some(base_dir), Some(dist)) = (p.ui_base_dir(), p.ui_dist()) {
                        self.register_plugin_ui(base_dir, dist);
                    }
                }
            }
        }
        if !dylibs.is_empty() {
            println!("  ✓ 自动加载 {} 个插件库", dylibs.len());
        }

        // 加载完成后恢复插件状态（Enabled/Running）
        self.manager.restore_plugin_statuses();

        self
    }

    /// 设置数据库后端。
    pub fn with_database(mut self, db: Arc<dyn DatabaseExt>) -> Self {
        self.manager = self.manager.with_database(db);
        self
    }

    /// 启用认证服务。
    ///
    /// 初始化数据库表（users/tokens/role_permissions），创建默认管理员（若不存在），
    /// 并设置 `auth_service`。
    pub fn with_auth(mut self) -> Self {
        let db = self.manager.database().clone();
        if let Some(db) = db {
            // 初始化认证相关表
            let _ = init_auth_tables(db.as_ref());
            // 创建默认管理员（密码从环境变量读取，或随机生成打印到控制台）
            let _ = ensure_default_admin(db.as_ref());
            let auth_service = AuthServiceImpl::new(db.clone());
            self.auth_service = Some(Arc::new(auth_service));
        }
        self
    }

    /// 获取认证服务（若已启用）。
    pub fn auth_service(&self) -> Option<&Arc<dyn AuthService>> {
        self.auth_service.as_ref()
    }

    /// 注册插件 UI 嵌入目录。
    ///
    /// 插件在编译期通过 `include_dir!` 把 `ui/dist` 嵌入 dylib，宿主在这里注册，
    /// 以便 `serve_plugin_ui_handler` 可以从内存服务前端文件。
    pub fn register_plugin_ui(&mut self, base_dir: &str, dist: &'static Dir<'static>) {
        let _ = self.plugin_ui_dirs.insert(base_dir.to_string(), dist);
    }

    /// 启动插件热重载。
    ///
    /// 监听 `plugin_search_dirs` 下所有 dylib 文件的变化，当文件被修改/创建时
    /// 自动重新加载该库对应的所有插件（stop → disable → unload → load）。
    ///
    /// 返回一个 `AbortHandle`，调用方可在宿主关闭时取消监听。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let state = Arc::new(RwLock::new(HostApp::new().with_plugin_search_dir("bin/plugins")));
    /// let _watch = HostApp::start_hot_reload(state.clone());
    /// ```
    pub fn start_hot_reload(state: SharedState) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

            // 获取搜索目录
            let dirs: Vec<PathBuf> = {
                let ctx = state.read().unwrap();
                ctx.plugin_search_dirs.clone()
            };

            if dirs.is_empty() {
                tracing::info!("[hot-reload] plugin_search_dirs 为空，跳过热重载");
                return;
            }

            let (tx, mut rx) = tokio::sync::mpsc::channel(32);

            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = tx.blocking_send(event);
                    }
                },
                Config::default().with_poll_interval(Duration::from_secs(2)),
            )
            .expect("无法创建文件监听器");

            for dir in &dirs {
                if dir.exists() {
                    let _ = watcher.watch(dir, RecursiveMode::Recursive);
                    tracing::info!("[hot-reload] 监听目录: {:?}", dir);
                }
            }

            while let Some(event) = rx.recv().await {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        for path in &event.paths {
                            if !path.is_file() {
                                continue;
                            }
                            let ext = match path.extension() {
                                Some(e) => e,
                                None => continue,
                            };
                            if ext != PLATFORM_DYLIB_EXTENSION {
                                continue;
                            }

                            tracing::info!("[hot-reload] 检测到插件变更: {:?}", path);

                            // 查找该库对应的插件 ID
                            let plugin_ids: Vec<String> = {
                                let ctx = state.read().unwrap();
                                ctx.library_plugins
                                    .iter()
                                    .filter(|(lib_path, _)| {
                                        lib_path.file_name() == path.file_name()
                                    })
                                    .flat_map(|(_, ids)| ids.clone())
                                    .collect()
                            };

                            if plugin_ids.is_empty() {
                                tracing::warn!("[hot-reload] 未找到库 {:?} 对应的插件", path);
                                continue;
                            }

                            // 卸载插件（stop → disable → unload）
                            for id in &plugin_ids {
                                tracing::info!("[hot-reload] 卸载插件: {}", id);
                                if let Err(e) = {
                                    let mut ctx = state.write().unwrap();
                                    // 停止 cron
                                    if let Some(flags) = ctx.cron_flags.remove(id) {
                                        for f in flags {
                                            f.store(true, Ordering::Relaxed);
                                        }
                                    }
                                    // 从 library_plugins 移除
                                    for (_lib_path, ids) in ctx.library_plugins.iter_mut() {
                                        ids.retain(|i| i != id);
                                    }
                                    ctx.library_plugins.retain(|_k, v| !v.is_empty());
                                    // 卸载
                                    ctx.manager.unload_plugin(id, false)
                                } {
                                    tracing::error!("[hot-reload] 卸载插件失败: {} — {}", id, e);
                                }
                            }

                            // 重新加载
                            tracing::info!("[hot-reload] 重新加载库: {:?}", path);
                            if let Err(e) = {
                                let mut ctx = state.write().unwrap();
                                ctx.manager.load_plugins_from(path)
                            } {
                                tracing::error!("[hot-reload] 重新加载失败: {:?} — {}", path, e);
                            }

                            // 重新注册插件 UI
                            let new_plugins: Vec<String> = {
                                let ctx = state.read().unwrap();
                                ctx.library_plugins
                                    .iter()
                                    .filter(|(lib_path, _)| {
                                        lib_path.file_name() == path.file_name()
                                    })
                                    .flat_map(|(_, ids)| ids.clone())
                                    .collect()
                            };
                            for id in &new_plugins {
                                if let Some(p) = {
                                    let ctx = state.read().unwrap();
                                    ctx.manager.get(id)
                                } {
                                    if let (Some(base_dir), Some(dist)) =
                                        (p.ui_base_dir(), p.ui_dist())
                                    {
                                        let mut ctx = state.write().unwrap();
                                        ctx.register_plugin_ui(base_dir, dist);
                                        tracing::info!("[hot-reload] 注册插件 UI: {}", id);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        })
    }
}

/// 共享状态的类型别名。
pub type SharedState = Arc<RwLock<HostApp>>;

// ------------------------------------------------------------------------------------------------
// API 请求/响应类型
// ------------------------------------------------------------------------------------------------

/// 插件库信息。
#[derive(Debug, Serialize)]
pub struct LibraryInfo {
    /// 清理后的库名（去掉 lib 前缀）。
    pub name: String,
    /// 原始文件名。
    pub file_name: String,
    /// 完整路径。
    pub path: String,
    /// 是否已加载。
    pub loaded: bool,
    /// 此库贡献的插件数量。
    pub plugin_count: usize,
}

/// 库扫描结果。
#[derive(Debug, Serialize)]
pub struct LibraryScanResult {
    /// 扫描到的库列表。
    pub libraries: Vec<LibraryInfo>,
}

/// 加载结果。
#[derive(Debug, Serialize)]
pub struct LoadResult {
    /// 新加载的插件列表。
    pub plugins: Vec<PluginInfo>,
}

/// 通用 API 消息响应。
#[derive(Debug, Serialize)]
pub struct ApiMessage {
    /// 消息文本。
    pub message: String,
}

/// 插件定时任务信息。
#[derive(Debug, Serialize)]
pub struct CronInfo {
    /// 任务名称。
    pub name: String,
    /// 间隔秒数。
    pub interval_secs: u64,
    /// 是否正在运行。
    pub running: bool,
}

/// 手动触发 cron 请求。
#[derive(Debug, Deserialize)]
pub struct CronRunRequest {
    /// 要触发的任务名称。
    pub name: String,
}

/// 插件前端信息（通用版本，不含业务字段）。
#[derive(Debug, Serialize, Clone)]
pub struct PluginInfo {
    /// 插件唯一 ID。
    pub id: String,
    /// 展示名称（来自 `metadata().title`）。
    pub name: String,
    /// 版本（来自 `metadata().version`）。
    pub version: String,
    /// 作者（来自 `metadata().author`）。
    pub author: String,
    /// 功能描述（来自 `metadata().description`）。
    pub description: String,
    /// 是否有嵌入的 UI。
    pub has_ui: bool,
    /// 是否声明了定时任务（cron）。
    pub has_cron: bool,
    /// 是否使用了数据库表（`metadata.tables` 非空）。
    pub has_database: bool,
    /// qiankun 子应用入口 URL（`has_ui` 为 false 时为 None）。
    pub ui_entry: Option<String>,
    /// 插件声明的菜单树。
    pub menu: Vec<PluginMenu>,
    /// 插件当前生命周期状态。
    pub status: PluginStatus,
    /// 数据库中已安装的版本（来自 plugkit_plugins 表）。
    /// None 表示从未安装过（全新插件）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    /// 是否有可用升级（已安装版本 < 扫描版本 或 预追踪安装）。
    pub needs_upgrade: bool,
}

// ------------------------------------------------------------------------------------------------
// 工具函数
// ------------------------------------------------------------------------------------------------

/// 把插件转为前端可消费的 [`PluginInfo`]。
///
/// 菜单仅在 `Enabled` / `Running` 状态下对外暴露。
pub fn plugin_to_info(
    p: &dyn Plugin,
    status: PluginStatus,
    installed_version: Option<String>,
) -> PluginInfo {
    let meta = p.metadata();
    let needs_upgrade = installed_version
        .as_ref()
        .map_or(false, |v| v != &meta.version);
    let author = meta.author.clone().unwrap_or_default();
    let description = meta.description.clone().unwrap_or_default();
    // cron 可来自 metadata 声明，也可来自插件的 cron_specs()（动态注册），两者都算
    let has_cron = !meta.crons().is_empty() || !p.cron_specs().is_empty();
    let has_database = !meta.tables().is_empty();
    // 仅在启用/运行状态下暴露菜单
    let menu = if matches!(status, PluginStatus::Enabled | PluginStatus::Running) {
        meta.menus().to_vec()
    } else {
        vec![]
    };
    PluginInfo {
        id: p.plugin_id().clone(),
        name: meta.title,
        version: meta.version,
        author,
        description,
        has_ui: p.has_ui(),
        has_cron,
        has_database,
        ui_entry: p
            .ui_base_dir()
            .map(|d| format!("/plugin-files/{}/dist/index.html", d)),
        menu,
        status,
        installed_version,
        needs_upgrade,
    }
}

/// 把 [`crate::error::Error`] 映射为 HTTP 响应：
/// `PluginNotFound` → 404，`InvalidPluginState` → 409，其他 → 500。
pub fn plugin_err_to_response(
    e: crate::error::Error,
    action: &str,
) -> (StatusCode, Json<ApiMessage>) {
    let code = match e.kind() {
        ErrorKind::PluginNotFound(_) => StatusCode::NOT_FOUND,
        ErrorKind::InvalidPluginState(_) => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        code,
        Json(ApiMessage {
            message: format!("{}失败: {}", action, e),
        }),
    )
}

// ------------------------------------------------------------------------------------------------
// dylib 扫描
// ------------------------------------------------------------------------------------------------

/// 扫描文件系统，发现可用的插件 dylib 库。
///
/// 扫描 `target/debug/`、`target/release/`、`bin/plugin/` 等常见路径，
/// 按 `.dylib` / `.so` / `.dll` 扩展名匹配。
pub fn find_dylib_paths(extra_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut results = Vec::new();

    for dir in extra_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == PLATFORM_DYLIB_EXTENSION {
                        if !results.iter().any(|r: &PathBuf| r == &path) {
                            results.push(path);
                        }
                    }
                }
            }
        }
    }

    results
}

/// 去掉 dylib 文件名中的 `lib` 前缀。
pub fn clean_lib_name(file_stem: &str) -> String {
    file_stem
        .strip_prefix(PLATFORM_DYLIB_PREFIX)
        .unwrap_or(file_stem)
        .to_string()
}

// ------------------------------------------------------------------------------------------------
// API Handlers
// ------------------------------------------------------------------------------------------------

async fn handle_scan_libraries(State(state): State<SharedState>) -> Json<LibraryScanResult> {
    let ctx = state.read().unwrap();
    let mut libs: Vec<LibraryInfo> = Vec::new();

    for path in find_dylib_paths(&ctx.plugin_search_dirs) {
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let name = clean_lib_name(&stem);
        let loaded = ctx.library_plugins.contains_key(&path);
        let plugin_count = ctx.library_plugins.get(&path).map(|v| v.len()).unwrap_or(0);

        libs.push(LibraryInfo {
            name,
            file_name,
            path: path.to_string_lossy().to_string(),
            loaded,
            plugin_count,
        });
    }

    Json(LibraryScanResult { libraries: libs })
}

async fn handle_load_library(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<LoadResult>, (StatusCode, Json<ApiMessage>)> {
    // 查找匹配的库文件
    let path = find_dylib_paths(&state.read().unwrap().plugin_search_dirs)
        .into_iter()
        .find(|p| {
            p.file_stem()
                .map(|s| {
                    let stem = s.to_string_lossy();
                    stem.as_ref() == name.as_str() || clean_lib_name(&stem) == name.as_str()
                })
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiMessage {
                    message: format!("未找到插件库 '{}'", name),
                }),
            )
        })?;

    let mut ctx = state.write().unwrap();
    ctx.manager.load_plugins_from(&path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("加载失败: {:?}", e),
            }),
        )
    })?;

    // 记录此库贡献的插件 ID
    let plugins = ctx.manager.plugins();
    let plugin_ids: Vec<String> = plugins.iter().map(|p| p.plugin_id().clone()).collect();

    let existing = ctx.library_plugins.get(&path).cloned().unwrap_or_default();
    let new_ids: Vec<String> = plugin_ids
        .iter()
        .filter(|id| !existing.contains(id))
        .cloned()
        .collect();

    // 注册新插件的嵌入 UI 目录
    for id in &new_ids {
        if let Some(p) = ctx.manager.get(id) {
            if let (Some(base_dir), Some(dist)) = (p.ui_base_dir(), p.ui_dist()) {
                ctx.register_plugin_ui(base_dir, dist);
            }
        }
    }

    let _ = ctx.library_plugins.insert(path, plugin_ids);

    let new_plugins: Vec<PluginInfo> = new_ids
        .iter()
        .filter_map(|id| {
            ctx.manager.get(id).map(|p| {
                plugin_to_info(
                    &*p,
                    ctx.manager
                        .plugin_status(id)
                        .unwrap_or(PluginStatus::Loaded),
                    ctx.manager.installed_version(id),
                )
            })
        })
        .collect();

    info!("已加载插件库，新增 {} 个插件", new_plugins.len());
    Ok(Json(LoadResult {
        plugins: new_plugins,
    }))
}

async fn handle_list_plugins(State(state): State<SharedState>) -> Json<Vec<PluginInfo>> {
    let ctx = state.read().unwrap();
    let plugins = ctx.manager.plugins();
    Json(
        plugins
            .iter()
            .map(|p| {
                plugin_to_info(
                    &**p,
                    ctx.manager
                        .plugin_status(p.plugin_id())
                        .unwrap_or(PluginStatus::Loaded),
                    ctx.manager.installed_version(p.plugin_id()),
                )
            })
            .collect(),
    )
}

async fn handle_get_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<PluginInfo>, (StatusCode, Json<ApiMessage>)> {
    let ctx = state.read().unwrap();
    let plugin = ctx.manager.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiMessage {
                message: format!("未找到插件 '{}'", id),
            }),
        )
    })?;

    let status = ctx
        .manager
        .plugin_status(&id)
        .unwrap_or(PluginStatus::Loaded);
    Ok(Json(plugin_to_info(
        &*plugin,
        status,
        ctx.manager.installed_version(&id),
    )))
}

/// 手动触发插件升级：从 plugkit_plugins 读旧版本 → on_upgrade → 更新版本。
async fn handle_upgrade_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<PluginInfo>, (StatusCode, Json<ApiMessage>)> {
    let ctx = state.read().unwrap();
    let plugin = ctx.manager.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiMessage {
                message: format!("未找到插件 '{}'", id),
            }),
        )
    })?;
    let new_version = plugin.metadata().version.clone();
    let old_version = ctx
        .manager
        .installed_version(&id)
        .unwrap_or_else(|| "0.0.0".to_string());
    let db = ctx.manager.database().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: "数据库未配置".to_string(),
            }),
        )
    })?;

    info!(
        "手动升级插件 '{}' 从 {} 到 {}",
        id, old_version, new_version
    );
    plugin.on_upgrade(&*db, &old_version).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("升级失败: {}", e),
            }),
        )
    })?;

    // 更新 plugkit_plugins 表
    drop(ctx);
    let mut ctx = state.write().unwrap();
    if let Some(db) = ctx.manager.database() {
        let _ = db.execute_with(
            "UPDATE plugkit_plugins SET version = ?, upgraded_at = datetime('now') WHERE plugin_id = ?",
            &[
                crate::database::DbValue::text(&new_version),
                crate::database::DbValue::text(&id),
            ],
        );
    }
    drop(ctx);

    let ctx = state.read().unwrap();
    let status = ctx
        .manager
        .plugin_status(&id)
        .unwrap_or(PluginStatus::Loaded);
    Ok(Json(plugin_to_info(
        &*plugin,
        status,
        ctx.manager.installed_version(&id),
    )))
}

async fn handle_unload_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    query: axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    let keep_data = query.get("keep_data").map(|s| s == "true").unwrap_or(false);
    let mut ctx = state.write().unwrap();
    // 先从 library_plugins 记录中移除
    for (_lib_path, ids) in ctx.library_plugins.iter_mut() {
        ids.retain(|i| i != &id);
    }
    ctx.library_plugins.retain(|_k, v| !v.is_empty());

    // 停止该插件的 cron 任务
    if let Some(flags) = ctx.cron_flags.remove(&id) {
        for f in flags {
            f.store(true, Ordering::Relaxed);
        }
    }

    // 注销路由
    ctx.active_plugin_routes.remove(&id);

    ctx.manager.unload_plugin(&id, keep_data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("卸载失败: {:?}", e),
            }),
        )
    })?;

    let action = if keep_data {
        "保留数据卸载"
    } else {
        "完全卸载"
    };
    info!("已{}插件 '{}'", action, id);
    Ok(Json(ApiMessage {
        message: format!(
            "插件 '{}' 已{}",
            id,
            if keep_data {
                "卸载（数据已保留）"
            } else {
                "完全卸载（数据已删除）"
            }
        ),
    }))
}

async fn handle_unload_all(State(state): State<SharedState>) -> Json<ApiMessage> {
    let mut ctx = state.write().unwrap();
    // 停止所有 cron 任务
    for (_, flags) in ctx.cron_flags.drain() {
        for f in flags {
            f.store(true, Ordering::Relaxed);
        }
    }
    ctx.library_plugins.clear();
    ctx.active_plugin_routes.clear();
    ctx.manager.unload_all(false).unwrap_or_default();
    info!("已卸载所有插件");
    Json(ApiMessage {
        message: "所有插件已卸载".to_string(),
    })
}

// 生命周期状态机
async fn handle_enable_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    let mut ctx = state.write().unwrap();
    ctx.manager
        .enable_plugin(&id)
        .map_err(|e| plugin_err_to_response(e, "启用"))?;

    // 动态注册该插件的路由
    if let Some(plugin) = ctx.manager.get(&id) {
        let routes = plugin.routes();
        if !routes.is_empty() {
            ctx.active_plugin_routes.insert(id.clone(), routes);
        }
    }

    info!("已启用插件 '{}'", id);
    Ok(Json(ApiMessage {
        message: format!("插件 '{}' 已启用", id),
    }))
}

async fn handle_disable_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    let mut ctx = state.write().unwrap();
    ctx.manager
        .disable_plugin(&id)
        .map_err(|e| plugin_err_to_response(e, "禁用"))?;

    // 动态注销该插件的路由
    ctx.active_plugin_routes.remove(&id);

    if let Some(flags) = ctx.cron_flags.remove(&id) {
        for f in flags {
            f.store(true, Ordering::Relaxed);
        }
    }
    info!("已禁用插件 '{}'", id);
    Ok(Json(ApiMessage {
        message: format!("插件 '{}' 已禁用", id),
    }))
}

async fn handle_start_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    let cron_specs = {
        let mut ctx = state.write().unwrap();
        ctx.manager
            .start_plugin(&id)
            .map_err(|e| plugin_err_to_response(e, "启动"))?
    };

    if !cron_specs.is_empty() {
        let plugin = {
            let ctx = state.read().unwrap();
            ctx.manager.get(&id)
        };
        if let Some(plugin) = plugin {
            let mut flags = Vec::new();
            for spec in cron_specs {
                let flag = Arc::new(AtomicBool::new(false));
                let p = plugin.clone();
                let name = spec.name.clone();
                let secs = spec.interval_secs;
                let stop_flag = flag.clone();
                let _ = tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(secs)).await;
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        let _ = p.on_cron(&name);
                    }
                });
                flags.push(flag);
            }
            let mut ctx = state.write().unwrap();
            let _ = ctx.cron_flags.insert(id.clone(), flags);
        }
    }

    info!("已启动插件 '{}'", id);
    Ok(Json(ApiMessage {
        message: format!("插件 '{}' 已启动", id),
    }))
}

async fn handle_stop_plugin(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    {
        let mut ctx = state.write().unwrap();
        ctx.manager
            .stop_plugin(&id)
            .map_err(|e| plugin_err_to_response(e, "停止"))?;
        if let Some(flags) = ctx.cron_flags.remove(&id) {
            for f in flags {
                f.store(true, Ordering::Relaxed);
            }
        }
    }
    info!("已停止插件 '{}'", id);
    Ok(Json(ApiMessage {
        message: format!("插件 '{}' 已停止", id),
    }))
}

async fn handle_list_cron(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CronInfo>>, (StatusCode, Json<ApiMessage>)> {
    let ctx = state.read().unwrap();
    let running = matches!(ctx.manager.plugin_status(&id), Some(PluginStatus::Running));
    let plugin = ctx.manager.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiMessage {
                message: format!("未找到插件 '{}'", id),
            }),
        )
    })?;
    let crons: Vec<CronInfo> = plugin
        .cron_specs()
        .into_iter()
        .map(|c| CronInfo {
            name: c.name,
            interval_secs: c.interval_secs,
            running,
        })
        .collect();
    Ok(Json(crons))
}

async fn handle_run_cron(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<CronRunRequest>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    let plugin = {
        let ctx = state.read().unwrap();
        ctx.manager.get(&id)
    };
    let plugin = plugin.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiMessage {
                message: format!("未找到插件 '{}'", id),
            }),
        )
    })?;
    plugin
        .on_cron(&req.name)
        .map_err(|e| plugin_err_to_response(e, "cron 执行"))?;
    info!("手动触发插件 '{}' cron '{}'", id, req.name);
    Ok(Json(ApiMessage {
        message: format!("cron '{}' 已触发", req.name),
    }))
}

// ------------------------------------------------------------------------------------------------
// 服务插件前端 UI 静态文件
// ------------------------------------------------------------------------------------------------

/// 服务插件嵌入的 UI 静态文件（从内存或磁盘）。
///
/// 优先从 [`HostApp::plugin_ui_dirs`]（编译期嵌入）查找匹配的目录，
/// 未命中则回退到 `{exe_dir}/plugin/{base_dir}/dist/` 磁盘路径。
///
/// 注册为 route：`/plugin-files/*path`
pub async fn handle_serve_plugin_ui(
    State(state): State<SharedState>,
    Path(path): Path<String>,
) -> Result<Response<Body>, (StatusCode, Json<ApiMessage>)> {
    // 1) 从编译期嵌入的 plugin_ui_dirs 查找
    {
        let ctx = state.read().unwrap();
        for (base_dir, dir) in &ctx.plugin_ui_dirs {
            let prefix = format!("{}/dist/", base_dir);
            if let Some(rest) = path.strip_prefix(&prefix) {
                if let Some(file) = dir.get_file(rest) {
                    return {
                        let mime = mime_guess::from_path(rest)
                            .first_or_octet_stream()
                            .to_string();
                        let response = Response::builder()
                            .header("Content-Type", mime)
                            .body(Body::from(file.contents().to_vec()))
                            .unwrap();
                        Ok(response)
                    };
                }
            }
        }
    }

    // 2) 回退到磁盘读
    let exe_path = std::env::current_exe().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: "Cannot determine executable path".to_string(),
            }),
        )
    })?;
    let exe_dir = exe_path.parent().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: "Cannot determine executable directory".to_string(),
            }),
        )
    })?;

    let file_path = exe_dir.join("plugin").join(&path);

    // 防止路径穿越攻击
    let plugin_root = exe_dir.join("plugin");
    if !file_path.starts_with(&plugin_root) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiMessage {
                message: "Forbidden".to_string(),
            }),
        ));
    }

    let content = tokio::fs::read_to_string(&file_path).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiMessage {
                message: format!("Plugin UI file not found: {}", path),
            }),
        )
    })?;

    let content_type = if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else if path.ends_with(".map") {
        "application/json; charset=utf-8"
    } else {
        "application/octet-stream"
    };

    let response = Response::builder()
        .header("Content-Type", content_type)
        .body(Body::from(content))
        .unwrap();

    Ok(response)
}

// ------------------------------------------------------------------------------------------------
// 服务宿主前端 SPA
// ------------------------------------------------------------------------------------------------

/// 编译期嵌入的通用插件管理前端。
pub static FRONTEND_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

/// 服务宿主前端 SPA（从编译期嵌入的 [`FRONTEND_DIST`] 读）。
///
/// 这是一个 catch-all fallback handler：
/// 1. 先尝试 `dist/<path>`（静态资源，如 JS/CSS/HTML 等）
/// 2. 找不到时回退到 `dist/index.html`（SPA fallback）
/// 3. 连 index.html 都没有则返回内置提示页面
pub async fn serve_frontend_handler(req: Request<Body>) -> Response<Body> {
    let path = req.uri().path().trim_start_matches('/').to_string();

    // 试按子路径查找文件
    let tried = if path.is_empty() {
        None
    } else {
        FRONTEND_DIST.get_file(&path)
    };

    // 命中静态资源,直接返回
    if let Some(file) = tried {
        let mime = mime_guess::from_path(file.path())
            .first_or_octet_stream()
            .to_string();
        return Response::builder()
            .header("Content-Type", mime)
            .body(Body::from(file.contents().to_vec()))
            .unwrap();
    }

    // 未命中 → SPA fallback 到 index.html
    if let Some(index) = FRONTEND_DIST.get_file("index.html") {
        return Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(index.contents().to_vec()))
            .unwrap();
    }

    // 连 index.html 都没有 (前端未 build):返回提示 HTML
    let body = b"<!doctype html><html><body style=\"font-family:sans-serif;padding:2rem\">\
        <h1>plugkit</h1>\
        <p>Frontend not embedded. Run <code>make frontend</code> then rebuild.</p>\
        </body></html>";
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(body.to_vec()))
        .unwrap()
}

/// 安装插件 — 接收 `.plugin` 文件上传（tar.gz 格式）。
///
/// 请求体为原始二进制数据（`Content-Type: application/octet-stream`）。
/// 服务器将解包到第一个插件搜索目录并自动加载。
///
/// 用法:
/// ```bash
/// curl -X POST --data-binary @my_plugin.plugin http://localhost:3000/api/plugins/install
/// ```
pub async fn handle_install_plugin(
    State(state): State<SharedState>,
    body: axum::body::Bytes,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    use flate2::read::GzDecoder;
    use std::fs;
    use std::io::Read;
    use tar::Archive;

    // 获取插件搜索目录
    let search_dir = {
        let ctx = state.read().unwrap();
        ctx.plugin_search_dirs
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("bin/plugins"))
    };

    // 创建临时目录
    let temp_dir = std::env::temp_dir().join(format!(
        "plugkit_install_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("无法创建临时目录: {}", e),
            }),
        )
    })?;

    // 写入临时文件
    let plugin_path = temp_dir.join("upload.plugin");
    fs::write(&plugin_path, &body).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("无法写入临时文件: {}", e),
            }),
        )
    })?;

    // 解压 tar.gz 到临时目录
    let file = fs::File::open(&plugin_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("无法打开临时文件: {}", e),
            }),
        )
    })?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(&temp_dir).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiMessage {
                message: format!("解包失败: {} — 请确认文件是有效的 .plugin (tar.gz) 格式", e),
            }),
        )
    })?;

    // 查找解包后的 dylib（只在临时目录中查找，不扫描整个搜索目录）
    let dylib_ext = PLATFORM_DYLIB_EXTENSION;
    let dylib_paths: Vec<PathBuf> = walkdir::WalkDir::new(&temp_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == dylib_ext)
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    if dylib_paths.is_empty() {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiMessage {
                message: format!(".plugin 文件中未找到 .{} 动态库文件", dylib_ext),
            }),
        ));
    }

    // 复制 dylib 到搜索目录并加载
    let mut loaded = Vec::new();
    for src_path in &dylib_paths {
        let file_name = src_path.file_name().unwrap();
        let dest_path = search_dir.join(file_name);

        // 复制到搜索目录
        fs::copy(src_path, &dest_path).map_err(|e| {
            let _ = fs::remove_dir_all(&temp_dir);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiMessage {
                    message: format!("复制 dylib 失败: {}", e),
                }),
            )
        })?;

        // 加载
        let mut ctx = state.write().unwrap();
        match ctx.manager.load_plugins_from(&dest_path) {
            Ok(()) => {
                let plugins = ctx.manager.plugins();
                let plugin_ids: Vec<String> =
                    plugins.iter().map(|p| p.plugin_id().clone()).collect();
                let _ = ctx
                    .library_plugins
                    .insert(dest_path.clone(), plugin_ids.clone());
                loaded.extend(plugin_ids);
                info!("已安装插件库: {:?}", dest_path);
            }
            Err(e) => {
                warn!("加载插件失败: {:?} — {}", dest_path, e);
            }
        }
    }

    // 清理临时目录
    let _ = fs::remove_dir_all(&temp_dir);

    let msg = if loaded.is_empty() {
        "插件安装失败：未找到可加载的插件".to_string()
    } else {
        format!(
            "✅ 安装成功，加载了 {} 个插件: {}",
            loaded.len(),
            loaded.join(", ")
        )
    };

    info!("{}", msg);
    Ok(Json(ApiMessage { message: msg }))
}

// ------------------------------------------------------------------------------------------------
// 插件自定义 HTTP 路由（catch-all）
// ------------------------------------------------------------------------------------------------

/// 处理 `/plugin-api/:plugin_id/*route` catch-all 请求。
///
/// 从 `HostApp.active_plugin_routes` 注册表中查找插件路由，
/// 按 method + path 匹配到对应 handler 函数指针后调用。
/// 未被注册的路由统一返回 404。
async fn handle_plugin_route(
    State(state): State<SharedState>,
    Path((plugin_id, route)): Path<(String, String)>,
    method: Method,
    headers: http::HeaderMap,
    axum::extract::Query(query_params): axum::extract::Query<HashMap<String, String>>,
    body_bytes: axum::body::Bytes,
) -> impl IntoResponse {
    let ctx = state.read().unwrap();
    let routes = ctx.active_plugin_routes.get(&plugin_id).cloned();
    let db = ctx.manager.database().clone();
    let plugin = ctx.manager.get(&plugin_id);
    drop(ctx);

    // 路由未注册（插件不存在或未启用）
    let (plugin, routes) = match (plugin, routes) {
        (Some(p), Some(r)) => (p, r),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiMessage {
                    message: "路由未找到".into(),
                }),
            )
                .into_response();
        }
    };

    // 构建标准 http::Request<Vec<u8>>
    let path = format!("/{}", route);
    let http_req = build_http_request(&method, &headers, &path, &query_params, &body_bytes);
    let req_path = http_req.uri().path().to_string();

    for route in &routes {
        if route.method == method && path_prefix_match(&route.path, &req_path) {
            let db_ref: Arc<dyn DatabaseExt> = db
                .clone()
                .unwrap_or_else(|| Arc::new(crate::manager::NoopDatabase));
            let event_bus = {
                let ctx = state.read().unwrap();
                ctx.event_bus.clone()
            };
            let auth_service = {
                let ctx = state.read().unwrap();
                ctx.auth_service.clone()
            };

            // 从 Authorization header 提取并验证 token，填充 principal
            let principal = auth_service.as_ref().and_then(|svc| {
                let auth_header = headers.get(http::header::AUTHORIZATION)?;
                let token = auth_header.to_str().ok()?.strip_prefix("Bearer ")?;
                svc.verify_token(token).ok()
            });

            let request_ctx = RequestCtx {
                principal,
                auth_service: auth_service
                    .unwrap_or_else(|| Arc::new(AuthServiceImpl::new(db_ref.clone()))),
                db: db_ref,
                event_bus: Arc::new(event_bus),
                plugin_id: plugin_id.clone(),
                request_id: uuid::Uuid::new_v4().to_string(),
            };
            let resp = (route.handler)(plugin.as_ref(), &request_ctx, http_req);
            return convert_http_response(resp);
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "路由未找到", "path": req_path})),
    )
        .into_response()
}

/// 检查请求路径是否匹配路由声明的路径（支持前缀匹配）。
fn path_prefix_match(route_path: &str, req_path: &str) -> bool {
    req_path == route_path || req_path.starts_with(&format!("{}/", route_path))
}

/// 从 axum 的请求组件构建标准 `http::Request<Vec<u8>>`。
fn build_http_request(
    method: &Method,
    headers: &http::HeaderMap,
    path: &str,
    query_params: &HashMap<String, String>,
    body_bytes: &[u8],
) -> Request<Vec<u8>> {
    // 构建 URI（含查询参数）
    let mut uri = path.to_string();
    if !query_params.is_empty() {
        let qs: Vec<String> = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        uri.push('?');
        uri.push_str(&qs.join("&"));
    }

    let mut builder = Request::builder().method(method.clone()).uri(&uri);
    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }
    builder.body(body_bytes.to_vec()).unwrap()
}

/// 将 `http::Response<Vec<u8>>` 转为 axum Response。
fn convert_http_response(resp: Response<Vec<u8>>) -> axum::response::Response {
    let (parts, body) = resp.into_parts();
    let mut builder = axum::response::Response::builder().status(parts.status);
    for (key, value) in parts.headers.iter() {
        builder = builder.header(key, value);
    }
    builder.body(Body::from(body)).unwrap()
}

// ------------------------------------------------------------------------------------------------
// 路由组装
// ------------------------------------------------------------------------------------------------

/// 创建通用插件宿主管理路由。
///
/// 返回的 `Router` 包含以下端点（所有端点需通过 `.with_state(state)` 注入 `SharedState`）：
///
/// | 方法 | 路径 | 说明 |
/// |------|------|------|
/// | GET  | `/api/libraries` | 扫描可用插件库 |
/// | POST | `/api/libraries/:name/load` | 加载插件库 |
/// | GET  | `/api/plugins` | 列出已加载插件 |
/// | GET  | `/api/plugins/:id` | 获取单个插件信息 |
/// | DELETE | `/api/plugins/:id` | 卸载插件 |
/// | DELETE | `/api/plugins` | 卸载所有插件 |
/// | POST | `/api/plugins/:id/enable` | 启用插件 |
/// | POST | `/api/plugins/:id/disable` | 禁用插件 |
/// | POST | `/api/plugins/:id/start` | 启动插件（含 cron 调度） |
/// | POST | `/api/plugins/:id/stop` | 停止插件 |
/// | GET  | `/api/plugins/:id/cron` | 列出插件定时任务 |
/// | POST | `/api/plugins/:id/cron/run` | 手动触发插件定时任务 |
/// | GET  | `/plugin-files/*path` | 服务插件 UI 静态文件 |
/// | ANY  | `/plugin-api/:plugin_id/*route` | 插件自定义 HTTP 路由 |
///
/// 调用方需在返回的 Router 上补充：
/// - `fallback(serve_frontend_handler)` 或自定义 fallback
/// - 业务路由（如发布新闻）
pub fn host_router() -> Router<SharedState> {
    Router::new()
        // 认证端点
        .route("/auth/login", post(handle_login))
        .route("/auth/logout", post(handle_logout))
        .route("/auth/revoke-all", post(handle_revoke_all))
        .route("/auth/me", get(handle_me))
        // 插件库管理
        .route("/api/libraries", get(handle_scan_libraries))
        .route("/api/libraries/:name/load", post(handle_load_library))
        // 插件安装（允许最大 50MB 上传）
        .route("/api/plugins/install", post(handle_install_plugin))
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024))
        // 插件管理
        .route("/api/plugins", get(handle_list_plugins))
        .route(
            "/api/plugins/:id",
            get(handle_get_plugin).delete(handle_unload_plugin),
        )
        // 插件生命周期状态机
        .route("/api/plugins/:id/enable", post(handle_enable_plugin))
        .route("/api/plugins/:id/disable", post(handle_disable_plugin))
        .route("/api/plugins/:id/start", post(handle_start_plugin))
        .route("/api/plugins/:id/stop", post(handle_stop_plugin))
        .route("/api/plugins/:id/cron", get(handle_list_cron))
        .route("/api/plugins/:id/cron/run", post(handle_run_cron))
        .route("/api/plugins/:id/upgrade", post(handle_upgrade_plugin))
        // 批量操作
        .route("/api/plugins", delete(handle_unload_all))
        // 插件前端 UI 静态文件
        .route("/plugin-files/*path", get(handle_serve_plugin_ui))
        // 插件自定义 HTTP 路由（catch-all）
        .route("/plugin-api/:plugin_id/*route", any(handle_plugin_route))
        // CORS
        .layer(CorsLayer::permissive())
}

// ------------------------------------------------------------------------------------------------
// 认证初始化辅助函数
// ------------------------------------------------------------------------------------------------

/// 初始化认证相关数据库表。
fn init_auth_tables(db: &dyn DatabaseExt) -> crate::error::Result<()> {
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            token_version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )?;
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS tokens (
            jti TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id),
            expires_at TEXT NOT NULL,
            revoked_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )?;
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            role TEXT NOT NULL,
            permission TEXT NOT NULL,
            plugin_id TEXT NOT NULL,
            PRIMARY KEY (role, permission)
        )
        "#,
    )?;
    db.execute("CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id)")?;
    db.execute("CREATE INDEX IF NOT EXISTS idx_tokens_expires_at ON tokens(expires_at)")?;
    Ok(())
}

/// 确保默认管理员存在。
///
/// 密码优先级：
/// 1. 环境变量 `PLUGKIT_ADMIN_PASSWORD`
/// 2. 随机生成并打印到控制台
fn ensure_default_admin(db: &dyn DatabaseExt) -> crate::error::Result<()> {
    let exists = db
        .query_with(
            "SELECT id FROM users WHERE username = ?",
            &[crate::database::DbValue::text("admin")],
        )?
        .first()
        .is_some();

    if exists {
        return Ok(());
    }

    let password = std::env::var("PLUGKIT_ADMIN_PASSWORD").unwrap_or_else(|_| {
        use rand::Rng;
        let random: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        println!("🔐 默认管理员密码（仅显示一次）: {}", random);
        random
    });

    let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).map_err(|e| {
        crate::error::Error::from(ErrorKind::DatabaseError(format!(
            "bcrypt hash failed: {}",
            e
        )))
    })?;

    db.execute_with(
        "INSERT INTO users (username, password_hash) VALUES (?, ?)",
        &[
            crate::database::DbValue::text("admin"),
            crate::database::DbValue::text(hash),
        ],
    )?;

    // 默认角色权限
    db.execute_with(
        "INSERT OR IGNORE INTO role_permissions (role, permission, plugin_id) VALUES (?, ?, ?)",
        &[
            crate::database::DbValue::text("admin"),
            crate::database::DbValue::text("admin:access"),
            crate::database::DbValue::text("core"),
        ],
    )?;
    db.execute_with(
        "INSERT OR IGNORE INTO role_permissions (role, permission, plugin_id) VALUES (?, ?, ?)",
        &[
            crate::database::DbValue::text("admin"),
            crate::database::DbValue::text("users:manage"),
            crate::database::DbValue::text("core"),
        ],
    )?;

    println!("✅ 默认管理员 'admin' 已创建");
    Ok(())
}
