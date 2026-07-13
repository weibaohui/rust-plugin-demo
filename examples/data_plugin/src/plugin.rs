use crate::handlers;
use plugkit::database::DatabaseExt;
use plugkit::host::HostContext;
use plugkit::metadata::{CronSpec, PluginMetadata};
use plugkit::plugin::{Plugin, PluginRoute};
use http::Method;
use include_dir::Dir;
use serde::{Deserialize, Serialize};

/// 数据记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DataRecord {
    pub(crate) id: i64,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) created_at: String,
}

#[derive(Debug)]
pub struct DataPlugin {
    id: String,
    ui_base_dir: Option<String>,
    ui_dist: Option<&'static Dir<'static>>,
}

impl DataPlugin {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ui_base_dir: None,
            ui_dist: None,
        }
    }

    pub fn with_ui_dist(mut self, base_dir: &str, dist: &'static Dir<'static>) -> Self {
        self.ui_base_dir = Some(base_dir.to_string());
        self.ui_dist = Some(dist);
        self
    }

    /// 生成一条模拟数据。
    pub(crate) fn generate_record(
        &self,
        ctx: &dyn HostContext,
        db: &dyn DatabaseExt,
    ) -> DataRecord {
        use chrono::Local;
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let id = {
            let rows = db
                .query("SELECT COALESCE(MAX(id), 0) + 1 AS next_id FROM data_items")
                .unwrap_or_default();
            rows.first()
                .and_then(|r| r.first())
                .and_then(|v| match v {
                    plugkit::database::DbValue::Int(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(1)
        };
        DataRecord {
            id,
            title: format!("记录 #{} — 来自 {}", id, ctx.server_name()),
            content: format!("这是由 data_plugin 在 {} 自动生成的示例数据", now),
            created_at: now,
        }
    }
}

impl Plugin for DataPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn metadata(&self) -> PluginMetadata {
        use plugkit::metadata::PluginMenu;
        PluginMetadata::new(&self.id, "Data Plugin", env!("CARGO_PKG_VERSION"))
            .with_icon("🗄️")
            .with_description("数据 CRUD 插件 — 演示数据库操作、UI 数据表格、cron 定时任务")
            .with_author("plugkit <plugkit@example.com>")
            .with_license("MIT")
            .with_tables_owned(vec!["data_items".to_string()])
            .with_menus(vec![PluginMenu {
                key: "data_panel".into(),
                title: "数据管理".into(),
                icon: Some("🗄️".into()),
                route: Some(format!("/plugin/{}", super::PLUGIN_ID)),
                order: 200,
                children: vec![],
            }])
    }

    fn on_load(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] ✅ loaded");
        Ok(())
    }
    fn on_unload(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[data_plugin] 🔄 unloaded");
        Ok(())
    }
    fn on_install(&self, db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        db.validate_table_name("data_items")?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS data_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
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
    fn cron_specs(&self) -> Vec<CronSpec> {
        vec![CronSpec {
            name: "generate_data".to_string(),
            interval_secs: 60,
        }]
    }
    fn ui_base_dir(&self) -> Option<&str> {
        self.ui_base_dir.as_deref()
    }
    fn has_ui(&self) -> bool {
        self.ui_dist.is_some()
    }
    fn ui_dist(&self) -> Option<&'static Dir<'static>> {
        self.ui_dist
    }

    fn routes(&self) -> Vec<PluginRoute> {
        vec![
            PluginRoute {
                method: Method::GET,
                path: "/items".into(),
                handler: handlers::handle_list_items,
            },
            PluginRoute {
                method: Method::POST,
                path: "/items".into(),
                handler: handlers::handle_create_item,
            },
            PluginRoute {
                method: Method::PUT,
                path: "/items".into(),
                handler: handlers::handle_update_item,
            },
            PluginRoute {
                method: Method::DELETE,
                path: "/items".into(),
                handler: handlers::handle_delete_item,
            },
        ]
    }
}
