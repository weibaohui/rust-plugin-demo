/*!
data_plugin — 数据 CRUD 插件示例。

演示：
- 使用宿主数据库能力做增删改查
- 嵌入 React UI 展示数据表格
- 完整的生命周期：on_install 建表，on_uninstall 删表
- 配置 cron 定时生成示例数据
*/

mod plugin;
mod trait_impl;
mod model;
mod handler;
mod service;
mod metadata;
mod routes;
mod db;

use plugkit::plugin::PluginRegistrar;
use std::sync::Arc;

/// 编译期嵌入的 React UI 产物。
pub static UI_DIST: include_dir::Dir<'static> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

pub(crate) const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "::",
    module_path!(),
    "::",
    "DataPlugin",
);

/// FFI 注册入口 — 宿主加载 dylib 后调用此函数注册插件。
#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar) {
    // 初始化 SeaORM 连接（与宿主共用同一 SQLite 文件）
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let conn = sea_orm::Database::connect("sqlite://plugkit.db?mode=rwc")
            .await
            .expect("SeaORM DB connection failed");
        crate::db::init_connection(conn);
    });

    registrar.register(Arc::new(
        plugin::DataPlugin::new(PLUGIN_ID)
            .with_ui_dist("data_plugin/ui", &UI_DIST),
    ));
}
