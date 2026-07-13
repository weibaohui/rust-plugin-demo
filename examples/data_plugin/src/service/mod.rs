//! 业务逻辑层 — 调用 repository，处理校验与转换。

use crate::repository;
use plugkit::database::DatabaseExt;

/// 获取全部记录（转为 JSON 列表）。
pub fn list_items(db: &dyn DatabaseExt) -> Result<Vec<serde_json::Value>, String> {
    let rows = repository::find_all(db)?;
    let items: Vec<serde_json::Value> = rows.iter().map(|r| repository::row_to_item(r)).collect();
    Ok(items)
}

/// 创建记录。
pub fn create_item(
    db: &dyn DatabaseExt,
    title: &str,
    content: &str,
) -> Result<(), String> {
    let now = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    repository::insert(db, title, content, &now)
}

/// 更新记录。
pub fn update_item(
    db: &dyn DatabaseExt,
    id: i64,
    title: &str,
    content: &str,
) -> Result<(), String> {
    repository::update(db, id, title, content)
}

/// 删除记录。
pub fn delete_item(db: &dyn DatabaseExt, id: i64) -> Result<(), String> {
    repository::delete(db, id)
}

/// 从 "/items/42" 形式的路径中提取 id。
pub fn parse_id(path: &str) -> Option<i64> {
    path.strip_prefix("/items/")?.parse().ok()
}
