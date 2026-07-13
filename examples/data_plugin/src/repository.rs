//! 数据访问层 — 纯 SQL 操作，不关心 HTTP 和业务逻辑。

use plugkit::database::{DatabaseExt, DbValue};

const TABLE: &str = "data_items";

/// 查询全部记录。
pub fn find_all(db: &dyn DatabaseExt) -> Result<Vec<Vec<DbValue>>, String> {
    db.query(&format!(
        "SELECT id, title, content, created_at FROM {} ORDER BY id DESC",
        TABLE
    ))
    .map_err(|e| format!("查询失败: {}", e))
}

/// 插入一条记录。
pub fn insert(db: &dyn DatabaseExt, title: &str, content: &str, created_at: &str) -> Result<(), String> {
    db.execute_with(
        &format!("INSERT INTO {} (title, content, created_at) VALUES (?1, ?2, ?3)", TABLE),
        &[
            DbValue::Text(title.to_string()),
            DbValue::Text(content.to_string()),
            DbValue::Text(created_at.to_string()),
        ],
    )
    .map(|_| ())
    .map_err(|e| format!("插入失败: {}", e))
}

/// 更新一条记录。
pub fn update(db: &dyn DatabaseExt, id: i64, title: &str, content: &str) -> Result<(), String> {
    db.execute_with(
        &format!("UPDATE {} SET title = ?1, content = ?2 WHERE id = ?3", TABLE),
        &[
            DbValue::Text(title.to_string()),
            DbValue::Text(content.to_string()),
            DbValue::Int(id),
        ],
    )
    .map(|_| ())
    .map_err(|e| format!("更新失败: {}", e))
}

/// 删除一条记录。
pub fn delete(db: &dyn DatabaseExt, id: i64) -> Result<(), String> {
    db.execute_with(
        &format!("DELETE FROM {} WHERE id = ?1", TABLE),
        &[DbValue::Int(id)],
    )
    .map(|_| ())
    .map_err(|e| format!("删除失败: {}", e))
}

/// DbValue → serde_json::Value
pub fn row_to_item(row: &[DbValue]) -> serde_json::Value {
    serde_json::json!({
        "id": val(row.get(0)),
        "title": val(row.get(1)),
        "content": val(row.get(2)),
        "created_at": val(row.get(3)),
    })
}

fn val(v: Option<&DbValue>) -> serde_json::Value {
    match v {
        Some(DbValue::Null) => serde_json::Value::Null,
        Some(DbValue::Int(n)) => serde_json::json!(n),
        Some(DbValue::Real(f)) => serde_json::json!(f),
        Some(DbValue::Text(s)) => serde_json::json!(s),
        Some(DbValue::Blob(_)) => serde_json::Value::Null,
        None => serde_json::Value::Null,
    }
}
