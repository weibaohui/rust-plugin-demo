/*!
认证模块。

提供框架级登录认证体系，包括：
- JWT 签发/验证
- 基于 bcrypt 的密码哈希
- 声明式路由认证要求
- 请求上下文（RequestCtx）聚合插件所需能力

插件通过 `ctx.auth_service` 验证用户真伪，不重复实现验证逻辑。
*/

pub mod ctx;
pub mod error;
pub mod jwt;
pub mod middleware;
pub mod principal;
pub mod routes;
pub mod service;

pub use ctx::RequestCtx;
pub use error::AuthError;
pub use jwt::{JwtClaims, JwtService};
pub use middleware::{auth_middleware, AuthMiddlewareState};
pub use principal::Principal;
pub use service::{AuthService, AuthServiceImpl};
