/*!
认证相关 HTTP 端点。
*/

use crate::auth::error::AuthError;
use crate::auth::jwt::JwtService;
use crate::auth::service::AuthService;
use crate::database::{DatabaseExt, DbValue};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, RwLock};

/// 登录请求。
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应。
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    pub expires_at: String,
}

/// 用户信息。
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 从 SharedState 提取认证路由状态。
fn extract_auth_state(
    state: &Arc<RwLock<crate::host::HostApp>>,
) -> Result<(Arc<dyn AuthService>, Arc<dyn DatabaseExt>), AuthError> {
    let ctx = state
        .read()
        .map_err(|_| AuthError::Internal("锁失败".into()))?;
    let auth_service = ctx
        .auth_service
        .clone()
        .ok_or_else(|| AuthError::Internal("认证未启用".into()))?;
    let db = ctx
        .manager
        .database()
        .clone()
        .ok_or_else(|| AuthError::Internal("数据库未配置".into()))?;
    Ok((auth_service, db))
}

/// POST /auth/login
pub async fn handle_login(
    State(state): State<Arc<RwLock<crate::host::HostApp>>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let (_auth_service, db) = match extract_auth_state(&state) {
        Ok(v) => v,
        Err(e) => return auth_error_response(&e),
    };

    // 获取 JWT service（通过 downcast）
    let jwt_service = {
        let ctx = state.read().unwrap();
        ctx.auth_service
            .as_ref()
            .and_then(|s| {
                s.as_any()
                    .downcast_ref::<crate::auth::service::AuthServiceImpl>()
            })
            .map(|impl_| impl_.jwt_service().clone())
    };

    let jwt_service = match jwt_service {
        Some(s) => s,
        None => {
            return auth_error_response(&AuthError::Internal("JWT 服务不可用".into()));
        }
    };

    match login(&db, &jwt_service, &req.username, &req.password).await {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response(),
        Err(e) => auth_error_response(&e),
    }
}

async fn login(
    db: &Arc<dyn DatabaseExt>,
    jwt_service: &JwtService,
    username: &str,
    password: &str,
) -> Result<LoginResponse, AuthError> {
    // 1. 查询用户
    let rows = db
        .query_with(
            "SELECT id, username, password_hash, token_version FROM users WHERE username = ?",
            &[DbValue::text(username)],
        )
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    let row = rows.first().ok_or_else(|| AuthError::InvalidCredentials)?;

    let user_id = match &row[0] {
        DbValue::Int(v) => v.to_string(),
        _ => return Err(AuthError::Internal("用户 ID 格式错误".to_string())),
    };
    let db_username = match &row[1] {
        DbValue::Text(v) => v.clone(),
        _ => return Err(AuthError::Internal("用户名格式错误".to_string())),
    };
    let password_hash = match &row[2] {
        DbValue::Text(v) => v.clone(),
        _ => return Err(AuthError::Internal("密码哈希格式错误".to_string())),
    };
    let token_version = match &row[3] {
        DbValue::Int(v) => *v as u64,
        _ => return Err(AuthError::Internal("token_version 格式错误".to_string())),
    };

    // 2. 验证密码
    let valid = bcrypt::verify(password, &password_hash)?;
    if !valid {
        return Err(AuthError::InvalidCredentials);
    }

    // 3. 提升 token_version（使旧 token 失效）
    let new_version = token_version + 1;
    db.execute_with(
        "UPDATE users SET token_version = ?, updated_at = datetime('now') WHERE id = ?",
        &[DbValue::int(new_version as i64), DbValue::text(&user_id)],
    )
    .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    // 4. 查询角色和权限
    let roles = get_user_roles(&user_id)?;
    let permissions = get_user_permissions(db, &roles)?;

    // 5. 签发 token
    let ttl = 24 * 3600; // 24 小时
    let (token, jti) = jwt_service.issue_token(
        &user_id,
        &db_username,
        roles.clone(),
        permissions.clone(),
        new_version,
        ttl,
    )?;

    // 6. 记录 token
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl as i64);
    db.execute_with(
        "INSERT INTO tokens (jti, user_id, expires_at) VALUES (?, ?, ?)",
        &[
            DbValue::text(&jti),
            DbValue::text(&user_id),
            DbValue::text(expires_at.to_rfc3339()),
        ],
    )
    .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    Ok(LoginResponse {
        token,
        user: UserInfo {
            id: user_id,
            username: db_username,
            roles,
            permissions,
        },
        expires_at: expires_at.to_rfc3339(),
    })
}

fn get_user_roles(_user_id: &str) -> Result<Vec<String>, AuthError> {
    // 当前单管理员设计，默认 admin 角色
    // 未来扩展：查询 user_roles 表
    Ok(vec!["admin".to_string()])
}

fn get_user_permissions(
    db: &Arc<dyn DatabaseExt>,
    roles: &[String],
) -> Result<Vec<String>, AuthError> {
    if roles.is_empty() {
        return Ok(vec![]);
    }

    // 构建 IN 查询的占位符
    let placeholders: Vec<&str> = roles.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT DISTINCT permission FROM role_permissions WHERE role IN ({})",
        placeholders.join(",")
    );

    let params: Vec<DbValue> = roles.iter().map(|r| DbValue::text(r)).collect();
    let rows = db
        .query_with(&sql, &params)
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    let permissions: Vec<String> = rows
        .iter()
        .filter_map(|row| match row.first() {
            Some(DbValue::Text(p)) => Some(p.clone()),
            _ => None,
        })
        .collect();

    Ok(permissions)
}

/// POST /auth/logout
pub async fn handle_logout(
    State(state): State<Arc<RwLock<crate::host::HostApp>>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (auth_service, _db) = match extract_auth_state(&state) {
        Ok(v) => v,
        Err(e) => return auth_error_response(&e),
    };

    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized", "message": "缺少 token"})),
            )
                .into_response();
        }
    };

    // 解析 token 获取 jti（不验证签名，因为可能已经过期）
    let jti = match parse_jti_without_verify(&token) {
        Some(j) => j,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid_token", "message": "无法解析 token"})),
            )
                .into_response();
        }
    };

    match auth_service.revoke_token(&jti) {
        Ok(()) => (StatusCode::OK, Json(json!({"message": "已登出"}))).into_response(),
        Err(e) => auth_error_response(&e),
    }
}

/// POST /auth/revoke-all
pub async fn handle_revoke_all(
    State(state): State<Arc<RwLock<crate::host::HostApp>>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (auth_service, _db) = match extract_auth_state(&state) {
        Ok(v) => v,
        Err(e) => return auth_error_response(&e),
    };

    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized", "message": "缺少 token"})),
            )
                .into_response();
        }
    };

    // 先验证 token 获取 user_id
    let principal = match auth_service.verify_token(&token) {
        Ok(p) => p,
        Err(e) => return auth_error_response(&e),
    };

    match auth_service.revoke_user_tokens(&principal.user_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "message": "所有会话已撤销",
                "token_version": principal.token_version + 1
            })),
        )
            .into_response(),
        Err(e) => auth_error_response(&e),
    }
}

/// GET /auth/me
pub async fn handle_me(
    State(state): State<Arc<RwLock<crate::host::HostApp>>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (auth_service, _db) = match extract_auth_state(&state) {
        Ok(v) => v,
        Err(e) => return auth_error_response(&e),
    };

    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized", "message": "缺少 token"})),
            )
                .into_response();
        }
    };

    match auth_service.verify_token(&token) {
        Ok(p) => (
            StatusCode::OK,
            Json(json!({
                "id": p.user_id,
                "username": p.username,
                "roles": p.roles,
                "permissions": p.permissions
            })),
        )
            .into_response(),
        Err(e) => auth_error_response(&e),
    }
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

fn parse_jti_without_verify(token: &str) -> Option<String> {
    // 简单解析 payload（不验证签名）
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    claims.get("jti")?.as_str().map(|s| s.to_string())
}

fn auth_error_response(e: &AuthError) -> axum::response::Response {
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
