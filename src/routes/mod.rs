// 导出路由模块
use actix_web::web;

pub mod main_routes;  // 现有的主要路由
pub mod log_test;     // 新的日志测试路由

// 路由配置函数
pub fn config(cfg: &mut web::ServiceConfig) {
    // 健康检查路由
    cfg.route("/health", web::get().to(main_routes::health_check));
    
    // 用户相关路由
    cfg.service(
        web::scope("/users")
            .route("/", web::post().to(main_routes::create_user))
            .route("/", web::get().to(main_routes::get_users))
            .route("/{id}", web::get().to(main_routes::get_user_by_id))
            .route("/{id}", web::put().to(main_routes::update_user))
            .route("/{id}", web::delete().to(main_routes::delete_user))
    );
    
    // 日志测试路由
    cfg.route("/log-test", web::get().to(log_test::test_json_logger));
}