/*!
axum 认证中间件。
*/

use crate::auth::ctx::RequestCtx;
use crate::auth::error::AuthError;
use crate::auth::service::AuthService;
use crate::database::DatabaseExt;
use crate::plugin::AuthRequirement;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// 认证中间件状态。
#[derive(Clone)]
pub struct AuthMiddlewareState {
    pub auth_service: Arc<dyn AuthService>,
    pub db: Arc<dyn DatabaseExt>,
    pub event_bus: Arc<crate::event_bus::EventBus>,
}

/// 认证中间件。
///
/// 解析 Authorization header，验证 token，按路由声明的 AuthRequirement 进行拦截。
/// 成功后将 RequestCtx 注入 request extensions。
pub async fn auth_middleware(
    State(state): State<AuthMiddlewareState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // 从 request extensions 获取路由声明的 AuthRequirement
    let auth_requirement = req
        .extensions()
        .get::<AuthRequirement>()
        .cloned()
        .unwrap_or(AuthRequirement::Public);

    // 提取 token
    let token = extract_token(&req);

    // 验证 token
    let principal = match token {
        Some(t) => match state.auth_service.verify_token(&t) {
            Ok(p) => Some(p),
            Err(e) => return auth_error_response(&e),
        },
        None => None,
    };

    // 按 AuthRequirement 检查
    match (&auth_requirement, &principal) {
        (AuthRequirement::Public, _) => {}
        (AuthRequirement::Authenticated, Some(_)) => {}
        (AuthRequirement::Authenticated, None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "unauthorized",
                    "message": "需要登录"
                })),
            )
                .into_response();
        }
        (AuthRequirement::Permission(perm), Some(p)) => {
            if !p.has_permission(perm) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "error": "forbidden",
                        "message": "权限不足",
                        "required": perm
                    })),
                )
                    .into_response();
            }
        }
        (AuthRequirement::Permission(perm), None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "unauthorized",
                    "message": "需要登录",
                    "required": perm
                })),
            )
                .into_response();
        }
    }

    // 构造 RequestCtx 并注入 extensions
    let plugin_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let ctx = RequestCtx {
        principal,
        auth_service: state.auth_service.clone(),
        db: state.db.clone(),
        event_bus: state.event_bus.clone(),
        plugin_id,
        request_id: Uuid::new_v4().to_string(),
    };

    req.extensions_mut().insert(ctx);
    next.run(req).await
}

/// 从请求头提取 Bearer token。
fn extract_token(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get(http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// 将认证错误转换为 HTTP 响应。
fn auth_error_response(e: &AuthError) -> Response {
    let (status, error_code, message) = match e {
        AuthError::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, "invalid_token", msg.clone()),
        AuthError::TokenRevoked => (
            StatusCode::UNAUTHORIZED,
            "token_revoked",
            "Token 已被撤销".to_string(),
        ),
        AuthError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "用户名或密码错误".to_string(),
        ),
        AuthError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
        AuthError::UserNotFound(msg) => (StatusCode::UNAUTHORIZED, "user_not_found", msg.clone()),
        AuthError::DatabaseError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error",
            msg.clone(),
        ),
        AuthError::PasswordHashError(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "password_hash_error",
            msg.clone(),
        ),
        AuthError::JwtError(msg) => (StatusCode::UNAUTHORIZED, "jwt_error", msg.clone()),
        AuthError::Internal(msg) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            msg.clone(),
        ),
    };

    (
        status,
        Json(json!({
            "error": error_code,
            "message": message
        })),
    )
        .into_response()
}
