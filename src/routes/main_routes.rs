use actix_web::{HttpResponse, Responder, web};
use crate::db::{DbPool, User, CreateUserRequest, UpdateUserRequest, ApiResponse, get_connection_or_return_error}; 
use mysql::prelude::Queryable; 
use serde_json;

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
    );
}