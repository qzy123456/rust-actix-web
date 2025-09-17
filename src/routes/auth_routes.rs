use mysql::prelude::Queryable;
use actix_web::{web, HttpResponse, Responder, HttpRequest, HttpMessage, Error}; 
use serde::{Deserialize, Serialize}; 
use std::time::Duration; 
use log::{info, error}; 
use crate::middleware::JwtMiddleware; 
use crate::db::{DbPool, get_connection_or_return_error}; 

// 登录请求结构体
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub phone: String, 
    pub password: String, // 在实际应用中应该使用加密密码
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

// 登录处理函数
pub async fn login(
    req: web::Json<LoginRequest>, 
    pool: web::Data<DbPool>, 
    jwt_middleware: web::Data<JwtMiddleware>,
) -> Result<HttpResponse, Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    // 查找用户 - 在实际应用中应该使用参数化查询防止SQL注入
    let query = format!("SELECT id, phone FROM users WHERE phone = '{}' AND password = '{}' LIMIT 1", req.phone, req.password);
    
    match conn.query_first::<(u64, String), &str>(&query) {
        Ok(Some((user_id, phone))) => {
            info!("用户登录成功: phone={}, user_id={}", phone, user_id);
            
            // 生成JWT令牌，有效期为7天
            match jwt_middleware.generate_token(user_id, phone.clone(), Duration::from_secs(7 * 24 * 60 * 60)) {
                Ok(token) => {
                    Ok(HttpResponse::Ok().json(LoginResponse {
                        token,
                        token_type: "Bearer".to_string(),
                        expires_in: 7 * 24 * 60 * 60, 
                        user_id,
                        phone,
                    }))
                },
                Err(e) => {
                    error!("生成JWT令牌失败: {}", e);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to generate token"})))
                },
            }
        },
        Ok(None) => {
            error!("登录失败: 用户名或密码错误，phone={}", req.phone);
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({"error": "Invalid phone or password"})))
        },
        Err(e) => {
            error!("数据库查询失败: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error"})))
        },
    }
}

// 注册处理函数
pub async fn register(
    req: web::Json<RegisterRequest>, 
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let mut conn = get_connection_or_return_error(&pool).await?;
    
    // 检查用户是否已存在
    let check_query = format!("SELECT id FROM users WHERE phone = '{}' LIMIT 1", req.phone);
    
    if let Ok(Some((_id,))) = conn.query_first::<(u64,), &str>(&check_query) {
        return Ok(HttpResponse::Conflict().json(serde_json::json!({"error": "User with this phone already exists"})));
    }
    
    // 创建新用户
    let empty_string = String::from("");
    let name = req.name.as_ref().unwrap_or(&empty_string);
    let insert_query = format!(
        "INSERT INTO users (phone, password, name) VALUES ('{}', '{}', '{}')",
        req.phone, req.password, name
    );
    
    match conn.query_drop(&insert_query) {
        Ok(_) => {
            // 获取新创建的用户ID
            if let Ok(Some((user_id,))) = conn.query_first::<(u64,), &str>("SELECT LAST_INSERT_ID()") {
                info!("用户注册成功: phone={}, user_id={}", req.phone, user_id);
                return Ok(HttpResponse::Ok().json(RegisterResponse {
                    success: true,
                    user_id,
                    phone: req.phone.clone(),
                }));
            }
            
            error!("注册成功但无法获取用户ID: phone={}", req.phone);
            Ok(HttpResponse::Ok().json(RegisterResponse {
                success: true,
                user_id: 0, // 无法获取ID的情况
                phone: req.phone.clone(),
            }))
        },
        Err(e) => {
            error!("用户注册失败: {}, phone={}", e, req.phone);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to register user"})))
        },
    }
}

// 获取当前用户信息 - 受JWT保护的路由示例
pub async fn get_current_user(req: HttpRequest) -> impl Responder {
    // 从请求扩展中获取用户ID和用户名
    if let (Some(user_id), Some(username)) = (
        req.extensions().get::<u64>(),
        req.extensions().get::<String>()
    ) {
        HttpResponse::Ok().json(serde_json::json!({
            "user_id": user_id,
            "username": username,
            "message": "This is protected data"
        }))
    } else {
        HttpResponse::Unauthorized().json(serde_json::json!({"error": "User not authenticated"}))
    }
}