use actix_web::{web, App, HttpServer, HttpRequest, middleware::Logger,dev,Result,middleware::ErrorHandlerResponse,middleware::ErrorHandlers};
use actix_web::http::{header, StatusCode};
use env_logger::Env;
use std::sync::{Arc, Mutex};

// 引入我们拆分出去的模块
mod db;
mod routes;
mod middleware;
mod utils;
mod cache;
mod redis_pool;
// 添加 rbatis 模块
mod rbatis_pool;

// 从middleware模块导入必要的类型
use middleware::{JsonLogger, JsonLoggerConfig, LogLevel, JwtMiddleware, Claims};
use serde_json::json;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化标准日志
    //env_logger::init_from_env(Env::default().default_filter_or("info"));
    // 配置日志,用于rbatis,可以打印mysql查询sql
    fast_log::init(fast_log::Config::new().console()).expect("rbatis初始化失败");
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
    
    // 注释掉原有的数据库连接池初始化代码
    // let pool = match db::init_db_pool() {
    //     Ok(pool) => {
    //         // 记录数据库连接成功
    //         {
    //             let mut logger = json_logger.lock().unwrap();
    //             logger.info("数据库连接池初始化成功").unwrap();
    //         }
    //         pool
    //     },
    //     Err(err) => {
    //         // 记录数据库连接失败
    //         {
    //             let mut logger = json_logger.lock().unwrap();
    //             let error_data = json!({"error": format!("{:?}", err)});
    //             logger.log_with_data(LogLevel::FATAL, "数据库连接池初始化失败", error_data).unwrap();
    //         }
    //         eprintln!("Failed to initialize database pool: {:?}", err);
    //         std::process::exit(1);
    //     }
    // };
    
    // 注册JSON日志器为应用数据
    let app_data_logger = web::Data::new(json_logger.clone());
    
    // 初始化JWT中间件 - 实际应用中应该从环境变量读取密钥
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-default-secret-key-1234567890".to_string());
    let jwt_middleware = JwtMiddleware::new(jwt_secret);
    let app_data_jwt = web::Data::new(jwt_middleware.clone());
    
    // 初始化缓存
    let cache = cache::init_cache();
    let app_data_cache = web::Data::new(cache.clone());
    
    // 记录缓存初始化信息
    {
        let mut logger = json_logger.lock().unwrap();
        logger.info("缓存初始化成功").unwrap();
    }
    
    // 初始化Redis连接池
    let redis_pool = match redis_pool::init_redis_pool() {
        Ok(redis_pool) => {
            // 记录Redis连接成功
            {
                let mut logger = json_logger.lock().unwrap();
                logger.info("Redis连接池初始化成功").unwrap();
            }
            redis_pool
        },
        Err(err) => {
            // 记录Redis连接失败
            {
                let mut logger = json_logger.lock().unwrap();
                let error_data = json!({"error": format!("{:?}", err)});
                logger.log_with_data(LogLevel::ERROR, "Redis连接池初始化失败", error_data).unwrap();
            }
            eprintln!("Failed to initialize Redis pool: {:?}", err);
            // 注意：Redis不是必须的，所以这里不退出程序
            // 我们将使用一个空的Arc，在实际使用时会检查连接池是否可用
            Arc::new(None)
        }
    };
    
    // 注册Redis连接池作为应用数据
    let app_data_redis = web::Data::new(redis_pool);
    
    // 启动HTTP服务器
    HttpServer::new(move || {
        App::new()
            // 添加JWT中间件 - 放在错误处理中间件之前
            .wrap(jwt_middleware.clone())
            // 添加错误处理中间件
            .wrap(middleware::ErrorHandler)
            // 添加日志中间件
            .wrap(Logger::default())
            .wrap(
                ErrorHandlers::new()  // 这里需要 new() 方法
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, add_error_header)
                    .handler(StatusCode::NOT_FOUND, add_error_header)
                    .handler(StatusCode::UNAUTHORIZED, add_error_header)
            )
            // 注释掉原有的数据库连接池注册
            // .app_data(web::Data::new(pool.clone()))
            // 注册JSON日志器作为应用数据
            .app_data(app_data_logger.clone())
            // 注册JWT中间件作为应用数据
            .app_data(app_data_jwt.clone())
            // 注册缓存作为应用数据
            .app_data(app_data_cache.clone())
            // 注册Redis连接池作为应用数据
            .app_data(app_data_redis.clone())
            // 配置路由
            .configure(routes::config)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// 自定义一些错误头
fn add_error_header<B>(mut res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut().headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}