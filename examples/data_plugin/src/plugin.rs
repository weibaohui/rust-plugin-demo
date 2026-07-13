use plugkit::database::DatabaseExt;
use plugkit::host::HostContext;
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
    pub(crate) id: String,
    pub(crate) ui_base_dir: Option<String>,
    pub(crate) ui_dist: Option<&'static Dir<'static>>,
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
