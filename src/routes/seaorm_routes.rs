use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;
use serde_json::Value;
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;

use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set, QueryFilter, ColumnTrait};
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
    );
}
