/*!
认证服务 trait 与实现。
*/

use crate::auth::error::AuthError;
use crate::auth::jwt::JwtService;
use crate::auth::principal::Principal;
use crate::database::{DatabaseExt, DbValue};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::Arc;

/// 认证服务 trait。
///
/// 插件通过 `ctx.auth_service` 调用，**只读接口**，插件不能签发或撤销 token。
pub trait AuthService: Send + Sync + std::fmt::Debug {
    /// 验证 token，返回 Principal。
    fn verify_token(&self, token: &str) -> Result<Principal, AuthError>;

    /// 撤销单个 token（加入内存黑名单 + 更新 tokens 表）。
    fn revoke_token(&self, jti: &str) -> Result<(), AuthError>;

    /// 撤销某用户所有 token（token_version + 1）。
    fn revoke_user_tokens(&self, user_id: &str) -> Result<(), AuthError>;

    /// 向下转型为具体实现（宿主内部使用）。
    fn as_any(&self) -> &dyn std::any::Any;
}

/// 认证服务实现。
#[derive(Debug)]
pub struct AuthServiceImpl {
    jwt: JwtService,
    db: Arc<dyn DatabaseExt>,
    /// 内存黑名单：已撤销的 jti 集合。
    revoked_jtis: Arc<RwLock<HashSet<String>>>,
}

impl AuthServiceImpl {
    /// 创建新的认证服务。
    pub fn new(db: Arc<dyn DatabaseExt>) -> Self {
        Self {
            jwt: JwtService::new(),
            db,
            revoked_jtis: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// 从指定 JWT 服务创建（用于测试）。
    pub fn with_jwt_service(jwt: JwtService, db: Arc<dyn DatabaseExt>) -> Self {
        Self {
            jwt,
            db,
            revoked_jtis: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// 获取 JWT 服务引用（用于登录端点签发 token）。
    pub fn jwt_service(&self) -> &JwtService {
        &self.jwt
    }

    /// 检查 token 是否被撤销。
    fn is_revoked(&self, jti: &str) -> bool {
        self.revoked_jtis.read().contains(jti)
    }

    /// 从数据库检查 token 是否被撤销。
    fn is_revoked_in_db(&self, jti: &str) -> Result<bool, AuthError> {
        let rows = self
            .db
            .query_with(
                "SELECT revoked_at FROM tokens WHERE jti = ?",
                &[DbValue::text(jti)],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        match rows.first() {
            Some(row) => match row.first() {
                Some(DbValue::Null) => Ok(false),
                Some(_) => Ok(true),
                None => Ok(false),
            },
            None => Ok(false),
        }
    }

    /// 从数据库获取用户的当前 token_version。
    fn get_user_token_version(&self, user_id: &str) -> Result<u64, AuthError> {
        let rows = self
            .db
            .query_with(
                "SELECT token_version FROM users WHERE id = ?",
                &[DbValue::text(user_id)],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        match rows.first() {
            Some(row) => match row.first() {
                Some(DbValue::Int(v)) => Ok(*v as u64),
                _ => Err(AuthError::UserNotFound(user_id.to_string())),
            },
            None => Err(AuthError::UserNotFound(user_id.to_string())),
        }
    }
}

impl AuthService for AuthServiceImpl {
    fn verify_token(&self, token: &str) -> Result<Principal, AuthError> {
        // 1. 验证 JWT 签名和过期时间
        let claims = self.jwt.verify_token(token)?;

        // 2. 检查内存黑名单
        if self.is_revoked(&claims.jti) {
            return Err(AuthError::TokenRevoked);
        }

        // 3. 检查数据库撤销状态
        if self.is_revoked_in_db(&claims.jti)? {
            // 同步到内存黑名单
            self.revoked_jtis.write().insert(claims.jti.clone());
            return Err(AuthError::TokenRevoked);
        }

        // 4. 检查 token_version
        let current_version = self.get_user_token_version(&claims.sub)?;
        if claims.token_version != current_version {
            return Err(AuthError::TokenRevoked);
        }

        Ok(Principal::from(claims))
    }

    fn revoke_token(&self, jti: &str) -> Result<(), AuthError> {
        // 1. 加入内存黑名单
        self.revoked_jtis.write().insert(jti.to_string());

        // 2. 更新数据库
        self.db
            .execute_with(
                "UPDATE tokens SET revoked_at = datetime('now') WHERE jti = ?",
                &[DbValue::text(jti)],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn revoke_user_tokens(&self, user_id: &str) -> Result<(), AuthError> {
        // 1. 提升 token_version
        self.db
            .execute_with(
                "UPDATE users SET token_version = token_version + 1, updated_at = datetime('now') WHERE id = ?",
                &[DbValue::text(user_id)],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // 2. 撤销该用户所有未过期 token
        self.db
            .execute_with(
                "UPDATE tokens SET revoked_at = datetime('now') WHERE user_id = ? AND revoked_at IS NULL",
                &[DbValue::text(user_id)],
            )
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
