use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;
use serde_json::Value;
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;

use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set, QueryFilter, ColumnTrait, QueryOrder, QuerySelect, TransactionTrait, QueryTrait, SelectColumns, Select};
use serde::Deserialize;

use crate::entity::{ActiveModel, Entity as User, Model};
use crate::entity::orders;

#[derive(Serialize)]
struct Meta {
    total_items: u64,
    total_pages: u64,
    current_page: u64,
    page_size: u64,
}

#[derive(Serialize)]
struct ApiResponse<T>
where
    T: Serialize,
{
    code: u16,
    message: String,
    data: Option<T>,
    meta: Option<Value>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    fn success(data: T) -> Self {
        ApiResponse {
            code: 200,
            message: "OK".to_string(),
            data: Some(data),
            meta: None,
        }
    }

    fn success_with_meta(data: T, meta: Meta) -> Self {
        ApiResponse {
            code: 200,
            message: "OK".to_string(),
            data: Some(data),
            meta: Some(serde_json::to_value(meta).unwrap_or(Value::Null)),
        }
    }

    fn error(code: u16, message: impl Into<String>) -> ApiResponse<T> {
        ApiResponse {
            code,
            message: message.into(),
            data: None,
            meta: None,
        }
    }
}

impl<T> Responder for ApiResponse<T>
where
    T: Serialize,
{
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<BoxBody> {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::OK);
        HttpResponse::build(status).json(self)
    }
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

// --------------------------
// 1. 添加用户 (Create)
// --------------------------
#[post("/users")]
async fn add_user(
    db: web::Data<DatabaseConnection>,
    item: web::Json<UserParams>,
) -> impl Responder {
    let new_user = ActiveModel {
        name: Set(item.name.clone()),
        ..Default::default()
    };

    match new_user.insert(db.get_ref()).await {
        Ok(user) => ApiResponse::success(user),
        Err(err) => {
            log::error!("add_user failed: {:#?}", err);
            ApiResponse::error(500, err.to_string())
        }
    }
}

// --------------------------
// 2. 查找用户 (Read - Find One)
// --------------------------
#[get("/users/{id}")]
async fn get_user(db: web::Data<DatabaseConnection>, id: web::Path<u64>) -> impl Responder {
    let user = User::find_by_id(id.into_inner()).one(db.get_ref()).await;

    match user {
        Ok(Some(u)) => ApiResponse::success(u),
        Ok(None) => ApiResponse::error(404, "User not found"),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 3. 分页查找用户 (Read - Pagination)
// --------------------------
#[get("/users")]
async fn get_users_page(
    db: web::Data<DatabaseConnection>,
    params: web::Query<PageParams>,
) -> impl Responder {
    // 默认第 1 页，每页 10 条
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);

    // SeaORM 的分页器
    let paginator = User::find().paginate(db.get_ref(), page_size);
    
    // SeaORM page 索引从 0 开始，所以如果前端传 1，我们要减 1
    let page_num = if page > 0 { page - 1 } else { 0 };

    match paginator.fetch_page(page_num).await {
        Ok(users) => {
            let total_items = paginator.num_items().await.unwrap_or(0);
            let total_pages = paginator.num_pages().await.unwrap_or(0);

            let meta = Meta {
                total_items,
                total_pages,
                current_page: page,
                page_size,
            };

            ApiResponse::success_with_meta(users, meta)
        }
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 4. 修改用户 (Update)
// --------------------------
#[put("/users/{id}")]
async fn update_user(
    db: web::Data<DatabaseConnection>,
    id: web::Path<u64>,
    item: web::Json<UserParams>,
) -> impl Responder {
    // 先查找是否存在
    let user_opt = User::find_by_id(id.into_inner()).one(db.get_ref()).await;

    match user_opt {
        Ok(Some(user)) => {
            // 将 Model 转换为 ActiveModel 以进行更新
            let mut active_user: ActiveModel = user.into();
            active_user.name = Set(item.name.clone());

            match active_user.update(db.get_ref()).await {
                Ok(updated_user) => ApiResponse::success(updated_user),
                Err(err) => ApiResponse::error(500, err.to_string()),
            }
        }
        Ok(None) => ApiResponse::error(404, "User not found"),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 5. 删除用户 (Delete)
// --------------------------
#[delete("/users/{id}")]
async fn delete_user(db: web::Data<DatabaseConnection>, id: web::Path<u64>) -> impl Responder {
    let result = User::delete_by_id(id.into_inner()).exec(db.get_ref()).await;

    match result {
        Ok(res) => {
            if res.rows_affected > 0 {
                ApiResponse::success(serde_json::json!({"message":"User deleted"}))
            } else {
                ApiResponse::error(404, "User not found")
            }
        }
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 6. 查询用户及其订单 (关联查询)
// --------------------------
#[get("/users/{id}/orders")]
async fn get_user_with_orders(
    db: web::Data<DatabaseConnection>,
    id: web::Path<u64>,
) -> impl Responder {
    let uid = id.into_inner();

    // 先查用户
    match User::find_by_id(uid).one(db.get_ref()).await {
        Ok(Some(user)) => {
            // 明确按 uid 查询订单（不依赖 DeriveRelation 的宏实现）
            match orders::Entity::find()
                .filter(orders::Column::Uid.eq(uid))
                .all(db.get_ref())
                .await
            {
                Ok(ord_list) => {
                    let resp = serde_json::json!({"user": user, "orders": ord_list});
                    ApiResponse::success(resp)
                }
                Err(err) => ApiResponse::error(500, err.to_string()),
            }
        }
        Ok(None) => ApiResponse::error(404, "User not found"),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 7. 按名称模糊搜索用户
// --------------------------
#[get("/users/search")]
async fn search_users_by_name(
    db: web::Data<DatabaseConnection>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let name = match query.get("name") {
        Some(n) => n.clone(),
        None => return ApiResponse::error(400, "Missing 'name' parameter"),
    };

    match User::find()
        .filter(crate::entity::users::Column::Name.contains(&name))
        .all(db.get_ref())
        .await
    {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 8. 获取用户数量统计
// --------------------------
#[get("/users/stats")]
async fn get_user_stats(db: web::Data<DatabaseConnection>) -> impl Responder {
    match User::find().count(db.get_ref()).await {
        Ok(count) => ApiResponse::success(serde_json::json!({ "total_users": count })),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 9. 批量创建用户
// --------------------------
#[post("/users/batch")]
async fn create_users_batch(
    db: web::Data<DatabaseConnection>,
    items: web::Json<Vec<UserParams>>,
) -> impl Responder {
    // 注意：在实际应用中，你可能需要使用事务来确保批量操作的一致性
    let mut created_users = Vec::new();
    
    for item in items.into_inner() {
        let new_user = ActiveModel {
            name: Set(item.name.clone()),
            ..Default::default()
        };

        match new_user.insert(db.get_ref()).await {
            Ok(user) => created_users.push(user),
            Err(err) => {
                log::error!("Failed to create user {}: {:#?}", item.name, err);
                // 根据业务需求决定是否继续或回滚
            }
        }
    }
    
    ApiResponse::success(created_users)
}

// --------------------------
// 10. 获取用户列表（带排序和限制）
// --------------------------
#[get("/users/sorted")]
async fn get_users_sorted(
    db: web::Data<DatabaseConnection>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let mut query_builder = User::find();
    
    // 处理排序参数
    if let Some(sort_by) = query.get("sort_by") {
        match sort_by.as_str() {
            "name" => {
                if let Some(order) = query.get("order") {
                    if order == "desc" {
                        query_builder = query_builder.order_by(crate::entity::users::Column::Name, sea_orm::Order::Desc);
                    } else {
                        query_builder = query_builder.order_by(crate::entity::users::Column::Name, sea_orm::Order::Asc);
                    }
                } else {
                    query_builder = query_builder.order_by(crate::entity::users::Column::Name, sea_orm::Order::Asc);
                }
            },
            "id" => {
                if let Some(order) = query.get("order") {
                    if order == "desc" {
                        query_builder = query_builder.order_by(crate::entity::users::Column::Id, sea_orm::Order::Desc);
                    } else {
                        query_builder = query_builder.order_by(crate::entity::users::Column::Id, sea_orm::Order::Asc);
                    }
                } else {
                    query_builder = query_builder.order_by(crate::entity::users::Column::Id, sea_orm::Order::Asc);
                }
            },
            _ => {} // 默认排序
        }
    }
    
    // 处理限制结果数量
    if let Some(limit_str) = query.get("limit") {
        if let Ok(limit) = limit_str.parse::<u64>() {
            query_builder = query_builder.limit(limit);
        }
    }
    
    match query_builder.all(db.get_ref()).await {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 11. 事务示例：批量删除用户
// --------------------------
#[delete("/users/batch")]
async fn delete_users_batch(
    db: web::Data<DatabaseConnection>,
    user_ids: web::Json<Vec<u64>>,
) -> impl Responder {
    // 开始事务
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(err) => return ApiResponse::error(500, format!("Failed to start transaction: {}", err)),
    };
    
    let ids = user_ids.into_inner();
    let mut deleted_count = 0;
    
    for user_id in &ids {
        match User::delete_by_id(*user_id).exec(&txn).await {
            Ok(res) => deleted_count += res.rows_affected,
            Err(err) => {
                // 回滚事务
                if let Err(rollback_err) = txn.rollback().await {
                    log::error!("Failed to rollback transaction: {}", rollback_err);
                }
                return ApiResponse::error(500, format!("Failed to delete user {}: {}", user_id, err));
            }
        }
    }
    
    // 提交事务
    match txn.commit().await {
        Ok(_) => ApiResponse::success(serde_json::json!({ 
            "message": format!("Successfully deleted {} users", deleted_count),
            "deleted_count": deleted_count 
        })),
        Err(err) => ApiResponse::error(500, format!("Failed to commit transaction: {}", err)),
    }
}

// --------------------------
// 12. 获取用户及其订单（使用关联查询）
// --------------------------
#[get("/users/{id}/orders/join")]
async fn get_user_with_orders_join(
    db: web::Data<DatabaseConnection>,
    id: web::Path<u64>,
) -> impl Responder {
    let uid = id.into_inner();
    
    // 使用关联查询获取用户及其订单
    match User::find_by_id(uid)
        .find_with_related(crate::entity::orders::Entity)
        .all(db.get_ref())
        .await
    {
        Ok(result) => {
            if result.is_empty() {
                return ApiResponse::error(404, "User not found");
            }
            
            let (user, orders) = result.into_iter().next().unwrap();
            let resp = serde_json::json!({"user": user, "orders": orders});
            ApiResponse::success(resp)
        },
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 13. 获取所有用户及其订单（关联查询）
// --------------------------
#[get("/users-with-orders")]
async fn get_all_users_with_orders(
    db: web::Data<DatabaseConnection>,
) -> impl Responder {
    // 使用关联查询获取所有用户及其订单
    match User::find()
        .find_with_related(crate::entity::orders::Entity)
        .all(db.get_ref())
        .await
    {
        Ok(results) => {
            let users_with_orders: Vec<serde_json::Value> = results
                .into_iter()
                .map(|(user, orders)| {
                    serde_json::json!({
                        "user": user,
                        "orders": orders
                    })
                })
                .collect();
                
            ApiResponse::success(users_with_orders)
        },
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// --------------------------
// 14. 获取订单统计信息
// --------------------------
#[get("/orders/stats")]
async fn get_orders_stats(
    db: web::Data<DatabaseConnection>,
) -> impl Responder {
    // 获取订单总数
    let total_orders = match crate::entity::orders::Entity::find().count(db.get_ref()).await {
        Ok(count) => count,
        Err(err) => return ApiResponse::error(500, format!("Failed to get total orders: {}", err)),
    };
    
    // 获取商品总数
    let total_goods_result: Result<Option<Option<i64>>, sea_orm::DbErr> = crate::entity::orders::Entity::find()
        .select_only()
        .column_as(crate::entity::orders::Column::Goods.sum(), "total_goods")
        .into_tuple::<Option<i64>>()
        .one(db.get_ref())
        .await;
        
    let total_goods = match total_goods_result {
        Ok(Some(sum)) => sum.unwrap_or(0),
        Ok(None) => 0,
        Err(err) => return ApiResponse::error(500, format!("Failed to get total goods: {}", err)),
    };
    
    ApiResponse::success(serde_json::json!({
        "total_orders": total_orders,
        "total_goods": total_goods
    }))
}

// --------------------------
// 15. 条件查询用户（复合条件）
// --------------------------
#[get("/users/conditional")]
async fn get_users_conditional(
    db: web::Data<DatabaseConnection>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let mut query_builder = User::find();
    
    // 根据查询参数添加条件
    if let Some(name) = query.get("name") {
        query_builder = query_builder.filter(crate::entity::users::Column::Name.contains(name));
    }
    
    if let Some(id_str) = query.get("id") {
        if let Ok(id) = id_str.parse::<u64>() {
            query_builder = query_builder.filter(crate::entity::users::Column::Id.eq(id));
        }
    }
    
    // 添加创建时间范围查询
    if let (Some(start_str), Some(end_str)) = (query.get("created_after"), query.get("created_before")) {
        if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>()) {
            // 注意：这里假设用户实体有 createTime 字段，但实际上没有
            // 如果有相应的字段，可以添加类似下面的条件：
            // query_builder = query_builder.filter(crate::entity::users::Column::CreateTime.between(start, end));
        }
    }
    
    match query_builder.all(db.get_ref()).await {
        Ok(users) => ApiResponse::success(users),
        Err(err) => ApiResponse::error(500, err.to_string()),
    }
}

// 注册路由到 /seaorm
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/seaorm")
            .service(add_user)
            .service(get_user)
            .service(get_users_page)
            .service(update_user)
            .service(delete_user)
            .service(get_user_with_orders)
            // 新增的路由
            .service(search_users_by_name)
            .service(get_user_stats)
            .service(create_users_batch)
            .service(get_users_sorted)
            .service(delete_users_batch)
            .service(get_user_with_orders_join)
            .service(get_all_users_with_orders)
            .service(get_orders_stats)
            .service(get_users_conditional)
    );
}
