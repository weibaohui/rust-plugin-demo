/*!
plugkit 通用插件宿主 — 纯框架演示，无任何业务代码。

启动后即获得完整的插件管理 API + 通用管理前端。
二开者从此起步，补充自己的业务路由即可。
*/

use plugkit::database::SqliteDatabase;
use plugkit::host::{host_router, serve_frontend_handler, HostApp};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("plugkit=info")
        .init();

    let db = SqliteDatabase::in_memory()?;
    let app = HostApp::new()
        .with_database(Arc::new(db))
        .with_plugin_search_dir("bin/plugins");
    let state: plugkit::host::SharedState = Arc::new(std::sync::RwLock::new(app));

    let router = host_router()
        .fallback(serve_frontend_handler)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("plugkit 通用插件管理后台已启动 → http://{}", addr);
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  plugkit 通用插件管理后台已启动                          ║");
    println!("║  后端 API:  http://localhost:3000/api                   ║");
    println!("║  前端 UI:   http://localhost:3000/                      ║");
    println!("║                                                         ║");
    println!("║  通过 /api/libraries 扫描并加载插件 dylib               ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
