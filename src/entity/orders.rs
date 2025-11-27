// src/entity/orders.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "order")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub uid: u64,
    #[sea_orm(column_name = "order")]
    pub order_id: String,
    pub read: u8,
    pub goods: u32,
    #[sea_orm(column_name = "createAt")]
    pub create_at: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::users::Entity", from = "Column::Uid", to = "super::users::Column::Id")]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
