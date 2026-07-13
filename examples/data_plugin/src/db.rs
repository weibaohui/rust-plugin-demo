//! SeaORM 数据库连接管理。

use sea_orm::DatabaseConnection;
use std::future::Future;
use std::sync::OnceLock;

static DB_CONN: OnceLock<DatabaseConnection> = OnceLock::new();

/// 初始化 SeaORM 连接（在插件加载时调用一次）。
pub fn init_connection(conn: DatabaseConnection) {
    let _ = DB_CONN.set(conn);
}

/// 获取 SeaORM 连接。
pub fn connection() -> &'static DatabaseConnection {
    DB_CONN.get().expect("SeaORM connection not initialized")
}

/// 在 tokio 异步上下文中调用同步 handler 时，安全地阻塞等待一个 Future。
pub fn block_on<F: Future>(f: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
}
