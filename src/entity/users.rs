// src/entity/users.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// 定义表名
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)] // 指定主键
    pub id: u64,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// 添加 PreloadItems 实现，支持关联查询
impl Related<super::orders::Entity> for Entity {
    fn to() -> RelationDef {
        super::orders::Relation::User.def()
    }
    
    fn via() -> Option<RelationDef> {
        Some(super::orders::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}