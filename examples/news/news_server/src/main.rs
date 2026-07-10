/*!
新闻插件宿主示例 — 展示如何基于 `plugkit` 框架加载异构插件。

每个插件 crate 都是独立的，自己的类型（`AfpPlugin`、`ReutersPlugin`...），
宿主通过 `dyn Plugin` trait object 统一管理它们，无需任何共享类型 crate。
*/

use plugkit::database::{DatabaseExt, SqliteDatabase};
use plugkit::host::{host_router, serve_frontend_handler, ApiMessage, HostApp, HostContext};
use plugkit::plugin::Plugin;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    Json,
};
use std::sync::Arc;
use tracing::info;

// ------------------------------------------------------------------------------------------------
// HostContext 实现（宿主侧，业务无关）
// ------------------------------------------------------------------------------------------------

struct ServerHostContext {
    server_name: &'static str,
    server_version: &'static str,
    plugin_count: usize,
}

impl HostContext for ServerHostContext {
    fn server_name(&self) -> &str {
        self.server_name
    }
    fn server_version(&self) -> &str {
        self.server_version
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
// main
// ------------------------------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("news_server=info,plugkit=info")
        .init();

    let db: Arc<dyn DatabaseExt> = {
        let db_path =
            std::env::var("NEWS_SERVER_DB").unwrap_or_else(|_| "news_server.sqlite".to_string());
        let db = SqliteDatabase::open(&db_path)?;
        info!("已打开 SQLite 数据库: {}", db.describe());
        Arc::new(db)
    };

    // 用 dyn Plugin 加载任意独立插件 crate
    // 搜索目录：各插件 crate 自己的 target/debug（包含已构建的 .dylib/.so）
    let host_app = HostApp::new()
        .with_database(db)
        .with_plugin_search_dir("../plugins/afp_plugin/target/debug")
        .with_plugin_search_dir("../plugins/reuters_plugin/target/debug");
    let state: plugkit::host::SharedState = Arc::new(std::sync::RwLock::new(host_app));

    // 构建通用插件管理路由 + 宿主前端
    let router = host_router()
        .fallback(serve_frontend_handler)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    info!("新闻插件管理示例已启动 → http://{}", addr);
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  新闻插件管理示例已启动                                  ║");
    println!("║  后端 API:  http://localhost:3000/api                   ║");
    println!("║  前端 UI:  http://localhost:3000/                       ║");
    println!("║                                                         ║");
    println!("║  通过 /api/libraries 扫描并加载插件 dylib               ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}