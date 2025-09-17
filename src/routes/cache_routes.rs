use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::cache::Cache;

// 设置缓存请求结构
#[derive(Debug, Deserialize)]
pub struct SetCacheRequest {
    key: String,
    value: String,
    ttl: Option<u64>, // 可选的过期时间（秒）
}

// 缓存状态响应结构
#[derive(Debug, Serialize)]
pub struct CacheStatusResponse {
    status: String,
    item_count: usize,
    message: String,
}

// 设置缓存项
pub async fn set_cache(
    cache: web::Data<Cache>,
    request: web::Json<SetCacheRequest>,
) -> impl Responder {
    match cache.set(&request.key, request.value.clone(), request.ttl) {
        Ok(_) => HttpResponse::Ok().json(json!({
            "status": "success",
            "message": format!("缓存项 '{}' 设置成功", request.key)
        })),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("设置缓存失败: {}", err)
        })),
    }
}

// 获取缓存项
pub async fn get_cache(
    cache: web::Data<Cache>,
    path: web::Path<String>,
) -> impl Responder {
    let key = path.into_inner();
    match cache.get(&key) {
        Ok(Some(value)) => HttpResponse::Ok().json(json!({
            "status": "success",
            "key": key,
            "value": value
        })),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "status": "error",
            "message": format!("缓存项 '{}' 不存在", key)
        })),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("获取缓存失败: {}", err)
        })),
    }
}

// 删除缓存项
pub async fn delete_cache(
    cache: web::Data<Cache>,
    path: web::Path<String>,
) -> impl Responder {
    let key = path.into_inner();
    match cache.remove(&key) {
        Ok(true) => HttpResponse::Ok().json(json!({
            "status": "success",
            "message": format!("缓存项 '{}' 删除成功", key)
        })),
        Ok(false) => HttpResponse::NotFound().json(json!({
            "status": "error",
            "message": format!("缓存项 '{}' 不存在", key)
        })),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("删除缓存失败: {}", err)
        })),
    }
}

// 获取缓存状态
pub async fn get_cache_status(
    cache: web::Data<Cache>,
) -> impl Responder {
    match cache.len() {
        Ok(count) => {
            let response = CacheStatusResponse {
                status: "success".to_string(),
                item_count: count,
                message: "缓存状态正常".to_string(),
            };
            HttpResponse::Ok().json(response)
        },
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("获取缓存状态失败: {}", err)
        })),
    }
}

// 清空缓存
pub async fn clear_cache(
    cache: web::Data<Cache>,
) -> impl Responder {
    match cache.clear() {
        Ok(_) => HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "缓存已清空"
        })),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": "error",
            "message": format!("清空缓存失败: {}", err)
        })),
    }
}