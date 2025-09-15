use actix_web::{web, App, HttpServer, HttpRequest, middleware::Logger};
use env_logger::Env;
use std::sync::{Arc, Mutex};

// 引入我们拆分出去的模块
mod db;
mod routes;
mod middleware;
mod utils;

// 从db模块导入必要的类型
use middleware::{JsonLogger, JsonLoggerConfig, LogLevel};
use serde_json::json;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化标准日志
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    
    // 初始化JSON日志器
    let json_logger = Arc::new(Mutex::new(
        JsonLogger::new(JsonLoggerConfig::default())
            .expect("Failed to initialize JSON logger")
    ));
    
    // 记录服务器启动信息
    {
        let mut logger = json_logger.lock().unwrap();
        logger.info("服务器开始初始化").unwrap();
    }
    
    // 初始化数据库连接池
    let pool = match db::init_db_pool() {
        Ok(pool) => {
            // 记录数据库连接成功
            {
                let mut logger = json_logger.lock().unwrap();
                logger.info("数据库连接池初始化成功").unwrap();
            }
            pool
        },
        Err(err) => {
            // 记录数据库连接失败
            {
                let mut logger = json_logger.lock().unwrap();
                let error_data = json!({"error": format!("{:?}", err)});
                logger.log_with_data(LogLevel::FATAL, "数据库连接池初始化失败", error_data).unwrap();
            }
            eprintln!("Failed to initialize database pool: {:?}", err);
            std::process::exit(1);
        }
    };
    
    // 注册JSON日志器为应用数据
    let app_data_logger = web::Data::new(json_logger.clone());
    
    // 启动HTTP服务器
    HttpServer::new(move || {
        App::new()
            // 添加错误处理中间件
            .wrap(middleware::ErrorHandler)
            // 添加日志中间件
            .wrap(Logger::default())
            // 注册数据库连接池作为应用数据
            .app_data(web::Data::new(pool.clone()))
            // 注册JSON日志器作为应用数据
            .app_data(app_data_logger.clone())
            // 配置路由
            .configure(routes::config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}