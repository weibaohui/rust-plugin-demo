/*!
hello_plugin — 最小可用插件示例。

演示：
- 无 UI、无数据库、无 cron
- 纯生命周期钩子：load → install → enable → start → stop → disable → unload → uninstall
- 依赖 `plugkit` 框架，不依赖任何其他 crate
- 适合初学者理解插件骨架
*/

use plugkit::database::DatabaseExt;
use plugkit::metadata::PluginMetadata;
use plugkit::plugin::{Plugin, PluginRegistrar};
use std::sync::Arc;

/// 插件实例。
#[derive(Debug)]
pub struct HelloPlugin {
    id: String,
}

impl HelloPlugin {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
        }
    }
}

impl Plugin for HelloPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(&self.id, "Hello Plugin", env!("CARGO_PKG_VERSION"))
            .with_icon("👋")
            .with_description("最小可用插件示例 — 展示了完整的生命周期钩子")
            .with_author("plugkit <plugkit@example.com>")
            .with_license("MIT")
    }

    fn on_load(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] ✅ on_load — 库已加载，插件已注册");
        Ok(())
    }
    fn on_unload(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] 🔄 on_unload — 正在卸载");
        Ok(())
    }
    fn on_install(&self, db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] 📦 on_install — 首次安装（无需建表）");
        // hello_plugin 不需要数据库，但演示了 on_install 钩子
        Ok(())
    }
    fn on_uninstall(&self, _db: &dyn DatabaseExt, keep_data: bool) -> plugkit::error::Result<()> {
        eprintln!(
            "[hello_plugin] 🗑️ on_uninstall — keep_data={}",
            keep_data
        );
        Ok(())
    }
    fn on_enable(&self) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] ▶️ on_enable — 插件已启用，菜单可见");
        Ok(())
    }
    fn on_disable(&self) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] ⏸️ on_disable — 插件已禁用，菜单隐藏");
        Ok(())
    }
    fn on_start(&self) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] 🚀 on_start — 后台任务已就绪");
        Ok(())
    }
    fn on_stop(&self) -> plugkit::error::Result<()> {
        eprintln!("[hello_plugin] 🛑 on_stop — 后台任务已停止");
        Ok(())
    }
}

// 注册入口（dylib 符号）
#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar) {
    registrar.register(Arc::new(HelloPlugin::new(PLUGIN_ID)));
}

// 插件标识符
const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "::",
    module_path!(),
    "::",
    "HelloPlugin",
);