//! 数据模型 — 映射到表 `data_items` 的 ORM 风格实体。
//!
//! 所有持久化方法直接在 `Item` 上提供，无需外层手写 SQL。

use plugkit::database::{DatabaseExt, DbValue};
use serde::{Deserialize, Serialize};

const TABLE: &str = "data_items";

/// 数据记录实体，自带增删改查能力。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: String,
}

// ---- 主动查询 ----

impl Item {
    /// 查询全部记录。
    pub fn find_all(db: &dyn DatabaseExt) -> Result<Vec<Self>, String> {
        let rows = db
            .query(&format!("SELECT * FROM {} ORDER BY id DESC", TABLE))
            .map_err(|e| format!("查询失败: {}", e))?;
        Ok(rows.iter().map(|r| Self::from_row(r)).collect())
    }

    /// 按 ID 查询单条记录。
    pub fn find_by_id(db: &dyn DatabaseExt, id: i64) -> Result<Option<Self>, String> {
        let rows = db
            .query(&format!(
                "SELECT * FROM {} WHERE id = ?1 LIMIT 1",
                TABLE
            ))
            .map_err(|e| format!("查询失败: {}", e))?;
        Ok(rows.first().map(|r| Self::from_row(r)))
    }
}

// ---- 持久化 ----

impl Item {
    /// 插入新记录，返回含数据库自增 id 的完整对象。
    pub fn insert(&self, db: &dyn DatabaseExt) -> Result<Self, String> {
        db.execute_with(
            &format!(
                "INSERT INTO {} (title, content, created_at) VALUES (?1, ?2, ?3)",
                TABLE
            ),
            &[
                DbValue::Text(self.title.clone()),
                DbValue::Text(self.content.clone()),
                DbValue::Text(self.created_at.clone()),
            ],
        )
        .map_err(|e| format!("插入失败: {}", e))?;

        let id = Self::last_insert_id(db);
        Ok(Self { id, ..self.clone() })
    }

    /// 更新当前记录（按 `self.id` 匹配）。
    pub fn update(&self, db: &dyn DatabaseExt) -> Result<(), String> {
        db.execute_with(
            &format!("UPDATE {} SET title = ?1, content = ?2 WHERE id = ?3", TABLE),
            &[
                DbValue::Text(self.title.clone()),
                DbValue::Text(self.content.clone()),
                DbValue::Int(self.id),
            ],
        )
        .map(|_| ())
        .map_err(|e| format!("更新失败: {}", e))
    }

    /// 删除当前记录（按 `self.id` 匹配）。
    pub fn delete(&self, db: &dyn DatabaseExt) -> Result<(), String> {
        db.execute_with(
            &format!("DELETE FROM {} WHERE id = ?1", TABLE),
            &[DbValue::Int(self.id)],
        )
        .map(|_| ())
        .map_err(|e| format!("删除失败: {}", e))
    }
}

// ---- 构造 ----

impl Item {
    pub fn new(title: &str, content: &str, created_at: &str) -> Self {
        Self {
            id: 0,
            title: title.to_string(),
            content: content.to_string(),
            created_at: created_at.to_string(),
        }
    }
}

// ---- 内部 ----

impl Item {
    fn from_row(row: &[DbValue]) -> Self {
        Self {
            id: row.get(0).and_then(int_val).unwrap_or(0),
            title: row.get(1).and_then(str_val).unwrap_or_default(),
            content: row.get(2).and_then(str_val).unwrap_or_default(),
            created_at: row.get(3).and_then(str_val).unwrap_or_default(),
        }
    }

    fn last_insert_id(db: &dyn DatabaseExt) -> i64 {
        db.query("SELECT last_insert_rowid()")
            .ok()
            .and_then(|rows| rows.first().and_then(|r| r.first()).and_then(int_val))
            .unwrap_or(0)
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
