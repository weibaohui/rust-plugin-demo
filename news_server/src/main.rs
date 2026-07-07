use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use dygpi::manager::{PluginManager, PLATFORM_DYLIB_EXTENSION, PLATFORM_DYLIB_PREFIX};
use dygpi::plugin::Plugin;
use news_api::{HostContext, NewsAgencyPlugin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;
use tracing::info;

// ------------------------------------------------------------------------------------------------
// 状态：共享的插件管理器 + 元数据
// ------------------------------------------------------------------------------------------------

struct AppContext {
    manager: PluginManager<NewsAgencyPlugin>,
    /// 记录每个库路径 → 它贡献的插件 ID 列表
    library_plugins: HashMap<PathBuf, Vec<String>>,
    /// 已发布的文章计数（供 HostContext 使用）
    article_count: AtomicUsize,
}

type SharedState = Arc<RwLock<AppContext>>;

// ------------------------------------------------------------------------------------------------
// API 请求/响应类型
// ------------------------------------------------------------------------------------------------

#[derive(Serialize)]
struct LibraryInfo {
    name: String,
    file_name: String,
    path: String,
    loaded: bool,
    plugin_count: usize,
}

#[derive(Serialize, Clone)]
struct PluginInfo {
    id: String,
    agency: String,
}

#[derive(Serialize)]
struct ArticleResponse {
    headline: String,
    body: String,
    dateline: String,
    agency: String,
}

#[derive(Deserialize)]
struct PublishRequest {
    headline: String,
    body: String,
}

#[derive(Serialize)]
struct ApiMessage {
    message: String,
}

#[derive(Serialize)]
struct LibraryScanResult {
    libraries: Vec<LibraryInfo>,
}

#[derive(Serialize)]
struct LoadResult {
    plugins: Vec<PluginInfo>,
}

// ------------------------------------------------------------------------------------------------
// HostContext 实现：宿主向插件暴露的信息和回调能力
// ------------------------------------------------------------------------------------------------

struct ServerHostContext {
    server_name: &'static str,
    server_version: &'static str,
    article_count: usize,
    plugin_count: usize,
}

impl HostContext for ServerHostContext {
    fn server_name(&self) -> &str {
        self.server_name
    }
    fn server_version(&self) -> &str {
        self.server_version
    }
    fn article_count(&self) -> usize {
        self.article_count
    }
    fn log_message(&self, msg: &str) {
        info!("[Plugin Log] {}", msg);
    }
    fn server_time(&self) -> String {
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }
    fn plugin_count(&self) -> usize {
        self.plugin_count
    }
}

// ------------------------------------------------------------------------------------------------
// 主函数
// ------------------------------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("news_server=info,dygpi=info,news_api=info")
        .init();

    let state: SharedState = Arc::new(RwLock::new(AppContext {
        manager: PluginManager::default(),
        library_plugins: HashMap::new(),
        article_count: AtomicUsize::new(0),
    }));

    let app = Router::new()
        // 插件库管理
        .route("/api/libraries", get(scan_libraries_handler))
        .route("/api/libraries/:name/load", post(load_library_handler))
        // 插件管理
        .route("/api/plugins", get(list_plugins_handler))
        .route(
            "/api/plugins/:id",
            get(get_plugin_handler).delete(unload_plugin_handler),
        )
        .route("/api/plugins/:id/publish", post(publish_handler))
        // 批量操作
        .route("/api/plugins", delete(unload_all_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    info!("新闻插件管理后台已启动 → http://{}", addr);
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  新闻插件管理后台已启动                                  ║");
    println!("║  后端 API:  http://localhost:3000/api                   ║");
    println!("║  启动前端:  cd frontend && npm run dev                  ║");
    println!("║  前端地址:  http://localhost:5173                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ------------------------------------------------------------------------------------------------
// 扫描可用的插件库
// ------------------------------------------------------------------------------------------------

fn find_dylib_paths() -> Vec<PathBuf> {
    let mut results = Vec::new();

    // 尝试从可执行文件路径推断项目根目录
    let project_dirs: Vec<PathBuf> = {
        let mut dirs = Vec::new();
        // 当前工作目录
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd.join("target/debug"));
            dirs.push(cwd.join("target/release"));
        }
        // 从 exe 路径推断（开发时 target/debug/news_server）
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                // parent = target/debug
                dirs.push(parent.to_path_buf());
                if let Some(parent2) = parent.parent() {
                    // parent2 = target
                    dirs.push(parent2.join("release"));
                    // parent2.parent = 项目根
                    if let Some(project_root) = parent2.parent() {
                        dirs.push(project_root.join("target/debug"));
                        dirs.push(project_root.join("target/release"));
                    }
                }
            }
        }
        dirs
    };

    for dir in &project_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == PLATFORM_DYLIB_EXTENSION {
                        // 去重
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

// 去掉 dylib 文件名中的 "lib" 前缀
fn clean_lib_name(file_stem: &str) -> String {
    file_stem
        .strip_prefix(PLATFORM_DYLIB_PREFIX)
        .unwrap_or(file_stem)
        .to_string()
}

async fn scan_libraries_handler(State(state): State<SharedState>) -> Json<LibraryScanResult> {
    let ctx = state.read().unwrap();
    let mut libs: Vec<LibraryInfo> = Vec::new();

    for path in find_dylib_paths() {
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

// ------------------------------------------------------------------------------------------------
// 加载插件库
// ------------------------------------------------------------------------------------------------

async fn load_library_handler(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<LoadResult>, (StatusCode, Json<ApiMessage>)> {
    // 查找匹配的库文件（支持 "reuters_plugin" 或 "libreuters_plugin" 两种格式）
    let path = find_dylib_paths()
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

    // 找出新加入的插件（减去之前已有的）
    let existing = ctx.library_plugins.get(&path).cloned().unwrap_or_default();
    let new_ids: Vec<String> = plugin_ids
        .iter()
        .filter(|id| !existing.contains(id))
        .cloned()
        .collect();

    ctx.library_plugins.insert(path, plugin_ids);

    let new_plugins: Vec<PluginInfo> = new_ids
        .iter()
        .filter_map(|id| {
            ctx.manager.get(id).map(|p| PluginInfo {
                id: p.plugin_id().clone(),
                agency: p.agency_name().to_string(),
            })
        })
        .collect();

    info!("已加载插件库，新增 {} 个插件", new_plugins.len());
    Ok(Json(LoadResult {
        plugins: new_plugins,
    }))
}

// ------------------------------------------------------------------------------------------------
// 列出已加载的插件
// ------------------------------------------------------------------------------------------------

async fn list_plugins_handler(State(state): State<SharedState>) -> Json<Vec<PluginInfo>> {
    let ctx = state.read().unwrap();
    let plugins = ctx.manager.plugins();
    Json(
        plugins
            .iter()
            .map(|p| PluginInfo {
                id: p.plugin_id().clone(),
                agency: p.agency_name().to_string(),
            })
            .collect(),
    )
}

// ------------------------------------------------------------------------------------------------
// 获取单个插件
// ------------------------------------------------------------------------------------------------

async fn get_plugin_handler(
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

    Ok(Json(PluginInfo {
        id: plugin.plugin_id().clone(),
        agency: plugin.agency_name().to_string(),
    }))
}

// ------------------------------------------------------------------------------------------------
// 调用插件：发布新闻
// ------------------------------------------------------------------------------------------------

async fn publish_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<PublishRequest>,
) -> Result<Json<ArticleResponse>, (StatusCode, Json<ApiMessage>)> {
    let state_guard = state.read().unwrap();
    let plugin = state_guard.manager.get(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiMessage {
                message: format!("未找到插件 '{}'", id),
            }),
        )
    })?;

    info!(
        "调用插件 '{}' ({}) 发布新闻: {}",
        plugin.agency_name(),
        plugin.plugin_id(),
        &req.headline
    );

    // 构建宿主上下文，插件可通过它获取信息并调用宿主方法
    let host_ctx = ServerHostContext {
        server_name: "News Server",
        server_version: "0.1.0",
        article_count: state_guard.article_count.load(Ordering::Relaxed),
        plugin_count: state_guard.manager.len(),
    };

    // 释放读锁再调用（避免持有锁期间调用插件代码）
    drop(state_guard);

    let article = plugin.publish(&host_ctx, &req.headline, &req.body);

    // 自增文章计数
    let state_guard = state.write().unwrap();
    state_guard.article_count.fetch_add(1, Ordering::Relaxed);
    drop(state_guard);

    Ok(Json(ArticleResponse {
        headline: article.headline,
        body: article.body,
        dateline: article.dateline,
        agency: article.agency,
    }))
}

// ------------------------------------------------------------------------------------------------
// 卸载单个插件
// ------------------------------------------------------------------------------------------------

async fn unload_plugin_handler(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiMessage>, (StatusCode, Json<ApiMessage>)> {
    let mut ctx = state.write().unwrap();
    // 先从 library_plugins 记录中移除
    for (_lib_path, ids) in ctx.library_plugins.iter_mut() {
        ids.retain(|i| i != &id);
    }
    ctx.library_plugins.retain(|_k, v| !v.is_empty());

    ctx.manager.unload_plugin(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiMessage {
                message: format!("卸载失败: {:?}", e),
            }),
        )
    })?;

    info!("已卸载插件 '{}'", id);
    Ok(Json(ApiMessage {
        message: format!("插件 '{}' 已卸载", id),
    }))
}

// ------------------------------------------------------------------------------------------------
// 卸载所有插件
// ------------------------------------------------------------------------------------------------

async fn unload_all_handler(State(state): State<SharedState>) -> Json<ApiMessage> {
    let mut ctx = state.write().unwrap();
    ctx.library_plugins.clear();
    ctx.manager.unload_all().unwrap_or_default();
    info!("已卸载所有插件");
    Json(ApiMessage {
        message: "所有插件已卸载".to_string(),
    })
}
