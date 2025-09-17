use std::sync::{Arc, Mutex};
use actix_web::{HttpResponse, Responder, web, HttpRequest};
use log::logger;
use crate::db::{DbPool, User, CreateUserRequest, UpdateUserRequest, ApiResponse, get_connection_or_return_error};
use mysql::prelude::Queryable; 
use serde_json;
use serde_json::json;
use crate::middleware::{JsonLogger, LogLevel};
// 导入rbatis_routes模块以使用其中的方法
use crate::routes::{rbatis_routes,auth_routes,cache_routes,redis_routes};
// 导入其他模块需要的类型
use serde::{Deserialize, Serialize};
use std::time::Duration;
use log::{info, error};
use crate::middleware::JwtMiddleware;
use crate::cache::Cache;
use std::sync::Arc as StdArc;
use crate::redis_pool;
use crate::rbatis_pool::RBATIS_POOL;
use rbatis::crud;
use rbs::value::Value;

// 登录请求结构体
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub phone: String, 
    pub password: String,
}

// 登录响应结构体
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String, 
    pub token_type: String, 
    pub expires_in: u64, 
    pub user_id: u64, 
    pub phone: String, 
}

// 注册请求结构体
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub phone: String, 
    pub password: String, 
    pub name: Option<String>, 
}

// 注册响应结构体
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub success: bool, 
    pub user_id: u64, 
    pub phone: String, 
}

// 设置缓存请求结构
#[derive(Debug, Deserialize)]
pub struct SetCacheRequest {
    key: String,
    value: String,
    ttl: Option<u64>,
}

// 缓存状态响应结构
#[derive(Debug, Serialize)]
pub struct CacheStatusResponse {
    status: String,
    item_count: usize,
    message: String,
}

// Redis操作请求体结构
#[derive(Debug, Deserialize)]
pub struct RedisSetRequest {
    key: String,
    value: String,
    expiry_seconds: Option<u64>
}

// Redis操作响应结构
#[derive(Debug, Serialize)]
pub struct RedisResponse {
    status: String,
    message: String,
    data: Option<String>
}

// Rbatis User 结构体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RbatisUser {
    pub id: Option<u64>,
    pub phone: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<u8>,
    pub create_time: Option<u32>,
    pub first_change: Option<u8>,
    pub is_business: Option<u8>,
    pub is_ban: Option<u8>,
}

// 为 RbatisUser 自动生成 CRUD 方法
crud!(RbatisUser{});

// 健康检查路由处理函数
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "UP",
        "version": "1.0.0",
        "message": "Service is running normally"
    }))
}

// 创建用户处理函数
pub async fn create_user(
    pool: web::Data<DbPool>,
    user: web::Json<CreateUserRequest>,
) -> Result<impl Responder, actix_web::Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    let _affected_rows = conn.exec_drop(
        "INSERT INTO user (phone, name, avatar, createTime, firstChange, isBusiness, isBan) VALUES (?, ?, ?, UNIX_TIMESTAMP(), 1, 0, 0)",
        (&user.phone, &user.name, &user.avatar.unwrap_or(0))
    ).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to insert user: {}", e))
    })?;
    
    let response = ApiResponse {
        message: "User created successfully".to_string(),
        status: "success".to_string(),
        data: None,
    };
    Ok(HttpResponse::Ok().json(response))
}

// 获取所有用户处理函数
pub async fn get_users(
    pool: web::Data<DbPool>,
) -> Result<impl Responder, actix_web::Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    let users: Vec<User> = conn.query("SELECT * FROM user").map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get users: {}", e))
    })?;
    
    let response = ApiResponse {
        message: "Users fetched successfully".to_string(),
        status: "success".to_string(),
        data: Some(serde_json::to_value(users).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to serialize users: {}", e))
        })?),
    };
    
    Ok(HttpResponse::Ok().json(response))
}

// 根据ID获取用户处理函数
pub async fn get_user_by_id(
    pool: web::Data<DbPool>,
    user_id: web::Path<u64>,
) -> Result<impl Responder, actix_web::Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    let user: Option<User> = conn.exec_first(
        "SELECT * FROM user WHERE id = ?",
        (user_id.into_inner(),)
    ).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get user: {}", e))
    })?;
    
    match user {
        Some(found_user) => {
            let response = ApiResponse {
                message: "User fetched successfully".to_string(),
                status: "success".to_string(),
                data: Some(serde_json::to_value(found_user).map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!("Failed to serialize user: {}", e))
                })?),
            };
            Ok(HttpResponse::Ok().json(response))
        },
        None => {
            let response = ApiResponse {
                message: "User not found".to_string(),
                status: "error".to_string(),
                data: None,
            };
            Ok(HttpResponse::NotFound().json(response))
        },
    }
}

// 更新用户处理函数
pub async fn update_user(
    pool: web::Data<DbPool>,
    user_id: web::Path<u64>,
    update_data: web::Json<UpdateUserRequest>,
) -> Result<impl Responder, actix_web::Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    // 提取用户ID值
    let user_id_value = user_id.into_inner();
    
    // 检查用户是否存在
    let existing_user: Option<User> = conn.exec_first(
        "SELECT * FROM user WHERE id = ?",
        (user_id_value,)
    ).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to check user: {}", e))
    })?;
    
    if existing_user.is_none() {
        let response = ApiResponse {
            message: "User not found".to_string(),
            status: "error".to_string(),
            data: None,
        };
        return Ok(HttpResponse::NotFound().json(response));
    }
    
    // 执行更新
    let _affected_rows = conn.exec_drop(
        "UPDATE user SET name = ?, avatar = ?, firstChange = ?, isBusiness = ?, isBan = ? WHERE id = ?",
        (&update_data.name, &update_data.avatar.unwrap_or(0), &update_data.first_change.unwrap_or(0), &update_data.is_business.unwrap_or(0), &update_data.is_ban.unwrap_or(0), user_id_value)
    ).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to update user: {}", e))
    })?;
    
    let response = ApiResponse {
        message: "User updated successfully".to_string(),
        status: "success".to_string(),
        data: None,
    };
    Ok(HttpResponse::Ok().json(response))
}

// 删除用户处理函数
pub async fn delete_user(
    pool: web::Data<DbPool>,
    user_id: web::Path<u64>,
) -> Result<impl Responder, actix_web::Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    // 提取用户ID值
    let user_id_value = user_id.into_inner();
    
    // 检查用户是否存在
    let existing_user: Option<User> = conn.exec_first(
        "SELECT * FROM user WHERE id = ?",
        (user_id_value,)
    ).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to check user: {}", e))
    })?;
    
    if existing_user.is_none() {
        let response = ApiResponse {
            message: "User not found".to_string(),
            status: "error".to_string(),
            data: None,
        };
        return Ok(HttpResponse::NotFound().json(response));
    }
    
    // 执行删除
    let _affected_rows = conn.exec_drop(
        "DELETE FROM user WHERE id = ?",
        (user_id_value,)
    ).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to delete user: {}", e))
    })?;
    
    let response = ApiResponse {
        message: "User deleted successfully".to_string(),
        status: "success".to_string(),
        data: None,
    };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn json_logger(
    req: HttpRequest,
    logger: web::Data<Arc<Mutex<JsonLogger>>>
) -> impl Responder {
    // 从路径中获取信息
    let path = req.path().to_string();
    let method = req.method().to_string();
    print!("{},{}",path,method);
    //尝试获取并锁定JSON日志器
    if let Ok(mut logger_guard) = logger.lock() {
        // 记录基本信息日志
        let _ = logger_guard.log_with_data(
            LogLevel::INFO,
            "JSON日志测试: 基本信息",
            json!({"path": path, "method": method})
        );

        // 记录警告级别的日志
        let _ = logger_guard.log_with_data(
            LogLevel::WARNING,
            "JSON日志测试: 警告信息",
            json!({"warning_type": "test_warning", "severity": "low"})
        );

        // 记录错误级别的日志
        let _ = logger_guard.log_with_data(
            LogLevel::ERROR,
            "JSON日志测试: 错误信息",
            json!({"error_type": "test_error", "code": "TEST_001", "description": "这是一个测试错误"})
        );
    }

    // 返回成功响应
    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "JSON日志记录成功，请查看日志文件",
        "log_file": format!("logs/app_{}.log", chrono::Local::now().format("%Y%m%d"))
    }))
}

// 配置主要路由
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health_check))
            .route("/users", web::post().to(create_user))
            .route("/users", web::get().to(get_users))
            .route("/users/{id}", web::get().to(get_user_by_id))
            .route("/users/{id}", web::put().to(update_user))
            .route("/users/{id}", web::delete().to(delete_user))
            .route("/logger", web::get().to(json_logger))
    ).service(
        web::scope("/rbatis")
            .route("/health", web::get().to(rbatis_routes::rbatis_health_check))
            .route("/users", web::get().to(rbatis_routes::rbatis_get_users))
            .route("/users", web::post().to(rbatis_routes::rbatis_create_user))
            .route("/users/{id}", web::get().to(rbatis_routes::rbatis_get_user_by_id))
            .route("/users/{id}", web::put().to(rbatis_routes::rbatis_update_user))
            .route("/users/{id}", web::delete().to(rbatis_routes::rbatis_delete_user))
    )
        .service(
        web::scope("/auth")
            .route("/login", web::post().to(auth_routes::login))
            .route("/register", web::post().to(auth_routes::register))
            .route("/me", web::get().to(auth_routes::get_current_user))
    ).service(
        web::scope("/cache")
            .route("/set", web::post().to(cache_routes::set_cache))
            .route("/get/{key}", web::get().to(cache_routes::get_cache))
            .route("/delete/{key}", web::delete().to(cache_routes::delete_cache))
            .route("/status", web::get().to(cache_routes::get_cache_status))
            .route("/clear", web::delete().to(cache_routes::clear_cache))
    ).service(
        web::scope("/redis")
            .route("/{key}", web::get().to(redis_routes::redis_get))
            .route("/set", web::post().to(redis_routes::redis_set))
    );
}