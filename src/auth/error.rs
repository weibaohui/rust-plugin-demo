/*!
认证相关错误类型。
*/

use std::fmt::{Display, Formatter};

/// 认证错误。
#[derive(Debug)]
pub enum AuthError {
    /// Token 无效或已过期。
    InvalidToken(String),
    /// Token 已被撤销。
    TokenRevoked,
    /// 用户名或密码错误。
    InvalidCredentials,
    /// 权限不足。
    Forbidden(String),
    /// 用户不存在。
    UserNotFound(String),
    /// 数据库操作失败。
    DatabaseError(String),
    /// 密码哈希失败。
    PasswordHashError(String),
    /// JWT 操作失败。
    JwtError(String),
    /// 内部错误。
    Internal(String),
}

impl Display for AuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidToken(msg) => write!(f, "Token 无效: {}", msg),
            AuthError::TokenRevoked => write!(f, "Token 已被撤销"),
            AuthError::InvalidCredentials => write!(f, "用户名或密码错误"),
            AuthError::Forbidden(msg) => write!(f, "权限不足: {}", msg),
            AuthError::UserNotFound(msg) => write!(f, "用户不存在: {}", msg),
            AuthError::DatabaseError(msg) => write!(f, "数据库错误: {}", msg),
            AuthError::PasswordHashError(msg) => write!(f, "密码哈希错误: {}", msg),
            AuthError::JwtError(msg) => write!(f, "JWT 错误: {}", msg),
            AuthError::Internal(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<bcrypt::BcryptError> for AuthError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AuthError::PasswordHashError(e.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AuthError::JwtError(e.to_string())
    }
}
