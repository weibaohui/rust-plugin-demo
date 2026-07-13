//! 数据模型 — 数据表 `data_items` 对应的 ORM 实体。

use serde::{Deserialize, Serialize};

/// 数据记录实体，映射到表 `data_items`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: String,
}

impl Item {
    /// 构造一条新记录（id 由数据库自增，此处填 0）。
    pub fn new(title: &str, content: &str, created_at: &str) -> Self {
        Self {
            id: 0,
            title: title.to_string(),
            content: content.to_string(),
            created_at: created_at.to_string(),
        }
    }
}
