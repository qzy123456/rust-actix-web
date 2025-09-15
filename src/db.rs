use mysql::*;
use mysql::prelude::*;
use mysql::OptsBuilder;
use r2d2::Pool;
use r2d2_mysql::MySqlConnectionManager;
use std::sync::Arc;
use actix_web::{web, error, Error};
use serde::{Deserialize, Serialize};

// 定义响应数据结构
#[derive(Serialize)]
pub struct ApiResponse {
    pub message: String,
    pub status: String,
    pub data: Option<serde_json::Value>,
}

// 用户数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub phone: String,
    pub name: String,
    pub avatar: u8,
    pub create_time: u32,
    pub first_change: u8,
    pub is_business: u8,
    pub is_ban: u8,
}

// 为User实现FromRow trait
impl FromRow for User {
    fn from_row(row: Row) -> Self {
        Self::from_row_opt(row).unwrap()
    }

    fn from_row_opt(row: Row) -> std::result::Result<Self, mysql::FromRowError> {
        // 使用ok_or_else将Option转换为Result，然后直接返回我们需要的错误类型
        let id: u64 = row.get("id").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let phone: String = row.get("phone").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let name: String = row.get("name").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let avatar: u8 = row.get("avatar").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let create_time: u32 = row.get("createTime").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let first_change: u8 = row.get("firstChange").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let is_business: u8 = row.get("isBusiness").ok_or_else(|| mysql::FromRowError(row.clone()))?;
        let is_ban: u8 = row.get("isBan").ok_or_else(|| mysql::FromRowError(row))?; // 最后一次使用可以移动
        
        Ok(User {
            id,
            phone,
            name,
            avatar,
            create_time,
            first_change,
            is_business,
            is_ban,
        })
    }
}

// 创建用户请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub phone: String,
    pub name: String,
    pub avatar: Option<u8>,
}

// 更新用户请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub avatar: Option<u8>,
    pub first_change: Option<u8>,
    pub is_business: Option<u8>,
    pub is_ban: Option<u8>,
}

// 定义请求数据结构
#[derive(Deserialize)]
pub struct GreetRequest {
    pub name: Option<String>,
}

// 数据库连接池类型别名
type DbPoolInner = Pool<MySqlConnectionManager>;
pub type DbPool = Arc<DbPoolInner>;

// 获取数据库连接的辅助函数 - 直接返回连接或错误响应
// 使用方式: let mut conn = get_connection_or_return_error(&pool).await?;
pub async fn get_connection_or_return_error(
    pool: &web::Data<DbPool>,
) -> Result<r2d2::PooledConnection<MySqlConnectionManager>, Error> {
    pool.get()
        .map_err(|e| {
            error::ErrorInternalServerError(format!("数据库连接失败: {}", e))
        })
}

// 初始化数据库连接池
pub fn init_db_pool() -> Result<DbPool, Box<dyn std::error::Error>> {
    let opts = OptsBuilder::new()
        .ip_or_hostname(Some("172.17.0.185"))
        .tcp_port(3306)
        .user(Some("admin"))
        .pass(Some("b7371d927aec647d"))
        .db_name(Some("grave"));
    let manager = MySqlConnectionManager::new(opts);
    let pool = Pool::builder()
        .build(manager)
        .map_err(|e| format!("Failed to create pool: {}", e))?;
    
    Ok(Arc::new(pool))
}