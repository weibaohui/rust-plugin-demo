//! 插件路由声明 — 定义本插件对外暴露的 HTTP 端点。
//!
//! 每个路由绑定一个 handler 函数指针，挂载在 `/plugin-api/<plugin-id>/` 命名空间下。

use crate::handler;
use http::Method;
use plugkit::plugin::{Plugin, PluginRoute, PluginRouteHandler};

/// 本插件声明的 HTTP 路由列表。
pub fn routes() -> Vec<PluginRoute> {
    vec![
        route(Method::GET, "/items", handler::handle_list_items),
        route(Method::POST, "/items", handler::handle_create_item),
        route(Method::PUT, "/items", handler::handle_update_item),
        route(Method::DELETE, "/items", handler::handle_delete_item),
    ]
}

fn route(method: Method, path: &str, handler: PluginRouteHandler) -> PluginRoute {
    PluginRoute {
        method,
        path: path.to_string(),
        handler,
    }
}
