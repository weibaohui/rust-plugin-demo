//! Plugin trait 实现 — 生命周期钩子。
//!
//! 元数据委托给 [`metadata`]，路由委托给 [`routes`]，
//! handler 实现在 [`handlers`]。

use crate::metadata;
use crate::routes;
use crate::plugin::DataPlugin;
use plugkit::database::DatabaseExt;
use plugkit::metadata::{CronSpec, PluginMetadata};
use plugkit::plugin::{Plugin, PluginRoute};
use include_dir::Dir;

impl Plugin for DataPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn metadata(&self) -> PluginMetadata {
        metadata::metadata()
    }

    // ---- 生命周期 ----

    fn on_load(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] ✅ loaded");
        Ok(())
    }
    fn on_unload(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] 🔄 unloaded");
        Ok(())
    }
    fn on_upgrade(&self, db: &dyn DatabaseExt, from_version: &str) -> plugkit::error::Result<()> {
        eprintln!(
            "[data_plugin] 🔄 on_upgrade — {} → {}",
            from_version,
            env!("CARGO_PKG_VERSION")
        );
        // 版本迁移：按 from_version 增量执行，重复列自动跳过（幂等）
        let migrate = |sql: &str| -> plugkit::error::Result<()> {
            match db.execute(sql) {
                Ok(_) => Ok(()),
                Err(e) if e.to_string().contains("duplicate column") => {
                    eprintln!("[data_plugin] ⏭️ 跳过已存在的列");
                    Ok(())
                }
                Err(e) => Err(e),
            }
        };
        if from_version == "0.1.0" || from_version == "0.0.0" {
            migrate("ALTER TABLE data_items ADD COLUMN created_by TEXT NOT NULL DEFAULT ''")?;
            migrate("ALTER TABLE data_items ADD COLUMN updated_by TEXT NOT NULL DEFAULT ''")?;
        }
        if from_version == "0.2.0" || from_version == "0.1.0" || from_version == "0.0.0" {
            migrate("ALTER TABLE data_items ADD COLUMN remark TEXT NOT NULL DEFAULT ''")?;
        }
        eprintln!("[data_plugin] ✅ 迁移完成");
        Ok(())
    }
    fn on_install(&self, db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        db.validate_table_name("data_items")?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS data_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            created_by TEXT NOT NULL DEFAULT '',
            updated_by TEXT NOT NULL DEFAULT '',
            remark TEXT NOT NULL DEFAULT ''
        )",
        )?;
        eprintln!("[data_plugin] 📦 on_install — 表 data_items 已创建");
        Ok(())
    }
    fn on_uninstall(&self, db: &dyn DatabaseExt, keep_data: bool) -> plugkit::error::Result<()> {
        if !keep_data {
            db.drop_table("data_items")?;
            eprintln!("[data_plugin] 🗑️ on_uninstall — 表 data_items 已删除");
        }
        Ok(())
    }
    fn on_enable(&self) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] ▶️ enabled");
        Ok(())
    }
    fn on_disable(&self) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] ⏸️ disabled");
        Ok(())
    }
    fn on_start(&self) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] 🚀 started — cron 定时生成数据已注册");
        Ok(())
    }
    fn on_stop(&self) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] 🛑 stopped");
        Ok(())
    }
    fn on_cron(&self, name: &str) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] ⏰ cron tick: {}", name);
        Ok(())
    }

    // ---- 定时任务 ----

    fn cron_specs(&self) -> Vec<CronSpec> {
        metadata::cron_specs()
    }

    // ---- UI ----

    fn ui_base_dir(&self) -> Option<&str> {
        self.ui_base_dir.as_deref()
    }
    fn has_ui(&self) -> bool {
        self.ui_dist.is_some()
    }
    fn ui_dist(&self) -> Option<&'static Dir<'static>> {
        self.ui_dist
    }

    // ---- 路由 ----

    fn routes(&self) -> Vec<PluginRoute> {
        routes::routes()
    }
}
