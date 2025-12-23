// 导出路由模块
use actix_web::web;

pub mod main_routes;  // 现有的主要路由
pub mod auth_routes;  // 新的认证路由
pub mod cache_routes; // 缓存相关路由
pub mod redis_routes; // Redis操作路由
pub mod rbatis_routes; // Rbatis路由
pub mod seaorm_routes; // SeaORM based routes
pub mod file_routes; // File upload routes

// 配置所有路由
pub fn config(cfg: &mut web::ServiceConfig) {
    main_routes::config(cfg);
    file_routes::config(cfg); // 添加文件上传路由到不需要权限校验的路由中
    seaorm_routes::config(cfg);
}