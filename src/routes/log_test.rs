use actix_web::{web, HttpRequest, HttpResponse}; 
use std::sync::{Arc, Mutex}; 
use crate::middleware::{JsonLogger, LogLevel}; 
use serde_json::json; 
use chrono;

// 日志测试路由处理函数
pub async fn test_json_logger(
    req: HttpRequest,
    logger: web::Data<Arc<Mutex<JsonLogger>>> 
) -> HttpResponse {
    // 从请求中获取查询参数
    let query_params = web::Query::<serde_json::Value>::from_query(req.query_string())
        .unwrap_or(web::Query(serde_json::Value::Object(Default::default())));
    
    // 从路径中获取信息
    let path = req.path().to_string();
    let method = req.method().to_string();
    
    // 尝试获取并锁定JSON日志器
    if let Ok(mut logger_guard) = logger.lock() {
        // 记录基本信息日志
        let _ = logger_guard.log_with_data(
            LogLevel::INFO, 
            "JSON日志测试: 基本信息", 
            json!({"path": path, "method": method, "query_params": query_params.into_inner()})
        );
        
        // 记录警告级别的日志
        let _ = logger_guard.log_with_data(
            LogLevel::WARNING, 
            "JSON日志测试: 警告信息", 
            json!({"warning_type": "test_warning", "severity": "low"})
        );
        
        // 记录错误级别的日志
        let _ = logger_guard.log_with_data(
            LogLevel::ERROR, 
            "JSON日志测试: 错误信息", 
            json!({"error_type": "test_error", "code": "TEST_001", "description": "这是一个测试错误"})
        );
    }
    
    // 返回成功响应
    HttpResponse::Ok().json(web::Json(json!({
        "status": "success",
        "message": "JSON日志记录成功，请查看日志文件",
        "log_file": format!("logs/app_{}.log", chrono::Local::now().format("%Y%m%d"))
    })))
}

// 配置日志测试路由
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/test-logger", web::get().to(test_json_logger))
    );
}