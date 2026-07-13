//! 数据访问层 — 基于 `Item` 模型的 ORM 式 CRUD 操作。

use crate::model::Item;
use plugkit::database::{DatabaseExt, DbValue};

const TABLE: &str = "data_items";

/// 查询全部记录。
pub fn find_all(db: &dyn DatabaseExt) -> Result<Vec<Item>, String> {
    let rows = db
        .query(&format!(
            "SELECT id, title, content, created_at FROM {} ORDER BY id DESC",
            TABLE
        ))
        .map_err(|e| format!("查询失败: {}", e))?;
    Ok(rows.iter().map(|r| row_to_item(r)).collect())
}

/// 按 ID 查询单条记录。
pub fn find_by_id(db: &dyn DatabaseExt, id: i64) -> Result<Option<Item>, String> {
    let rows = db
        .query(&format!(
            "SELECT id, title, content, created_at FROM {} WHERE id = ?1 LIMIT 1",
            TABLE
        ))
        .map_err(|e| format!("查询失败: {}", e))?;
    Ok(rows.first().map(|r| row_to_item(r)))
}

/// 插入一条记录，返回创建后的完整对象（含数据库生成的 id）。
pub fn insert(db: &dyn DatabaseExt, title: &str, content: &str, created_at: &str) -> Result<Item, String> {
    db.execute_with(
        &format!(
            "INSERT INTO {} (title, content, created_at) VALUES (?1, ?2, ?3)",
            TABLE
        ),
        &[
            DbValue::Text(title.to_string()),
            DbValue::Text(content.to_string()),
            DbValue::Text(created_at.to_string()),
        ],
    )
    .map_err(|e| format!("插入失败: {}", e))?;

    // 取回自增 id
    let id = db
        .query("SELECT last_insert_rowid()")
        .ok()
        .and_then(|rows| {
            rows.first()
                .and_then(|r| r.first())
                .and_then(|v| match v {
                    DbValue::Int(n) => Some(*n),
                    _ => None,
                })
        })
        .unwrap_or(0);

    Ok(Item {
        id,
        title: title.to_string(),
        content: content.to_string(),
        created_at: created_at.to_string(),
    })
}

/// 更新一条记录，返回更新后的完整对象。
pub fn update(db: &dyn DatabaseExt, id: i64, title: &str, content: &str) -> Result<Item, String> {
    db.execute_with(
        &format!("UPDATE {} SET title = ?1, content = ?2 WHERE id = ?3", TABLE),
        &[
            DbValue::Text(title.to_string()),
            DbValue::Text(content.to_string()),
            DbValue::Int(id),
        ],
    )
    .map_err(|e| format!("更新失败: {}", e))?;

    Ok(Item {
        id,
        title: title.to_string(),
        content: content.to_string(),
        created_at: String::new(), // 不修改 created_at
    })
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

// ---- 内部辅助 ----

fn row_to_item(row: &[DbValue]) -> Item {
    Item {
        id: row.get(0).and_then(int_val).unwrap_or(0),
        title: row.get(1).and_then(str_val).unwrap_or_default(),
        content: row.get(2).and_then(str_val).unwrap_or_default(),
        created_at: row.get(3).and_then(str_val).unwrap_or_default(),
    }
}

fn int_val(v: &DbValue) -> Option<i64> {
    match v {
        DbValue::Int(n) => Some(*n),
        _ => None,
    }
}

fn str_val(v: &DbValue) -> Option<String> {
    match v {
        DbValue::Text(s) => Some(s.clone()),
        _ => None,
    }
}
