// 导出路由模块
use actix_web::web;

pub mod main_routes;  // 现有的主要路由
pub mod log_test;     // 新的日志测试路由
pub mod auth_routes;  // 新的认证路由

// 配置所有路由
pub fn config(cfg: &mut web::ServiceConfig) {
    main_routes::config(cfg);
    log_test::config(cfg);
    auth_routes::config(cfg);
}