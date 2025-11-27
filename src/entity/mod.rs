// src/entity/mod.rs
pub mod users;

// 统一导出常用类型
pub use users::{ActiveModel, Entity, Model};
