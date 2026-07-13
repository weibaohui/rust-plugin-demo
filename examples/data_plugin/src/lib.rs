/*!
data_plugin — 数据 CRUD 插件示例。

演示：
- 使用宿主数据库能力做增删改查
- 嵌入 React UI 展示数据表格
- 完整的生命周期：on_install 建表，on_uninstall 删表
- 配置 cron 定时生成示例数据
*/

use plugkit::database::DatabaseExt;
use plugkit::host::HostContext;
use plugkit::metadata::{CronSpec, PluginMetadata};
use plugkit::plugin::{Plugin, PluginRegistrar, PluginRoute};
use http::{Method, StatusCode};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub static UI_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

/// 数据记录。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataRecord {
    id: i64,
    title: String,
    content: String,
    created_at: String,
}

#[derive(Debug)]
pub struct DataPlugin {
    id: String,
    ui_base_dir: Option<String>,
    ui_dist: Option<&'static Dir<'static>>,
}

impl DataPlugin {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ui_base_dir: None,
            ui_dist: None,
        }
    }

    fn with_ui_dist(mut self, base_dir: &str, dist: &'static Dir<'static>) -> Self {
        self.ui_base_dir = Some(base_dir.to_string());
        self.ui_dist = Some(dist);
        self
    }

    /// 生成一条模拟数据。
    fn generate_record(&self, ctx: &dyn HostContext, db: &dyn DatabaseExt) -> DataRecord {
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
                route: Some(format!("/plugin/{}", PLUGIN_ID)),
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
        db.execute("CREATE TABLE IF NOT EXISTS data_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        )")?;
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
                handler: handle_list_items,
            },
            PluginRoute {
                method: Method::POST,
                path: "/items".into(),
                handler: handle_create_item,
            },
            PluginRoute {
                method: Method::PUT,
                path: "/items".into(),
                handler: handle_update_item,
            },
            PluginRoute {
                method: Method::DELETE,
                path: "/items".into(),
                handler: handle_delete_item,
            },
        ]
    }
}

// ------------------------------------------------------------------------------------------------
// 路由 handler 函数（一个功能一个方法）
// ------------------------------------------------------------------------------------------------

/// GET /items — 列出所有数据记录。
fn handle_list_items(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    _req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    match db.query("SELECT id, title, content, created_at FROM data_items ORDER BY id DESC") {
        Ok(rows) => {
            let items: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "id": to_json_val(row.get(0)),
                        "title": to_json_val(row.get(1)),
                        "content": to_json_val(row.get(2)),
                        "created_at": to_json_val(row.get(3)),
                    })
                })
                .collect();
            json_response(StatusCode::OK, &serde_json::json!(items))
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("查询失败: {}", e)),
    }
}

/// POST /items — 创建一条数据记录。
fn handle_create_item(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let body: serde_json::Value = match serde_json::from_slice(req.body()) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "无效的请求体"),
    };
    let title = body.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("");

    let now = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    use plugkit::database::DbValue;
    match db.execute_with(
        "INSERT INTO data_items (title, content, created_at) VALUES (?1, ?2, ?3)",
        &[
            DbValue::Text(title.to_string()),
            DbValue::Text(content.to_string()),
            DbValue::Text(now),
        ],
    ) {
        Ok(_) => json_response(StatusCode::CREATED, &serde_json::json!({"message": "创建成功"})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("插入失败: {}", e)),
    }
}

/// PUT /items — 更新一条数据记录（ID 从 URI 路径中提取）。
fn handle_update_item(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let id: i64 = match parse_id(req.uri().path()) {
        Some(id) => id,
        None => return error_response(StatusCode::BAD_REQUEST, "无效的ID"),
    };
    let body: serde_json::Value = match serde_json::from_slice(req.body()) {
        Ok(v) => v,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "无效的请求体"),
    };
    let title = body.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("");

    use plugkit::database::DbValue;
    match db.execute_with(
        "UPDATE data_items SET title = ?1, content = ?2 WHERE id = ?3",
        &[
            DbValue::Text(title.to_string()),
            DbValue::Text(content.to_string()),
            DbValue::Int(id),
        ],
    ) {
        Ok(_) => json_response(StatusCode::OK, &serde_json::json!({"message": "更新成功"})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("更新失败: {}", e)),
    }
}

/// DELETE /items — 删除一条数据记录（ID 从 URI 路径中提取）。
fn handle_delete_item(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let id: i64 = match parse_id(req.uri().path()) {
        Some(id) => id,
        None => return error_response(StatusCode::BAD_REQUEST, "无效的ID"),
    };

    use plugkit::database::DbValue;
    match db.execute_with("DELETE FROM data_items WHERE id = ?1", &[DbValue::Int(id)]) {
        Ok(_) => json_response(StatusCode::OK, &serde_json::json!({"message": "删除成功"})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("删除失败: {}", e)),
    }
}

// ------------------------------------------------------------------------------------------------
// 辅助函数
// ------------------------------------------------------------------------------------------------

/// 构建 JSON 成功响应。
fn json_response(status: StatusCode, body: &serde_json::Value) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(body).unwrap())
        .unwrap()
}

/// 构建 JSON 错误响应。
fn error_response(status: StatusCode, msg: &str) -> http::Response<Vec<u8>> {
    let body = serde_json::json!({"error": msg});
    http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
        .unwrap()
}

/// 将 DbValue 转为 serde_json::Value。
fn to_json_val(v: Option<&plugkit::database::DbValue>) -> serde_json::Value {
    match v {
        Some(plugkit::database::DbValue::Null) => serde_json::Value::Null,
        Some(plugkit::database::DbValue::Int(n)) => serde_json::json!(n),
        Some(plugkit::database::DbValue::Real(f)) => serde_json::json!(f),
        Some(plugkit::database::DbValue::Text(s)) => serde_json::json!(s),
        Some(plugkit::database::DbValue::Blob(_)) => serde_json::Value::Null,
        None => serde_json::Value::Null,
    }
}

/// 从 "/items/42" 形式的路径中提取 id。
fn parse_id(path: &str) -> Option<i64> {
    path.strip_prefix("/items/")?.parse().ok()
}

#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar) {
    registrar.register(Arc::new(
        DataPlugin::new(PLUGIN_ID).with_ui_dist("data_plugin/ui", &UI_DIST),
    ));
}

const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "::",
    module_path!(),
    "::",
    "DataPlugin",
);