//! 插件路由声明 — 定义本插件对外暴露的 HTTP 端点。
//!
//! 每个路由绑定一个 handler 函数指针，挂载在 `/plugin-api/<plugin-id>/` 命名空间下。

use crate::handler;
use http::Method;
use plugkit::plugin::{AuthRequirement, Plugin, PluginRoute, PluginRouteHandler};

/// 本插件声明的 HTTP 路由列表。
pub fn routes() -> Vec<PluginRoute> {
    vec![
        route(Method::GET, "/whoami", handler::handle_whoami, AuthRequirement::Public),
        route(Method::GET, "/items", handler::handle_list_items, AuthRequirement::Public),
        route(Method::POST, "/items", handler::handle_create_item, AuthRequirement::Public),
        route(Method::PUT, "/items", handler::handle_update_item, AuthRequirement::Public),
        route(Method::DELETE, "/items", handler::handle_delete_item, AuthRequirement::Public),
    ]
}

fn route(
    method: Method,
    path: &str,
    handler: PluginRouteHandler,
    auth: AuthRequirement,
) -> PluginRoute {
    PluginRoute {
        method,
        path: path.to_string(),
        handler,
        auth,
    }
}
