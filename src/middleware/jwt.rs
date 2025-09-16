use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage, web, HttpRequest}; 
use actix_web::dev::{Transform, Service}; 
use futures::{future::{ok, Ready}, Future}; 
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation, errors::Error as JwtError}; 
use serde::{Deserialize, Serialize}; 
use std::pin::Pin; 
use std::task::{Context, Poll}; 
use std::time::{Duration, SystemTime, UNIX_EPOCH}; 
use log::{info, error}; 

// JWT声明结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 用户名或用户ID
    pub user_id: u64, // 用户ID
    pub exp: u64, // 过期时间（UNIX时间戳）
    pub iat: u64, // 签发时间
}

// JWT中间件配置
#[derive(Clone)]
pub struct JwtMiddleware {
    secret_key: String, 
    algorithm: Algorithm, 
}

impl Default for JwtMiddleware {
    fn default() -> Self {
        Self {
            secret_key: "your-default-secret-key".to_string(), // 生产环境应该从环境变量读取
            algorithm: Algorithm::HS256,
        }
    }
}

impl JwtMiddleware {
    // 创建新的JWT中间件实例
    pub fn new(secret_key: String) -> Self {
        Self {
            secret_key,
            algorithm: Algorithm::HS256,
        }
    }
    
    // 签发JWT令牌
    pub fn generate_token(&self, user_id: u64, username: String, expires_in: Duration) -> Result<String, JwtError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
            
        let claims = Claims {
            sub: username,
            user_id,
            exp: now + expires_in.as_secs(),
            iat: now,
        };
        
        encode(
            &Header::new(self.algorithm),
            &claims,
            &EncodingKey::from_secret(self.secret_key.as_ref()),
        )
    }
    
    // 验证JWT令牌
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        decode(
            token, 
            &DecodingKey::from_secret(self.secret_key.as_ref()), 
            &Validation::new(self.algorithm),
        ).map(|data| data.claims)
    }
    
    // 从请求头中提取JWT令牌
    fn extract_token(&self, req: &ServiceRequest) -> Option<String> {
        req.headers()
            .get("Authorization")
            .and_then(|header| header.to_str().ok())
            .and_then(|auth_header| {
                if auth_header.starts_with("Bearer ") {
                    Some(auth_header[7..].to_owned())
                } else {
                    None
                }
            })
    }
}

// 实现Transform trait，用于创建中间件
impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware 
where 
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>, 
    S::Future: 'static,
    B: 'static, 
{
    type Response = ServiceResponse<B>; 
    type Error = Error; 
    type InitError = (); 
    type Transform = JwtAuthMiddleware<S>; 
    type Future = Ready<Result<Self::Transform, Self::InitError>>; 

    fn new_transform(&self, service: S) -> Self::Future {
        info!("JWT中间件初始化完成");
        ok(JwtAuthMiddleware {
            service,
            secret_key: self.secret_key.clone(),
            algorithm: self.algorithm,
        })
    }
}

// JWT认证中间件的具体实现
pub struct JwtAuthMiddleware<S> {
    service: S,
    secret_key: String,
    algorithm: Algorithm,
}

// 实现Service trait，处理请求
impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S> 
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
        let path = req.path().to_string();
        let method = req.method().to_string();
        // 跳过认证的路径（如登录、注册、健康检查等）
        if path.starts_with("/api/auth") || path == "/api/health" || path == "/api/logger"  || path == "/favicon.ico"{
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }
        
        // 创建JWT中间件实例用于验证
        let jwt_middleware = JwtMiddleware::new(self.secret_key.clone());
        
        // 提取并验证JWT令牌
        match jwt_middleware.extract_token(&req) {
            Some(token) => {
                match jwt_middleware.validate_token(&token) {
                    Ok(claims) => {
                        // 将用户信息存储在请求扩展中，以便后续处理函数使用
                        req.extensions_mut().insert(claims.user_id);
                        req.extensions_mut().insert(claims.sub);
                        
                        let fut = self.service.call(req);
                        Box::pin(async move { fut.await })
                    },
                    Err(e) => {
                        error!("JWT验证失败: {}, 路径: {}, 方法: {}", e, path, method);
                        // 返回错误，让Actix Web处理响应
                        Box::pin(async move { 
                            Err(actix_web::error::ErrorUnauthorized("Invalid or expired token")) 
                        })
                    },
                }
            },
            None => {
                error!("未提供JWT令牌, 路径: {}, 方法: {}", path, method);
                // 返回错误，让Actix Web处理响应
                Box::pin(async move { 
                    Err(actix_web::error::ErrorUnauthorized("Authorization token is missing")) 
                })
            },
        }
    }
}

// 认证错误
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    ExpiredToken,
    Other(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Authorization token is missing"),
            AuthError::InvalidToken => write!(f, "Invalid token"),
            AuthError::ExpiredToken => write!(f, "Token has expired"),
            AuthError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

// 从请求中提取用户信息的辅助函数
pub fn get_user_id_from_request(req: &HttpRequest) -> Option<u64> {
    req.extensions().get::<u64>().copied()
}

pub fn get_username_from_request(req: &HttpRequest) -> Option<String> {
    req.extensions().get::<String>().cloned()
}