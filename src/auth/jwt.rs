/*!
JWT 签发与验证。
*/

use crate::auth::error::AuthError;
use crate::auth::principal::Principal;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims 结构。
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// 用户 ID。
    pub sub: String,
    /// 用户名。
    pub username: String,
    /// 角色列表。
    pub roles: Vec<String>,
    /// 权限列表。
    pub permissions: Vec<String>,
    /// Token 版本。
    pub token_version: u64,
    /// Token 唯一标识。
    pub jti: String,
    /// 签发时间。
    pub iat: u64,
    /// 过期时间。
    pub exp: u64,
}

impl From<JwtClaims> for Principal {
    fn from(claims: JwtClaims) -> Self {
        Principal {
            user_id: claims.sub,
            username: claims.username,
            roles: claims.roles,
            permissions: claims.permissions,
            jti: claims.jti,
            token_version: claims.token_version,
        }
    }
}

/// JWT 服务，负责签发和验证 token。
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService").finish_non_exhaustive()
    }
}

impl JwtService {
    /// 创建新的 JWT 服务，使用随机生成的密钥。
    pub fn new() -> Self {
        let mut secret = [0u8; 32];
        rand::thread_rng().fill(&mut secret);
        Self::from_secret(&secret)
    }

    /// 从指定密钥创建 JWT 服务。
    pub fn from_secret(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
        }
    }

    /// 签发 token。
    pub fn issue_token(
        &self,
        user_id: &str,
        username: &str,
        roles: Vec<String>,
        permissions: Vec<String>,
        token_version: u64,
        ttl_secs: u64,
    ) -> Result<(String, String), AuthError> {
        let now = chrono::Utc::now().timestamp() as u64;
        let jti = Uuid::new_v4().to_string();
        let claims = JwtClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            roles,
            permissions,
            token_version,
            jti: jti.clone(),
            iat: now,
            exp: now + ttl_secs,
        };
        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok((token, jti))
    }

    /// 验证 token，返回 Claims。
    pub fn verify_token(&self, token: &str) -> Result<JwtClaims, AuthError> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &Validation::default())?;
        Ok(token_data.claims)
    }
}

use rand::Rng;
