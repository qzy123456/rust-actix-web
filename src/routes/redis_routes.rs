use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use crate::redis_pool;

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

// Redis GET操作处理函数
// 使用方式: GET /api/redis/{key}
pub async fn redis_get(
    path: web::Path<String>,
    redis_pool: web::Data<redis_pool::RedisPool>,
) -> Result<impl Responder, actix_web::Error> {
    // 获取路径中的key参数
    let key = path.into_inner();
    
    // 获取Redis连接
    let mut conn = redis_pool::get_redis_connection_or_return_error(&redis_pool).await?;
    
    // 从Redis中获取值
    match redis_pool::get(&mut conn, &key).await {
        Ok(value) => {
            if let Some(val) = value {
                let response = RedisResponse {
                    status: "success".to_string(),
                    message: format!("Key '{}' found", key),
                    data: Some(val),
                };
                Ok(HttpResponse::Ok().json(response))
            } else {
                let response = RedisResponse {
                    status: "not_found".to_string(),
                    message: format!("Key '{}' not found", key),
                    data: None,
                };
                Ok(HttpResponse::NotFound().json(response))
            }
        },
        Err(e) => {
            let response = RedisResponse {
                status: "error".to_string(),
                message: format!("Failed to get key '{}': {}", key, e),
                data: None,
            };
            Ok(HttpResponse::InternalServerError().json(response))
        }
    }
}

// Redis SET操作处理函数
// 使用方式: POST /api/redis/set
pub async fn redis_set(
    req: web::Json<RedisSetRequest>,
    redis_pool: web::Data<redis_pool::RedisPool>,
) -> Result<impl Responder, actix_web::Error> {
    // 获取请求体中的参数
    let key = &req.key;
    let value = &req.value;
    let expiry_seconds = req.expiry_seconds.unwrap_or(3600); // 默认过期时间为1小时
    
    // 获取Redis连接
    let mut conn = redis_pool::get_redis_connection_or_return_error(&redis_pool).await?;
    
    // 设置键值对，带过期时间
    if let Err(e) = redis_pool::set_with_expiry(&mut conn, key, value, expiry_seconds).await {
        let response = RedisResponse {
            status: "error".to_string(),
            message: format!("Failed to set key '{}': {}", key, e),
            data: None,
        };
        return Ok(HttpResponse::InternalServerError().json(response));
    }
    
    let response = RedisResponse {
        status: "success".to_string(),
        message: format!("Key '{}' set successfully with expiry of {} seconds", key, expiry_seconds),
        data: Some(value.clone()),
    };
    Ok(HttpResponse::Ok().json(response))
}

// 配置Redis路由
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/redis")
            .route("/{key}", web::get().to(redis_get))
            .route("/set", web::post().to(redis_set))
    );
}