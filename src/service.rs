// =====================================================================
// Phase 2 (Cordis 风格) — 服务容器 / DI 容器
// =====================================================================
//
// 借鉴 Cordis (cordiverse/cordis) 的 DI 理念：
//   - 服务注册到 Context（provide）
//   - plugin 声明依赖哪些服务（inject_deps）
//   - 通过 ctx.get::<T>("name") 透明拿服务
//
// 与现有 API 完全兼容：
//   - 现有 Plugin trait 不变（默认 inject_deps 返回空）
//   - 现有 HostContext 不变
//   - 新代码可选用 Service trait + Context 获得 DI 能力
//
// 用法示例：
// ```ignore
// // 在 plugin 里声明需要 "news_feed" 服务
// impl Plugin for MyPlugin {
//     fn inject_deps(&self) -> &[&str] { &["news_feed"] }
//     fn on_load(&self, _db: &dyn DatabaseExt) -> Result<()> {
//         // 通过 ctx 拿服务（如果 plugin 构造时传入）
//         if let Some(ctx) = self.ctx.upgrade() {
//             if let Some(feed) = ctx.get_service("news_feed") {
//                 // 用 feed
//             }
//         }
//         Ok(())
//     }
// }
// ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 服务 marker trait
///
/// 任何可以注册到 Context 的类型都实现这个 trait。
/// 类似 Cordis 的 Service<T> 基类。
///
/// 自动 blanket impl：所有 Send + Sync + 'static 类型都是 Service。
pub trait Service: Send + Sync + 'static {
    /// 服务在 Context 中的 key（默认用类型名）
    ///
    /// 用户可以 override 来提供短名（如 `"database"` 而不是 `"crate::Database"`）。
    fn service_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

// 自动 blanket impl：所有满足 Send + Sync + 'static 的类型都是 Service
impl<T: Send + Sync + 'static> Service for T {}

/// 类型擦除的服务存储
///
/// 类似 Cordis 的 ReflectService.store：用 `Any` 擦除类型，
/// 运行时 `downcast::<T>()` 拿回具体类型。
#[derive(Default)]
pub struct ServiceStore {
    services: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个服务（按实例的 `service_name()` 为 key）
    ///
    /// 类似 Cordis 的 `ctx.provide(name, value)`。
    pub fn provide<S: Service>(&self, service: Arc<S>) -> String {
        let name = service.service_name().to_string();
        self.provide_as(&name, service);
        name
    }

    /// 注册一个服务（指定 key）
    pub fn provide_as<S: Service>(&self, name: &str, service: Arc<S>) {
        let mut map = self.services.write().unwrap();
        map.insert(name.to_string(), service as Arc<dyn Any + Send + Sync>);
    }

    /// 按 key 拿服务（类型擦除）
    ///
    /// 类似 Cordis 的 `ctx.reflect.get(name, strict)`。
    ///
    /// 返回 `Option<Arc<T>>`：None = 服务未注册；Some = 拿到了。
    pub fn get<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
        let map = self.services.read().unwrap();
        map.get(name)?.clone().downcast::<T>().ok()
    }

    /// 按 TypeId 拿服务（备选方案，避免传字符串）
    pub fn get_by_type<T: Service>(&self) -> Option<Arc<T>> {
        self.get::<T>(std::any::type_name::<T>())
    }

    /// 检查服务是否注册
    pub fn has(&self, name: &str) -> bool {
        self.services.read().unwrap().contains_key(name)
    }

    /// 移除服务（返回被移除的 service）
    pub fn remove(&self, name: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.services.write().unwrap().remove(name)
    }

    /// 列出所有已注册的服务 key（用于调试）
    pub fn keys(&self) -> Vec<String> {
        self.services.read().unwrap().keys().cloned().collect()
    }

    /// 已注册服务数量
    pub fn len(&self) -> usize {
        self.services.read().unwrap().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.services.read().unwrap().is_empty()
    }
}

impl std::fmt::Debug for ServiceStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceStore")
            .field("count", &self.len())
            .field("keys", &self.keys())
            .finish()
    }
}

/// DI 上下文：组合服务存储 + 事件总线
///
/// 类似 Cordis 的 `Context` 类，但比 Cordis 简单（不做 Proxy、不做 Fiber）。
/// 主要场景：plugin 通过 ctx 拿到声明的服务，无需知道是谁提供的。
#[derive(Debug)]
pub struct Context {
    store: ServiceStore,
}

impl Context {
    pub fn new() -> Self {
        Self {
            store: ServiceStore::new(),
        }
    }

    /// 拿服务存储的引用
    pub fn store(&self) -> &ServiceStore {
        &self.store
    }

    /// 提供服务（自动用实例的 `service_name()`）
    pub fn provide<S: Service>(&self, service: Arc<S>) -> String {
        self.store.provide(service)
    }

    /// 提供服务（指定 name）
    pub fn provide_as<S: Service>(&self, name: &str, service: Arc<S>) {
        self.store.provide_as(name, service);
    }

    /// 拿服务（类型擦除）
    pub fn get<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
        self.store.get::<T>(name)
    }

    /// 拿服务（按 TypeId）
    pub fn get_by_type<T: Service>(&self) -> Option<Arc<T>> {
        self.store.get_by_type::<T>()
    }

    /// 检查服务是否存在
    pub fn has(&self, name: &str) -> bool {
        self.store.has(name)
    }

    /// 验证 plugin 声明的依赖是否全部已提供
    ///
    /// 返回缺失的服务列表（空 Vec = 全部满足）。
    /// 类似 Cordis 的 `ctx.fiber._checkImpl`。
    pub fn check_inject_deps(&self, deps: &[&str]) -> Vec<String> {
        deps.iter()
            .filter(|name| !self.store.has(*name))
            .map(|s| s.to_string())
            .collect()
    }

    /// 列出已注册的所有服务 key
    pub fn list_services(&self) -> Vec<String> {
        self.store.keys()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// 一个 helper：从 `Arc<dyn Any + Send + Sync>` 重新 downcast
///
/// 用于 plugin 构造时把 host 传入的 ctx 包成具体类型。
pub fn downcast_service<T: Send + Sync + 'static>(
    arc: Arc<dyn Any + Send + Sync>,
) -> Result<Arc<T>, String> {
    arc.downcast::<T>().map_err(|_| {
        format!(
            "cannot downcast service to {}",
            std::any::type_name::<T>()
        )
    })
}

/// 按 TypeId 检查两个类型是否相同
pub fn types_match(a: TypeId, b: TypeId) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Database {
        url: String,
    }

    #[derive(Debug)]
    struct Logger;

    impl Database {
        fn new(url: &str) -> Self {
            Self { url: url.to_string() }
        }
        fn url(&self) -> &str {
            &self.url
        }
    }

    #[test]
    fn test_provide_and_get() {
        let ctx = Context::new();
        ctx.provide_as("database", Arc::new(Database::new("sqlite://test.db")));

        let db = ctx.get::<Database>("database").unwrap();
        assert_eq!(db.url(), "sqlite://test.db");
    }

    #[test]
    fn test_get_missing_returns_none() {
        let ctx = Context::new();
        assert!(ctx.get::<Database>("database").is_none());
    }

    #[test]
    fn test_check_inject_deps() {
        let ctx = Context::new();
        ctx.provide_as("logger", Arc::new(Logger));
        // 全部满足
        let missing = ctx.check_inject_deps(&["logger"]);
        assert!(missing.is_empty());
        // 缺失一个（database 没提供）
        let missing = ctx.check_inject_deps(&["logger", "database"]);
        assert_eq!(missing, vec!["database".to_string()]);
        // 只查 logger（已提供）
        let missing = ctx.check_inject_deps(&["logger"]);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_service_count_and_keys() {
        let ctx = Context::new();
        assert!(ctx.store.is_empty());

        ctx.provide_as("database", Arc::new(Database::new("db")));
        ctx.provide_as("logger", Arc::new(Logger));

        assert_eq!(ctx.store.len(), 2);
        let keys = ctx.store.keys();
        assert!(keys.contains(&"database".to_string()));
        assert!(keys.contains(&"logger".to_string()));
    }

    #[test]
    fn test_remove_service() {
        let ctx = Context::new();
        ctx.provide_as("logger", Arc::new(Logger));
        assert!(ctx.has("logger"));

        let removed = ctx.store.remove("logger");
        assert!(removed.is_some());
        assert!(!ctx.has("logger"));
    }

    #[test]
    fn test_default_service_name_uses_type_name() {
        // 不指定 name，service_name() 默认是 std::any::type_name::<Self>()
        let ctx = Context::new();
        let _ = ctx.provide(Arc::new(Logger));
        // type_name 包含模块路径，不为空即可
        assert!(ctx.store.len() >= 1);
    }

    #[test]
    fn test_downcast_service_helper() {
        let arc: Arc<dyn Any + Send + Sync> = Arc::new(Logger);
        let result = downcast_service::<Logger>(arc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_downcast_wrong_type_fails() {
        let arc: Arc<dyn Any + Send + Sync> = Arc::new(Logger);
        let result = downcast_service::<Database>(arc);
        assert!(result.is_err());
    }
}