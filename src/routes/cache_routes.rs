use actix_web::{web, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::cache::Cache;
// 导入通用的 ApiResponse
use crate::common::ApiResponse;

// 设置缓存请求结构
#[derive(Debug, Deserialize)]
pub struct SetCacheRequest {
    key: String,
    value: String,
    ttl: Option<u64>, // 可选的过期时间（秒）
}

// 设置缓存项
pub async fn set_cache(
    cache: web::Data<Cache>,
    request: web::Json<SetCacheRequest>,
) -> impl Responder {
    match cache.set(&request.key, request.value.clone(), request.ttl) {
        Ok(_) => ApiResponse::success(json!({
            "message": format!("缓存项 '{}' 设置成功", request.key)
        })),
        Err(err) => ApiResponse::<serde_json::Value>::error(500, format!("设置缓存失败: {}", err)),
    }
}

// 获取缓存项
pub async fn get_cache(
    cache: web::Data<Cache>,
    path: web::Path<String>,
) -> impl Responder {
    let key = path.into_inner();
    match cache.get(&key) {
        Ok(Some(value)) => ApiResponse::success(json!({
            "key": key,
            "value": value
        })),
        Ok(None) => ApiResponse::<serde_json::Value>::error(404, format!("缓存项 '{}' 不存在", key)),
        Err(err) => ApiResponse::<serde_json::Value>::error(500, format!("获取缓存失败: {}", err)),
    }
}

// 删除缓存项
pub async fn delete_cache(
    cache: web::Data<Cache>,
    path: web::Path<String>,
) -> impl Responder {
    let key = path.into_inner();
    match cache.remove(&key) {
        Ok(true) => ApiResponse::success(json!({
            "message": format!("缓存项 '{}' 删除成功", key)
        })),
        Ok(false) => ApiResponse::<serde_json::Value>::error(404, format!("缓存项 '{}' 不存在", key)),
        Err(err) => ApiResponse::<serde_json::Value>::error(500, format!("删除缓存失败: {}", err)),
    }
}

// 获取缓存状态
pub async fn get_cache_status(
    cache: web::Data<Cache>,
) -> impl Responder {
    match cache.len() {
        Ok(count) => {
            let response_data = json!({
                "status": "success",
                "item_count": count,
                "message": "缓存状态正常"
            });
            ApiResponse::success(response_data)
        },
        Err(err) => ApiResponse::<serde_json::Value>::error(500, format!("获取缓存状态失败: {}", err)),
    }
}

// 清空缓存
pub async fn clear_cache(
    cache: web::Data<Cache>,
) -> impl Responder {
    match cache.clear() {
        Ok(_) => ApiResponse::success(json!({
            "message": "缓存已清空"
        })),
        Err(err) => ApiResponse::<serde_json::Value>::error(500, format!("清空缓存失败: {}", err)),
    }
}