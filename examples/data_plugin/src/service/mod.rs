//! 业务逻辑层 — 调用 repository，基于 `Item` 模型操作。

use crate::model::Item;
use crate::repository;
use plugkit::database::DatabaseExt;

/// 获取全部记录。
pub fn list_items(db: &dyn DatabaseExt) -> Result<Vec<Item>, String> {
    repository::find_all(db)
}

/// 按 ID 获取记录。
pub fn get_item(db: &dyn DatabaseExt, id: i64) -> Result<Option<Item>, String> {
    repository::find_by_id(db, id)
}

/// 创建记录。
pub fn create_item(db: &dyn DatabaseExt, title: &str, content: &str) -> Result<Item, String> {
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
) -> Result<Item, String> {
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
