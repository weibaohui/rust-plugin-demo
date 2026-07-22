//! 数据模型 — 基于 SeaORM 的 ORM 实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 数据记录实体，映射到表 `data_items`。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "data_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_by: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
