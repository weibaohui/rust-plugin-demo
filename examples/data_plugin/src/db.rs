//! SeaORM 数据库连接管理。
//!
//! data_plugin 拥有独立的 tokio runtime，不依赖宿主运行时。

use sea_orm::DatabaseConnection;
use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static ASYNC_RT: OnceLock<Runtime> = OnceLock::new();
static DB_CONN: OnceLock<DatabaseConnection> = OnceLock::new();

/// 初始化独立 tokio runtime（在 `register_plugins` 中调用）。
pub fn init_runtime() {
    ASYNC_RT
        .set(Runtime::new().expect("data_plugin: failed to create tokio runtime"))
        .ok();
}

/// 初始化 SeaORM 连接。
pub fn init_connection(conn: DatabaseConnection) {
    let _ = DB_CONN.set(conn);
}

/// 获取 SeaORM 连接。
pub fn connection() -> &'static DatabaseConnection {
    DB_CONN.get().expect("SeaORM connection not initialized")
}

/// 在独立 runtime 上阻塞等待一个 Future。
pub fn block_on<F: Future>(f: F) -> F::Output {
    ASYNC_RT
        .get()
        .expect("data_plugin: async runtime not initialized")
        .block_on(f)
}
