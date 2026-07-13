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
mod handlers;
mod metadata;
mod routes;

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
    registrar.register(Arc::new(
        plugin::DataPlugin::new(PLUGIN_ID)
            .with_ui_dist("data_plugin/ui", &UI_DIST),
    ));
}
