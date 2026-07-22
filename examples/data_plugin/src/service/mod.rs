//! 业务逻辑层 — 基于 SeaORM Entity 的 CRUD。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::model;

pub type DbConn = DatabaseConnection;

/// 获取全部记录。
pub async fn list_items(conn: &DbConn) -> Result<Vec<model::Model>, String> {
    model::Entity::find()
        .order_by_desc(model::Column::Id)
        .all(conn)
        .await
        .map_err(|e| format!("查询失败: {}", e))
}

/// 按 ID 获取记录。
pub async fn get_item(conn: &DbConn, id: i64) -> Result<Option<model::Model>, String> {
    model::Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(|e| format!("查询失败: {}", e))
}

/// 创建记录。
pub async fn create_item(conn: &DbConn, title: &str, content: &str, created_by: &str) -> Result<model::Model, String> {
    let now = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let item = model::ActiveModel {
        title: Set(title.to_string()),
        content: Set(content.to_string()),
        created_at: Set(now),
        created_by: Set(created_by.to_string()),
        updated_by: Set(created_by.to_string()),
        ..Default::default()
    };

    item.insert(conn)
        .await
        .map_err(|e| format!("插入失败: {}", e))
}

/// 更新记录。
pub async fn update_item(
    conn: &DbConn,
    id: i64,
    title: &str,
    content: &str,
    updated_by: &str,
) -> Result<model::Model, String> {
    let item = model::ActiveModel {
        id: Set(id),
        title: Set(title.to_string()),
        content: Set(content.to_string()),
        updated_by: Set(updated_by.to_string()),
        ..Default::default()
    };

    item.update(conn)
        .await
        .map_err(|e| format!("更新失败: {}", e))
}

/// 删除记录。
pub async fn delete_item(conn: &DbConn, id: i64) -> Result<(), String> {
    model::Entity::delete_by_id(id)
        .exec(conn)
        .await
        .map(|_| ())
        .map_err(|e| format!("删除失败: {}", e))
}

/// 从 "/items/42" 形式的路径中提取 id。
pub fn parse_id(path: &str) -> Option<i64> {
    path.strip_prefix("/items/")?.parse().ok()
}
