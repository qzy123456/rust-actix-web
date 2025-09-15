// 导出路由模块
use actix_web::web;

pub mod main_routes;  // 现有的主要路由
pub mod auth_routes;  // 新的认证路由
pub mod cache_routes; // 缓存相关路由
pub mod redis_routes; // Redis操作路由

// 配置所有路由
pub fn config(cfg: &mut web::ServiceConfig) {
    main_routes::config(cfg);
    auth_routes::config(cfg);
    cache_routes::config(cfg);
    redis_routes::config(cfg);
    
}