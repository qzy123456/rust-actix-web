use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use crate::rbatis_pool::RBATIS_POOL;
use rbatis::crud;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Option<u64>,
    pub phone: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<u8>,
    pub create_time: Option<u32>,
    pub first_change: Option<u8>,
    pub is_business: Option<u8>,
    pub is_ban: Option<u8>,
}

// 自动生成 CRUD 方法
crud!(User{});

// 健康检查路由处理函数
pub async fn rbatis_health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "UP",
        "service": "rbatis",
        "message": "Rbatis service is running normally"
    }))
}

// 获取所有用户处理函数
pub async fn rbatis_get_users() -> Result<impl Responder, actix_web::Error> {
    let rb = &*RBATIS_POOL;
    let users = User::select_all(&**rb).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get users: {}", e))
    })?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Users fetched successfully",
        "status": "success",
        "data": users
    })))
}

// 根据ID获取用户处理函数
pub async fn rbatis_get_user_by_id(user_id: web::Path<u64>) -> Result<impl Responder, actix_web::Error> {
    let rb = &*RBATIS_POOL;
    let condition = rbs::value! { "id": user_id.into_inner() };
    let users = User::select_by_map(&**rb, condition).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get user: {}", e))
    })?;
    
    match users.first() {
        Some(found_user) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "User fetched successfully",
                "status": "success",
                "data": found_user
            })))
        },
        None => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "message": "User not found",
                "status": "error",
                "data": null
            })))
        },
    }
}

// 创建用户处理函数
pub async fn rbatis_create_user(user: web::Json<User>) -> Result<impl Responder, actix_web::Error> {
    let rb = &*RBATIS_POOL;
    let result = User::insert(&**rb, &user.into_inner()).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to create user: {}", e))
    })?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "User created successfully",
        "status": "success",
        "data": result
    })))
}

// 更新用户处理函数
pub async fn rbatis_update_user(
    user_id: web::Path<u64>,
    user: web::Json<User>
) -> Result<impl Responder, actix_web::Error> {
    let rb = &*RBATIS_POOL;
    // 首先检查用户是否存在
    let condition = rbs::value! { "id": user_id.into_inner() };
    let users = User::select_by_map(&**rb, condition.clone()).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to get user: {}", e))
    })?;
    
    if users.is_empty() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "message": "User not found",
            "status": "error",
            "data": null
        })));
    }
    
    // 更新用户
    let result = User::update_by_map(&**rb, &user.into_inner(), condition).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to update user: {}", e))
    })?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "User updated successfully",
        "status": "success",
        "data": result
    })))
}

// 删除用户处理函数
pub async fn rbatis_delete_user(user_id: web::Path<u64>) -> Result<impl Responder, actix_web::Error> {
    let rb = &*RBATIS_POOL;
    let condition = rbs::value! { "id": user_id.into_inner() };
    let result = User::delete_by_map(&**rb, condition).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to delete user: {}", e))
    })?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "User deleted successfully",
        "status": "success",
        "data": result
    })))
}