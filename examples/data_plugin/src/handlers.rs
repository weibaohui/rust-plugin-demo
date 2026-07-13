//! HTTP 路由 handler 函数 — 一个功能一个方法。

use plugkit::database::DatabaseExt;
use plugkit::plugin::Plugin;
use http::StatusCode;

/// GET /items — 列出所有数据记录。
pub fn handle_list_items(
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
pub fn handle_create_item(
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
        Err(e) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("插入失败: {}", e))
        }
    }
}

/// PUT /items — 更新一条数据记录（ID 从 URI 路径中提取）。
pub fn handle_update_item(
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
        Err(e) => {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("更新失败: {}", e))
        }
    }
}

/// DELETE /items — 删除一条数据记录（ID 从 URI 路径中提取）。
pub fn handle_delete_item(
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
