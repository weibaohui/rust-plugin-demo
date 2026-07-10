/*!
新闻插件宿主示例 — 展示如何基于 `plugkit` 框架构建业务宿主。

通用插件管理能力（库扫描/加载/卸载、生命周期状态机、cron 调度、UI 托盘）
全部来自 `plugkit::host`，本文件只保留新闻业务逻辑：
  - `publish` 发布新闻
  - `FRONTEND_DIST` 宿主前端 SPA 服务
  - `HostContext` 实现（含 article_count 业务字段）
*/

use plugkit::database::{DatabaseExt, SqliteDatabase};
use plugkit::host::{host_router, ApiMessage, SharedState};
use plugkit::plugin::Plugin;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    routing::post,
    Json,
};
use include_dir::{include_dir, Dir};
use news_api::{HostContext, NewsAgencyPlugin};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::info;

// ------------------------------------------------------------------------------------------------
// 编译期嵌入宿主前端 (frontend/dist/)
// ------------------------------------------------------------------------------------------------

pub static FRONTEND_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../frontend/dist");

// ------------------------------------------------------------------------------------------------
// 文章计数（业务字段，全局静态）
// ------------------------------------------------------------------------------------------------

static ARTICLE_COUNT: AtomicUsize = AtomicUsize::new(0);

// ------------------------------------------------------------------------------------------------
// API 请求/响应类型（仅新闻业务）
// ------------------------------------------------------------------------------------------------

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

// ------------------------------------------------------------------------------------------------
// HostContext 实现
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
// 业务 Handler：发布新闻
// ------------------------------------------------------------------------------------------------

async fn publish_handler(
    State(state): State<SharedState<NewsAgencyPlugin>>,
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

    let host_ctx = ServerHostContext {
        server_name: "News Server",
        server_version: "0.1.0",
        article_count: ARTICLE_COUNT.load(Ordering::Relaxed),
        plugin_count: state_guard.manager.len(),
    };

    drop(state_guard);

    let article = plugin.publish(&host_ctx, &req.headline, &req.body);

    ARTICLE_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(Json(ArticleResponse {
        headline: article.headline,
        body: article.body,
        dateline: article.dateline,
        agency: article.agency,
    }))
}

// ------------------------------------------------------------------------------------------------
// 宿主前端 SPA fallback（从编译期嵌入的 FRONTEND_DIST 读）
// ------------------------------------------------------------------------------------------------

async fn frontend_fallback(req: Request<Body>) -> Response<Body> {
    let path = req.uri().path().trim_start_matches('/').to_string();

    let tried = if path.is_empty() {
        None
    } else {
        FRONTEND_DIST.get_file(&path)
    };

    if let Some(file) = tried {
        let mime = mime_guess::from_path(file.path())
            .first_or_octet_stream()
            .to_string();
        return Response::builder()
            .header("Content-Type", mime)
            .body(Body::from(file.contents().to_vec()))
            .unwrap();
    }

    if let Some(index) = FRONTEND_DIST.get_file("index.html") {
        return Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(index.contents().to_vec()))
            .unwrap();
    }

    let body = b"<!doctype html><html><body style=\"font-family:sans-serif;padding:2rem\">\
        <h1>news_server (example)</h1>\
        <p>Frontend not embedded. Run <code>make ui-frontend</code> then rebuild.</p>\
        </body></html>";
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(body.to_vec()))
        .unwrap()
}

// ------------------------------------------------------------------------------------------------
// main
// ------------------------------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            "news_server=info,plugkit=info,news_api=info,afp_plugin=info,reuters_plugin=info",
        )
        .init();

    let db: Arc<dyn DatabaseExt> = {
        let db_path =
            std::env::var("NEWS_SERVER_DB").unwrap_or_else(|_| "news_server.sqlite".to_string());
        let db = SqliteDatabase::open(&db_path)?;
        info!("已打开 SQLite 数据库: {}", db.describe());
        Arc::new(db)
    };

    let host_app = plugkit::host::HostApp::<NewsAgencyPlugin>::new().with_database(db);
    let state: SharedState<NewsAgencyPlugin> = Arc::new(std::sync::RwLock::new(host_app));

    // 构建通用插件管理路由 + 新闻业务路由
    let router = host_router::<NewsAgencyPlugin>()
        .route("/api/plugins/:id/publish", post(publish_handler))
        .fallback(frontend_fallback)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    info!("新闻插件管理示例已启动 → http://{}", addr);
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  新闻插件管理示例已启动                                  ║");
    println!("║  后端 API:  http://localhost:3000/api                   ║");
    println!("║  前端 UI:  http://localhost:3000/                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}