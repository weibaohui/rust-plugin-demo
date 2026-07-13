//! HTTP handler 层 — 解析请求，调用 service，构建 HTTP 响应。

use crate::service;
use plugkit::database::DatabaseExt;
use plugkit::plugin::Plugin;
use http::StatusCode;

/// GET /items — 列出所有数据记录。
pub fn handle_list_items(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    _req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    match service::list_items(db) {
        Ok(items) => json_response(StatusCode::OK, &serde_json::to_value(items).unwrap()),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// POST /items — 创建一条数据记录。
pub fn handle_create_item(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let (title, content) = match parse_body(req.body()) {
        Some(v) => v,
        None => return error_response(StatusCode::BAD_REQUEST, "无效的请求体"),
    };
    match service::create_item(db, &title, &content) {
        Ok(item) => json_response(StatusCode::CREATED, &serde_json::to_value(item).unwrap()),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// PUT /items — 更新一条数据记录（ID 从 URI 路径中提取）。
pub fn handle_update_item(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let id = match service::parse_id(req.uri().path()) {
        Some(id) => id,
        None => return error_response(StatusCode::BAD_REQUEST, "无效的ID"),
    };
    let (title, content) = match parse_body(req.body()) {
        Some(v) => v,
        None => return error_response(StatusCode::BAD_REQUEST, "无效的请求体"),
    };
    match service::update_item(db, id, &title, &content) {
        Ok(()) => json_response(StatusCode::OK, &serde_json::json!({"message": "更新成功"})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// DELETE /items — 删除一条数据记录（ID 从 URI 路径中提取）。
pub fn handle_delete_item(
    _plugin: &dyn Plugin,
    db: &dyn DatabaseExt,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let id = match service::parse_id(req.uri().path()) {
        Some(id) => id,
        None => return error_response(StatusCode::BAD_REQUEST, "无效的ID"),
    };
    match service::delete_item(db, id) {
        Ok(()) => json_response(StatusCode::OK, &serde_json::json!({"message": "删除成功"})),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

// ------------------------------------------------------------------------------------------------
// HTTP 辅助
// ------------------------------------------------------------------------------------------------

fn parse_body(body: &[u8]) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let title = v.get("title").and_then(|s| s.as_str()).unwrap_or("").to_string();
    let content = v.get("content").and_then(|s| s.as_str()).unwrap_or("").to_string();
    Some((title, content))
}

fn json_response(status: StatusCode, body: &serde_json::Value) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(body).unwrap())
        .unwrap()
}

fn error_response(status: StatusCode, msg: &str) -> http::Response<Vec<u8>> {
    let body = serde_json::json!({"error": msg});
    json_response(status, &body)
}
