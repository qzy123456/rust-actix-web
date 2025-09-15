use actix_web::{web, HttpRequest, HttpResponse, Error, dev::{Service, ServiceRequest, ServiceResponse, Transform}};
use futures::{Future, FutureExt}; 
use std::pin::Pin; 
use std::task::{Context, Poll}; 
use std::fmt::Display; 
use log::{error, warn, info}; 
use backtrace::Backtrace;
use serde_json::json; 
use std::sync::{Arc, Mutex}; 
use super::{JsonLogger, LogLevel}; 

// 错误处理中间件
#[derive(Clone, Debug)]
pub struct ErrorHandler;

// 中间件的工厂实现
impl<S, B> Transform<S, ServiceRequest> for ErrorHandler 
where 
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>, 
    S::Future: 'static, 
    B: 'static, 
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlerMiddleware<S>;
    type Future = futures::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        // 添加初始化日志，确认中间件被正确创建
        info!("[ERROR MIDDLEWARE] 初始化错误处理中间件");
        futures::future::ready(Ok(ErrorHandlerMiddleware { service }))
    }
} 

// 中间件的具体实现
pub struct ErrorHandlerMiddleware<S> {
    service: S,
} 

// 服务实现
impl<S, B> Service<ServiceRequest> for ErrorHandlerMiddleware<S> 
where 
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>, 
    S::Future: 'static, 
    B: 'static, 
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 获取请求信息用于日志记录 
        let path = req.path().to_string();
        let method = req.method().to_string();
        let headers = req.headers().clone();
        let version = req.version();
        
        // 克隆请求头信息用于JSON日志
        let headers_clone = headers.clone();
        
        // 添加明确的中间件调用日志
        info!("[ERROR MIDDLEWARE] 收到请求: {} {}", method, path);
        
        // 检查是否有JSON日志器
        let json_logger = req.app_data::<web::Data<Arc<Mutex<JsonLogger>>>>().cloned();
        
        // 如果有JSON日志器，记录请求信息
        if let Some(logger) = &json_logger {
            if let Ok(mut logger_guard) = logger.lock() {
                let request_data = json!({
                    "method": method.clone(),
                    "path": path.clone(),
                    "http_version": format!("{:?}", version),
                    "headers": headers_clone.iter()
                        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                        .collect::<serde_json::Value>()
                });
                
                let _ = logger_guard.log_with_data(
                    LogLevel::INFO, 
                    "[ERROR MIDDLEWARE] 收到请求", 
                    request_data
                );
            }
        }
        
        // 调用后续服务并处理结果 
        let fut = self.service.call(req);
        
        Box::pin(fut.then(move |result| async move {
            // 添加日志记录result的类型，确认中间件在处理结果
            info!("[ERROR MIDDLEWARE] 处理请求结果: {} {}, 结果类型: {}", method, path, if result.is_ok() { "成功" } else { "失败" });
            
            match result {
                // 如果没有错误，检查状态码并返回响应 
                Ok(response) => {
                    // 检查响应状态码
                    let status_code = response.status();
                    let status_code_num = status_code.as_u16();
                    
                    // 记录状态码信息
                    info!("[ERROR MIDDLEWARE] 响应状态码: {} ({}) for {} {}", 
                          status_code_num, status_code.canonical_reason().unwrap_or("Unknown"), method, path);
                    
                    // 对于错误状态码，添加详细日志
                    if status_code.is_client_error() || status_code.is_server_error() {
                        info!("[ERROR MIDDLEWARE] 检测到错误状态码: {} {}", status_code_num, path);
                        
                        // 使用error级别确保日志能被看到
                        error!("[ERROR MIDDLEWARE] 捕获到错误响应: {} {} 状态码: {}", 
                               method, path, status_code_num);
                        
                        // 如果有JSON日志器，记录错误状态码信息
                        if let Some(logger) = &json_logger {
                            if let Ok(mut logger_guard) = logger.lock() {
                                let error_data = json!({"status_code": status_code_num, "status_text": status_code.canonical_reason()});
                                let _ = logger_guard.log_with_data(
                                    if status_code.is_server_error() { LogLevel::ERROR } else { LogLevel::WARNING }, 
                                    &format!("[ERROR MIDDLEWARE] 捕获到错误响应: {} {} 状态码: {}", method, path, status_code_num), 
                                    error_data
                                );
                            }
                        }
                    }
                    
                    Ok(response)
                },
                
                // 如果有错误，记录错误信息和堆栈 
                Err(err) => {
                    // 使用error级别确保日志能被看到
                    error!("[ERROR MIDDLEWARE] 请求处理失败: {} {} - 错误: {}", method, path, err);
                    
                    // 记录错误堆栈
                    let backtrace = Backtrace::new();
                    error!("[ERROR MIDDLEWARE] 错误堆栈: {:?}", backtrace);
                    
                    // 如果有JSON日志器，记录错误信息
                    if let Some(logger) = &json_logger {
                        if let Ok(mut logger_guard) = logger.lock() {
                            let error_data = json!({"error": format!("{:?}", err), "stack_trace": format!("{:?}", backtrace)});
                            let _ = logger_guard.log_with_data(
                                LogLevel::ERROR, 
                                &format!("[ERROR MIDDLEWARE] 请求处理失败: {} {}", method, path), 
                                error_data
                            );
                        }
                    }
                    
                    // 返回错误响应 
                    Err(err)
                }
            }
        }))
    }
} 

// 自定义错误类型，用于包装其他错误并提供更详细的上下文
#[derive(Debug)] 
pub struct ApiError { 
    pub code: u16, 
    pub message: String, 
    pub details: Option<String>, 
} 

// 实现Display trait用于格式化输出
impl Display for ApiError { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        write!(f, "Error {}: {}", self.code, self.message)?; 
        if let Some(details) = &self.details { 
            write!(f, " - {}", details)?; 
        } 
        Ok(()) 
    } 
} 

// 实现Error trait
impl std::error::Error for ApiError {} 

// 实现From<ApiError> for actix_web::Error
impl From<ApiError> for actix_web::Error { 
    fn from(err: ApiError) -> Self { 
        let backtrace = Backtrace::new(); 
        error!( 
            "ApiError {}: {}\nStack trace:\n{:?}", 
            err.code, err.message, backtrace 
        ); 
        
        match err.code { 
            400 => actix_web::error::ErrorBadRequest(err.message), 
            401 => actix_web::error::ErrorUnauthorized(err.message), 
            403 => actix_web::error::ErrorForbidden(err.message), 
            404 => actix_web::error::ErrorNotFound(err.message), 
            500 => actix_web::error::ErrorInternalServerError(err.message), 
            _ => actix_web::error::ErrorInternalServerError(err.message), 
        } 
    } 
}