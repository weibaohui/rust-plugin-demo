//! 业务逻辑层 — 基于 `Item` 模型的 CRUD。

use crate::model::Item;
use plugkit::database::DatabaseExt;

/// 获取全部记录。
pub fn list_items(db: &dyn DatabaseExt) -> Result<Vec<Item>, String> {
    Item::find_all(db)
}

/// 按 ID 获取记录。
pub fn get_item(db: &dyn DatabaseExt, id: i64) -> Result<Option<Item>, String> {
    Item::find_by_id(db, id)
}

/// 创建记录。
pub fn create_item(db: &dyn DatabaseExt, title: &str, content: &str) -> Result<Item, String> {
    let now = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let item = Item::new(title, content, &now);
    item.insert(db)
}

/// 更新记录。
pub fn update_item(db: &dyn DatabaseExt, id: i64, title: &str, content: &str) -> Result<(), String> {
    let item = Item {
        id,
        title: title.to_string(),
        content: content.to_string(),
        created_at: String::new(),
    };
    item.update(db)
}

/// 删除记录。
pub fn delete_item(db: &dyn DatabaseExt, id: i64) -> Result<(), String> {
    let item = Item {
        id,
        title: String::new(),
        content: String::new(),
        created_at: String::new(),
    };
    item.delete(db)
}

/// 从 "/items/42" 形式的路径中提取 id。
pub fn parse_id(path: &str) -> Option<i64> {
    path.strip_prefix("/items/")?.parse().ok()
}
