/*!
已认证用户主体。
*/

/// 已认证用户的完整上下文。
///
/// 由宿主 middleware 构造，插件通过 `ctx.principal` 只读访问。
#[derive(Debug, Clone)]
pub struct Principal {
    /// 用户 ID。
    pub user_id: String,
    /// 用户名。
    pub username: String,
    /// 角色列表。
    pub roles: Vec<String>,
    /// 权限列表（登录时从 role_permissions 展开）。
    pub permissions: Vec<String>,
    /// Token 唯一标识，用于撤销。
    pub jti: String,
    /// Token 版本，与用户表中的 `token_version` 对比。
    pub token_version: u64,
}

impl Principal {
    /// 检查是否拥有指定权限。
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// 检查是否拥有指定角色。
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}
