use actix_web::http::{StatusCode, header};
use actix_web::{
    App, HttpRequest, HttpServer, Result, dev, middleware::ErrorHandlerResponse,
    middleware::ErrorHandlers, middleware::Logger, web,
};
use env_logger::Env;
use std::sync::{Arc, Mutex};

// 引入我们拆分出去的模块
mod cache;
mod db;
mod entity;
mod middleware;
mod redis_pool;
mod routes;
mod utils;
// 添加 rbatis 模块
mod rbatis_pool;
// 添加通用响应模块
mod common;

// 从middleware模块导入必要的类型
use middleware::{Claims, JsonLogger, JsonLoggerConfig, JwtMiddleware, LogLevel};
use sea_orm::{ConnectOptions, Database};
use serde_json::json;
use sqlx::mysql::MySqlPoolOptions;
use std::env;
use std::time::Duration;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化标准日志
    //env_logger::init_from_env(Env::default().default_filter_or("info"));
    // 配置日志,用于rbatis,可以打印mysql查询sql
    fast_log::init(
        fast_log::Config::new()
            .level(log::LevelFilter::Debug)
            .console(),
    )
    .expect("rbatis初始化失败");
    // 初始化JSON日志器
    let json_logger = Arc::new(Mutex::new(
        JsonLogger::new(JsonLoggerConfig::default()).expect("Failed to initialize JSON logger"),
    ));

    // 记录服务器启动信息
    {
        let mut logger = json_logger.lock().unwrap();
        logger.info("服务器开始初始化").unwrap();
    }

    // 注释掉原有的数据库连接池初始化代码
    // 初始化数据库连接池（用于 `auth` 等路由）
    let pool = match db::init_db_pool() {
        Ok(pool) => {
            // 记录数据库连接成功
            {
                let mut logger = json_logger.lock().unwrap();
                logger.info("数据库连接池初始化成功").unwrap();
            }
            pool
        }
        Err(err) => {
            // 记录数据库连接失败
            {
                let mut logger = json_logger.lock().unwrap();
                let error_data = json!({"error": format!("{:?}", err)});
                // 使用 ERROR 而非 FATAL，以便程序可继续运行（取决于你的策略）
                logger
                    .log_with_data(LogLevel::ERROR, "数据库连接池初始化失败", error_data)
                    .unwrap();
            }
            eprintln!("Failed to initialize database pool: {:?}", err);
            std::process::exit(1);
        }
    };

    // 注册JSON日志器为应用数据
    let app_data_logger = web::Data::new(json_logger.clone());

    // 初始化JWT中间件 - 实际应用中应该从环境变量读取密钥
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-default-secret-key-1234567890".to_string());
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
        }
        Err(err) => {
            // 记录Redis连接失败
            {
                let mut logger = json_logger.lock().unwrap();
                let error_data = json!({"error": format!("{:?}", err)});
                logger
                    .log_with_data(LogLevel::ERROR, "Redis连接池初始化失败", error_data)
                    .unwrap();
            }
            eprintln!("Failed to initialize Redis pool: {:?}", err);
            // 注意：Redis不是必须的，所以这里不退出程序
            // 我们将使用一个空的Arc，在实际使用时会检查连接池是否可用
            Arc::new(None)
        }
    };

    // 注册Redis连接池作为应用数据
    let app_data_redis = web::Data::new(redis_pool);

    // 注册 MySQL 连接池为应用数据（供 auth_routes 等使用）
    let app_data_pool = web::Data::new(pool.clone());

    // 尝试创建 SeaORM 数据库连接池（可选），通过一次读取 `DATABASE_URL`
    dotenvy::dotenv().ok();
    let app_data_db = if let Ok(db_url) = env::var("DATABASE_URL") {
        println!("Database URL: {}", db_url);

        // 配置连接选项
        let mut opt = ConnectOptions::new(db_url.clone());
        opt.max_connections(20)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(8))
            .max_lifetime(Duration::from_secs(8))
            .sqlx_logging(true);

        match Database::connect(opt).await {
            Ok(db_conn) => {
                if let Ok(mut logger) = json_logger.lock() {
                    let _ = logger.info(&format!("SeaORM 连接池已建立: {}", db_url));
                }
                Some(web::Data::new(db_conn))
            }
            Err(e) => {
                if let Ok(mut logger) = json_logger.lock() {
                    let _ = logger.log_with_data(
                        LogLevel::ERROR,
                        "SeaORM 连接池建立失败",
                        json!({"error": format!("{}", e)}),
                    );
                }
                None
            }
        }
    } else {
        None
    };

    // 初始化 SQLx 连接池（直接使用 sqlx）
    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let sqlx_pool = MySqlPoolOptions::new()
        .max_connections(10)
        .min_connections(3)
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .connect(&db_url)
        .await
        .expect("Failed to connect to MySQL with sqlx");

    if let Ok(mut logger) = json_logger.lock() {
        let _ = logger.info("SQLx 连接池已建立");
    }
    let app_data_sqlx_pool = web::Data::new(sqlx_pool);

    // 启动HTTP服务器
    HttpServer::new(move || {
        // 先构建基础的 App
        let mut app = App::new()
            // 添加JWT中间件 - 放在错误处理中间件之前
            .wrap(jwt_middleware.clone())
            // 添加错误处理中间件
            .wrap(middleware::ErrorHandler)
            // 添加日志中间件
            .wrap(Logger::default())
            .wrap(
                ErrorHandlers::new() // 这里需要 new() 方法
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, add_error_header)
                    .handler(StatusCode::NOT_FOUND, add_error_header)
                    .handler(StatusCode::UNAUTHORIZED, add_error_header),
            )
            // 注册JSON日志器作为应用数据
            .app_data(app_data_logger.clone())
            // 注册JWT中间件作为应用数据
            .app_data(app_data_jwt.clone())
            // 注册缓存作为应用数据
            .app_data(app_data_cache.clone())
            // 注册Redis连接池作为应用数据
            .app_data(app_data_redis.clone())
            // 注册 MySQL 连接池作为应用数据
            .app_data(app_data_pool.clone())
            // 注册sea—orm的链接
            .app_data(app_data_db.clone())
            // 注册 sqlx 连接池
            .app_data(app_data_sqlx_pool.clone());

        // 配置路由
        app.configure(routes::config)
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
