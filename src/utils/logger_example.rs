use crate::middleware::{JsonLogger, JsonLoggerConfig, LogLevel};
use serde_json::json;

// JSON日志器示例
pub fn demonstrate_json_logger() {
    // 创建默认配置的日志器
    let mut default_logger = JsonLogger::new(JsonLoggerConfig::default()).expect("Failed to create default logger");
    
    // 使用默认配置的日志器记录不同级别的日志
    default_logger.trace("这是一条TRACE级别的日志").unwrap();
    default_logger.debug("这是一条DEBUG级别的日志").unwrap();
    default_logger.info("这是一条INFO级别的日志").unwrap();
    default_logger.warning("这是一条WARNING级别的日志").unwrap();
    default_logger.error("这是一条ERROR级别的日志").unwrap();
    default_logger.fatal("这是一条FATAL级别的日志").unwrap();
    
    // 创建自定义配置的日志器
    let custom_config = JsonLoggerConfig {
        log_dir: String::from("custom_logs"),  // 自定义日志目录
        max_file_size_mb: 5,                    // 最大文件大小为5MB
        min_level: LogLevel::DEBUG,             // 最小日志级别为DEBUG
    };
    
    let mut custom_logger = JsonLogger::new(custom_config).expect("Failed to create custom logger");
    
    // 使用自定义日志器记录带附加数据的日志
    let user_data = json!({
        "user_id": 12345,
        "username": "test_user",
        "ip_address": "127.0.0.1"
    });
    
    custom_logger.log_with_data(
        LogLevel::INFO, 
        "用户登录成功", 
        user_data
    ).unwrap();
    
    // 记录详细日志，包括模块、文件和行号
    custom_logger.log_detailed(
        LogLevel::ERROR, 
        "数据库连接失败", 
        "database", 
        "src/db.rs", 
        42
    ).unwrap();
    
    // 记录带有复杂附加数据的日志
    let request_data = json!({
        "method": "GET",
        "path": "/api/users",
        "headers": {
            "User-Agent": "Mozilla/5.0",
            "Content-Type": "application/json"
        },
        "query_params": {
            "page": 1,
            "limit": 10
        }
    });
    
    default_logger.log_with_data(
        LogLevel::DEBUG, 
        "处理API请求", 
        request_data
    ).unwrap();
    
    // 记录错误信息和堆栈
    let error_info = json! ({
        "error_type": "DatabaseError",
        "error_code": "DB001",
        "details": "无法连接到数据库服务器",
        "stack_trace": "at src/db.rs:42\nat src/routes.rs:123\nat src/main.rs:67"
    });
    
    default_logger.log_with_data(
        LogLevel::FATAL, 
        "系统致命错误", 
        error_info
    ).unwrap();
}

// 在main.rs中集成JSON日志器的示例
/*
// 在main.rs中添加以下代码
use crate::utils::logger_example::demonstrate_json_logger;
use crate::middleware::{JsonLogger, JsonLoggerConfig};
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化JSON日志器
    let json_logger = Arc::new(Mutex::new(
        JsonLogger::new(JsonLoggerConfig::default())
            .expect("Failed to initialize JSON logger")
    ));
    
    // 将日志器注册为应用数据
    let app_data_logger = web::Data::new(json_logger.clone());
    
    // 在需要记录日志的地方使用
    { 
        let mut logger = json_logger.lock().unwrap();
        logger.info("服务器启动成功").unwrap();
    }
    
    // 启动HTTP服务器
    HttpServer::new(move || {
        App::new()
            // 添加JSON日志器作为应用数据
            .app_data(app_data_logger.clone())
            // ... 其他中间件和路由配置
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
*/

// 在中间件中使用JSON日志器的示例
/*
// 在中间件中添加以下代码
use crate::middleware::JsonLogger;
use actix_web::web::Data;
use std::sync::{Arc, Mutex};

// 在中间件的call方法中
fn call(&self, req: ServiceRequest) -> Self::Future {
    // 获取请求信息
    let path = req.path().to_string();
    let method = req.method().to_string();
    
    // 从应用数据中获取日志器
    if let Some(logger) = req.app_data::<Data<Arc<Mutex<JsonLogger>>>>() {
        let request_data = json!({
            "method": method,
            "path": path,
            "headers": req.headers().clone()
        });
        
        let mut logger = logger.lock().unwrap();
        logger.log_with_data(
            LogLevel::INFO, 
            "收到请求", 
            request_data
        ).unwrap();
    }
    
    // ... 其余中间件逻辑
}
*/