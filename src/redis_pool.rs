use std::sync::Arc;
use actix_web::{web, error, Error};
use deadpool_redis::{Config, Pool, Connection, Runtime, redis::cmd};

// Redis连接池类型别名
type RedisPoolInner = Pool;
pub type RedisPool = Arc<Option<RedisPoolInner>>;

// 获取Redis连接的辅助函数 - 直接返回连接或错误响应
// 使用方式: let mut conn = get_redis_connection_or_return_error(&pool).await?;
pub async fn get_redis_connection_or_return_error(
    pool: &web::Data<RedisPool>,
) -> Result<Connection, Error> {
    if let Some(pool_inner) = &****pool {
        pool_inner.get()
            .await
            .map_err(|e| {
                error::ErrorInternalServerError(format!("Redis连接失败: {}", e))
            })
    } else {
        Err(error::ErrorInternalServerError("Redis连接池未初始化"))
    }
}

// 初始化Redis连接池
pub fn init_redis_pool() -> Result<RedisPool, Box<dyn std::error::Error>> {
    // 创建Redis连接配置
    let mut config = Config::default();
    // 设置Redis服务器地址，默认端口6379,库可以自己选择
    config.url = Some("redis://localhost:6379/1".to_string());
    // 可以根据需要配置其他选项，如超时、密码等
    
    // 构建连接池
    let pool = config.create_pool(Some(Runtime::Tokio1))?;
    
    Ok(Arc::new(Some(pool)))
}

// Redis操作工具函数示例

// 设置带过期时间的键值对
pub async fn set_with_expiry(
    conn: &mut Connection,
    key: &str,
    value: &str,
    expiry_seconds: u64
) -> Result<(), Box<dyn std::error::Error>> {
    let _: () = cmd("SET")
        .arg(key)
        .arg(value)
        .arg("EX")
        .arg(expiry_seconds)
        .query_async(conn)
        .await?;
    Ok(())
}

// 获取键值
pub async fn get(
    conn: &mut Connection,
    key: &str
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let result: Option<String> = cmd("GET")
        .arg(key)
        .query_async(conn)
        .await?;
    Ok(result)
}

// 删除键
pub async fn del(
    conn: &mut Connection,
    key: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let _: () = cmd("DEL")
        .arg(key)
        .query_async(conn)
        .await?;
    Ok(())
}

// 增加计数器
pub async fn incr(
    conn: &mut Connection,
    key: &str
) -> Result<i64, Box<dyn std::error::Error>> {
    let result: i64 = cmd("INCR")
        .arg(key)
        .query_async(conn)
        .await?;
    Ok(result)
}

// 存储哈希值
pub async fn hset(
    conn: &mut Connection,
    key: &str,
    field: &str,
    value: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let _: () = cmd("HSET")
        .arg(key)
        .arg(field)
        .arg(value)
        .query_async(conn)
        .await?;
    Ok(())
}

// 获取哈希值
pub async fn hget(
    conn: &mut Connection,
    key: &str,
    field: &str
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let result: Option<String> = cmd("HGET")
        .arg(key)
        .arg(field)
        .query_async(conn)
        .await?;
    Ok(result)
}