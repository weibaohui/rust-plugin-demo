/*!
请求上下文，聚合插件 handler 所需的所有能力。
*/

use crate::auth::principal::Principal;
use crate::auth::service::AuthService;
use crate::database::DatabaseExt;
use std::sync::Arc;

/// 请求上下文。
///
/// 替代原有的 `db` 参数，成为插件 handler 的统一入口。
/// 包含认证信息、数据库访问、事件总线等能力。
#[derive(Debug, Clone)]
pub struct RequestCtx {
    /// 当前认证用户。Public 路由为 `None`。
    pub principal: Option<Principal>,

    /// 认证服务引用。插件可用它验证其他 token。
    pub auth_service: Arc<dyn AuthService>,

    /// 数据库访问。
    pub db: Arc<dyn DatabaseExt>,

    /// 事件总线。
    pub event_bus: Arc<crate::event_bus::EventBus>,

    /// 当前插件 ID。
    pub plugin_id: String,

    /// 请求唯一标识（用于日志追踪）。
    pub request_id: String,
}

impl RequestCtx {
    /// 获取当前用户，若未认证则返回 `None`。
    pub fn current_user(&self) -> Option<&Principal> {
        self.principal.as_ref()
    }

    /// 获取当前用户，若未认证则返回错误。
    pub fn require_user(&self) -> Result<&Principal, crate::auth::error::AuthError> {
        self.principal
            .as_ref()
            .ok_or_else(|| crate::auth::error::AuthError::InvalidToken("未认证".to_string()))
    }

    /// 检查当前用户是否拥有指定权限。
    pub fn has_permission(&self, permission: &str) -> bool {
        self.principal
            .as_ref()
            .map(|p| p.has_permission(permission))
            .unwrap_or(false)
    }
}
