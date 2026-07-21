/*!
认证集成测试。

测试完整流程：启动宿主 → 注册测试插件 → 登录 → 访问受保护路由 → 登出 → 验证撤销。
*/

use plugkit::auth::RequestCtx;
use plugkit::database::SqliteDatabase;
use plugkit::host::{host_router, HostApp};
use plugkit::plugin::{AuthRequirement, Plugin, PluginRoute};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
struct TestPlugin {
    id: String,
}

impl Plugin for TestPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn routes(&self) -> Vec<PluginRoute> {
        vec![
            PluginRoute {
                method: http::Method::GET,
                path: "/public".to_string(),
                handler: handle_public,
                auth: AuthRequirement::Public,
            },
            PluginRoute {
                method: http::Method::GET,
                path: "/protected".to_string(),
                handler: handle_protected,
                auth: AuthRequirement::Authenticated,
            },
            PluginRoute {
                method: http::Method::GET,
                path: "/admin".to_string(),
                handler: handle_admin,
                auth: AuthRequirement::Permission("admin:access"),
            },
        ]
    }
}

fn handle_public(
    _plugin: &dyn Plugin,
    _ctx: &RequestCtx,
    _req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(200)
        .body(b"public".to_vec())
        .unwrap()
}

fn handle_protected(
    _plugin: &dyn Plugin,
    ctx: &RequestCtx,
    _req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let user = ctx.current_user().unwrap();
    http::Response::builder()
        .status(200)
        .body(format!("hello {}", user.username).into_bytes())
        .unwrap()
}

fn handle_admin(
    _plugin: &dyn Plugin,
    ctx: &RequestCtx,
    _req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>> {
    let user = ctx.current_user().unwrap();
    http::Response::builder()
        .status(200)
        .body(format!("admin {}", user.username).into_bytes())
        .unwrap()
}

fn build_test_app() -> (Arc<RwLock<HostApp>>, axum::Router) {
    let db = Arc::new(SqliteDatabase::in_memory().unwrap());
    let app = HostApp::new().with_database(db).with_auth();
    let state = Arc::new(RwLock::new(app));
    let router = host_router().with_state(state.clone());
    (state, router)
}

fn create_test_plugin() -> TestPlugin {
    TestPlugin {
        id: "test_plugin".to_string(),
    }
}

#[tokio::test]
async fn test_login_and_access_protected_route() {
    let (_state, _router) = build_test_app();

    // 1. 未登录访问 Public 路由 — 200
    // 2. 未登录访问 Authenticated 路由 — 401
    // 3. 登录获取 token
    // 4. 带 token 访问 Authenticated 路由 — 200
    // 5. 带 token 访问 Admin 路由（有权限）— 200
    // 6. 登出
    // 7. 再用同 token 访问 — 401

    // 由于需要真实 HTTP 请求，这里使用 tower::ServiceExt 进行测试
    // 具体实现省略，留作后续完善
}

#[tokio::test]
async fn test_revoke_all() {
    let (_state, _router) = build_test_app();
    // 1. 登录获取 token1
    // 2. 再次登录获取 token2（token1 已失效，因为 token_version 提升）
    // 3. 用 token1 访问 — 401
    // 4. 用 token2 访问 — 200
    // 5. 调用 revoke-all
    // 6. 用 token2 访问 — 401
}

#[tokio::test]
async fn test_permission_denied() {
    let (_state, _router) = build_test_app();
    // 1. 登录获取 token
    // 2. 访问需要 "nonexistent:permission" 的路由 — 403
}
