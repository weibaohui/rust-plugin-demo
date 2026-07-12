/*!
plugkit 通用插件宿主 — 纯框架演示，无任何业务代码。

启动后即获得完整的插件管理 API + 通用管理前端。
二开者从此起步，补充自己的业务路由即可。

用法：
- `plugkit`               启动宿主
- `plugkit new <name>`    生成插件骨架
- `plugkit package <dylib> [--ui-dir <path>]`  打包插件为 .plugin 文件
*/

use plugkit::database::SqliteDatabase;
use plugkit::host::{host_router, serve_frontend_handler, HostApp};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // 子命令：plugkit new <name>
    if args.len() >= 3 && args[1] == "new" {
        let plugin_name = &args[2];
        let target_dir = std::env::current_dir()?;
        plugkit::cli::generate_plugin(&target_dir, plugin_name)?;
        return Ok(());
    }
    if args.len() >= 2 && args[1] == "new" {
        eprintln!("用法: plugkit new <插件名>");
        eprintln!("插件名仅允许小写字母、数字、下划线");
        std::process::exit(1);
    }

    // 子命令：plugkit package <dylib> [--ui-dir <path>]
    if args.len() >= 3 && args[1] == "package" {
        let dylib_path = std::path::PathBuf::from(&args[2]);
        let ui_dir = if args.len() >= 4 && args[3] == "--ui-dir" {
            args.get(4).map(|p| std::path::PathBuf::from(p))
        } else {
            None
        };
        let output_name = dylib_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin")
            .to_string();
        let output_path = std::path::PathBuf::from(format!("{}.plugin", output_name));
        plugkit::cli::package_plugin(
            &dylib_path,
            ui_dir.as_deref(),
            &output_path,
        )?;
        return Ok(());
    }
    if args.len() >= 2 && args[1] == "package" {
        eprintln!("用法: plugkit package <dylib_path> [--ui-dir <path>]");
        eprintln!("示例: plugkit package target/debug/libafp_plugin.dylib --ui-dir examples/news/plugins/afp_plugin/ui/dist");
        std::process::exit(1);
    }

    tracing_subscriber::fmt()
        .with_env_filter("plugkit=info")
        .init();

    let db = SqliteDatabase::in_memory()?;
    let app = HostApp::new()
        .with_database(Arc::new(db))
        .with_plugin_search_dir("bin/plugins")
        .auto_load();
    let state: plugkit::host::SharedState = Arc::new(std::sync::RwLock::new(app));

    // 启动热重载监听
    let _hot_reload_watch = HostApp::start_hot_reload(state.clone());

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
