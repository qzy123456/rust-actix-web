use actix_web::{delete, get, post, put, web, Responder};
use serde::Deserialize;
use sqlx::mysql::MySqlPool;
use sqlx::FromRow;

use crate::common::{ApiResponse, Meta};

// ----------------------
// 数据结构定义
// ----------------------

#[derive(Debug, Deserialize, FromRow, Clone, serde::Serialize)]
struct User {
    id: u64,
    name: String,
}

#[derive(Debug, Deserialize, FromRow, Clone)]
struct UserWithId {
    id: u64,
    name: String,
    #[sqlx(rename = "create_time")]
    create_time: Option<chrono::NaiveDateTime>,
}

// 用于接收前端 JSON 的结构体
#[derive(Deserialize)]
struct UserParams {
    name: String,
}

// 用于接收分页参数
#[derive(Deserialize)]
struct PageParams {
    page: Option<u64>,
    page_size: Option<u64>,
}

// 分页结果结构
#[derive(Debug, serde::Serialize)]
struct PageResult<T> {
    data: Vec<T>,
    total: i64,
    page: u64,
    page_size: u64,
}

// ----------------------
// 路由处理函数
// ----------------------

// 1. 添加用户 (Create)
#[post("/users")]
async fn add_user(
    pool: web::Data<MySqlPool>,
    item: web::Json<UserParams>,
) -> impl Responder {
    let sql = "INSERT INTO users (name) VALUES (?)";
    
    match sqlx::query(sql)
        .bind(&item.name)
        .execute(pool.get_ref())
        .await
    {
        Ok(result) => {
            let id = result.last_insert_id();
            ApiResponse::success(serde_json::json!({
                "id": id,
                "name": item.name.clone()
            }))
        }
        Err(err) => {
            log::error!("add_user failed: {}", err);
            ApiResponse::error(500, err.to_string())
        }
    }
}

// 2. 查找用户 (Read - Find One)
#[get("/users/{id}")]
async fn get_user(pool: web::Data<MySqlPool>, id: web::Path<u64>) -> impl Responder {
    let sql = "SELECT id, name FROM users WHERE id = ?";
    
    match sqlx::query_as::<_, User>(sql)
        .bind(id.into_inner())
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(user)) => ApiResponse::success(user),
        Ok(None) => ApiResponse::error(404, "User not found"),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 3. 分页查找用户 (Read - Pagination)
#[get("/users")]
async fn get_users_page(
    pool: web::Data<MySqlPool>,
    params: web::Query<PageParams>,
) -> impl Responder {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let offset = (page - 1) * page_size;
    
    // 查询数据
    let data_sql = "SELECT id, name FROM users LIMIT ? OFFSET ?";
    let data_result = sqlx::query_as::<_, User>(data_sql)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool.get_ref())
        .await;
    
    // 查询总数
    let count_sql = "SELECT COUNT(*) as total FROM users";
    let count_result = sqlx::query_as::<_, (i64,)>(count_sql)
        .fetch_one(pool.get_ref())
        .await;
    
    match (data_result, count_result) {
        (Ok(users), Ok((total,))) => {
            let total_pages = (total as f64 / page_size as f64).ceil() as u64;
            let meta = Meta {
                total_items: total as u64,
                total_pages,
                current_page: page,
                page_size,
            };
            ApiResponse::success_with_meta(users, meta)
        }
        (Err(err), _) => ApiResponse::error(500, err.to_string()),
        (_, Err(err)) => ApiResponse::error(500, err.to_string()),
    }
}

// 4. 修改用户 (Update)
#[put("/users/{id}")]
async fn update_user(
    pool: web::Data<MySqlPool>,
    id: web::Path<u64>,
    item: web::Json<UserParams>,
) -> impl Responder {
    let user_id = id.into_inner();
    let sql = "UPDATE users SET name = ? WHERE id = ?";
    
    match sqlx::query(sql)
        .bind(&item.name)
        .bind(user_id)
        .execute(pool.get_ref())
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                ApiResponse::success(serde_json::json!({
                    "id": user_id,
                    "name": item.name.clone()
                }))
            } else {
                ApiResponse::error(404, "User not found")
            }
        }
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 5. 删除用户 (Delete)
#[delete("/users/{id}")]
async fn delete_user(pool: web::Data<MySqlPool>, id: web::Path<u64>) -> impl Responder {
    let sql = "DELETE FROM users WHERE id = ?";
    
    match sqlx::query(sql)
        .bind(id.into_inner())
        .execute(pool.get_ref())
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                ApiResponse::success(serde_json::json!({"message": "User deleted"}))
            } else {
                ApiResponse::error(404, "User not found")
            }
        }
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 6. 获取所有用户
#[get("/users/all")]
async fn get_all_users(pool: web::Data<MySqlPool>) -> impl Responder {
    let sql = "SELECT id, name FROM users ORDER BY id DESC";
    
    match sqlx::query_as::<_, User>(sql)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 7. 按名称模糊搜索用户
#[get("/users/search")]
async fn search_users(
    pool: web::Data<MySqlPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let name = match query.get("name") {
        Some(n) => format!("%{}%", n),
        None => return ApiResponse::error(400, "Missing 'name' parameter"),
    };
    
    let sql = "SELECT id, name FROM users WHERE name LIKE ?";
    
    match sqlx::query_as::<_, User>(sql)
        .bind(&name)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 8. 获取用户数量统计
#[get("/users/stats")]
async fn get_user_stats(pool: web::Data<MySqlPool>) -> impl Responder {
    let sql = "SELECT COUNT(*) as total FROM users";
    
    match sqlx::query_as::<_, (i64,)>(sql)
        .fetch_one(pool.get_ref())
        .await
    {
        Ok((total,)) => ApiResponse::success(serde_json::json!({ "total_users": total })),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 9. 批量创建用户
#[post("/users/batch")]
async fn create_users_batch(
    pool: web::Data<MySqlPool>,
    items: web::Json<Vec<UserParams>>,
) -> impl Responder {
    let mut created = Vec::new();
    let mut last_error = String::new();
    
    // 开启事务
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return ApiResponse::error(500, format!("Failed to start transaction: {}", err)),
    };
    
    for item in items.into_inner() {
        let sql = "INSERT INTO users (name) VALUES (?)";
        match sqlx::query(sql)
            .bind(&item.name)
            .execute(&mut *tx)
            .await
        {
            Ok(result) => {
                created.push(serde_json::json!({
                    "id": result.last_insert_id(),
                    "name": item.name
                }));
            }
            Err(err) => {
                last_error = err.to_string();
                if let Err(_) = tx.rollback().await {
                    log::error!("Failed to rollback transaction");
                }
                return ApiResponse::error(500, format!("Failed to create user: {}", last_error));
            }
        }
    }
    
    match tx.commit().await {
        Ok(_) => ApiResponse::success(created),
        Err(err) => {
            log::error!("Failed to commit transaction: {}", err);
            ApiResponse::error(500, err.to_string())
        }
    }
}

// 10. 批量删除用户
#[delete("/users/batch")]
async fn delete_users_batch(
    pool: web::Data<MySqlPool>,
    user_ids: web::Json<Vec<u64>>,
) -> impl Responder {
    let ids = user_ids.into_inner();
    if ids.is_empty() {
        return ApiResponse::error(400, "Empty user ids list");
    }
    
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return ApiResponse::error(500, format!("Failed to start transaction: {}", err)),
    };
    
    let mut deleted_count = 0u64;
    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!("DELETE FROM users WHERE id IN ({})", placeholders.join(","));
    
    let mut query = sqlx::query(&sql);
    for id in &ids {
        query = query.bind(id);
    }
    
    match query.execute(&mut *tx).await {
        Ok(result) => deleted_count = result.rows_affected(),
        Err(err) => {
            if let Err(_) = tx.rollback().await {
                log::error!("Failed to rollback transaction");
            }
            return ApiResponse::error(500, err.to_string());
        }
    }
    
    match tx.commit().await {
        Ok(_) => ApiResponse::success(serde_json::json!({
            "message": format!("Successfully deleted {} users", deleted_count),
            "deleted_count": deleted_count
        })),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 11. 获取用户列表（带排序和限制）
#[get("/users/sorted")]
async fn get_users_sorted(
    pool: web::Data<MySqlPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let sort_by = query.get("sort_by").cloned().unwrap_or_else(|| "id".to_string());
    let order = query.get("order").cloned().unwrap_or_else(|| "asc".to_string());
    let limit = query.get("limit").and_then(|s| s.parse::<u64>().ok()).unwrap_or(100);
    
    let order_clause = match order.as_str() {
        "desc" => "DESC",
        _ => "ASC",
    };
    
    let column = match sort_by.as_str() {
        "name" => "name",
        _ => "id",
    };
    
    let sql = format!("SELECT id, name FROM users ORDER BY {} {} LIMIT ?", column, order_clause);
    
    match sqlx::query_as::<_, User>(&sql)
        .bind(limit)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 12. 条件查询用户（复合条件）
#[get("/users/conditional")]
async fn get_users_conditional(
    pool: web::Data<MySqlPool>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();
    
    if let Some(name) = query.get("name") {
        conditions.push("name LIKE ?");
        params.push(format!("%{}%", name));
    }
    
    if let Some(id_str) = query.get("id") {
        if id_str.parse::<u64>().is_ok() {
            conditions.push("id = ?");
            params.push(id_str.clone());
        }
    }
    
    let sql = if conditions.is_empty() {
        "SELECT id, name FROM users".to_string()
    } else {
        format!("SELECT id, name FROM users WHERE {}", conditions.join(" AND "))
    };
    
    let mut query_builder = sqlx::query_as::<_, User>(&sql);
    for param in &params {
        query_builder = query_builder.bind(param);
    }
    
    match query_builder.fetch_all(pool.get_ref()).await {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// ----------------------
// 注册路由到 /sqlx
// ----------------------
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sqlx")
            .service(add_user)
            .service(get_user)
            .service(get_users_page)
            .service(update_user)
            .service(delete_user)
            .service(get_all_users)
            .service(search_users)
            .service(get_user_stats)
            .service(create_users_batch)
            .service(delete_users_batch)
            .service(get_users_sorted)
            .service(get_users_conditional)
    );
}
