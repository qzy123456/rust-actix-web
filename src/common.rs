use actix_web::{HttpResponse, HttpRequest, Responder};
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Meta {
    pub total_items: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub page_size: u64,
}

#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
    pub meta: Option<Value>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        ApiResponse {
            code: 200,
            message: "OK".to_string(),
            data: Some(data),
            meta: None,
        }
    }

    pub fn success_with_meta(data: T, meta: Meta) -> Self {
        ApiResponse {
            code: 200,
            message: "OK".to_string(),
            data: Some(data),
            meta: Some(serde_json::to_value(meta).unwrap_or(Value::Null)),
        }
    }

    pub fn error(code: u16, message: impl Into<String>) -> ApiResponse<T> {
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