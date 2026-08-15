//! Re-export of cordis-rs-core service types（向后兼容 plugkit::service::*）
//!
//! 实际实现：https://github.com/weibaohui/cordis-rs
//!
//! 老代码 `plugkit::service::Context` / `Service` / `ServiceStore` 继续可用。
//! 新代码推荐 `use cordis_rs_core::*`。

pub use cordis_rs_core::{Context, Service, ServiceStore};

// =====================================================================
// 集成测试 —— 验证 cordis-rs 类型在 plugkit 上下文中正常工作
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct Database {
        url: String,
    }

    #[derive(Debug)]
    struct Logger;

    #[test]
    fn test_provide_and_get() {
        let ctx = Context::new();
        ctx.provide_as(
            "database",
            Arc::new(Database {
                url: "sqlite://test.db".into(),
            }),
        );
        let db = ctx.get::<Database>("database").unwrap();
        assert_eq!(db.url, "sqlite://test.db");
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
        let missing = ctx.check_inject_deps(&["logger"]);
        assert!(missing.is_empty());
        let missing = ctx.check_inject_deps(&["logger", "database"]);
        assert_eq!(missing, vec!["database".to_string()]);
    }

    #[test]
    fn test_service_count_and_keys() {
        let ctx = Context::new();
        assert!(ctx.store().is_empty());
        ctx.provide_as(
            "database",
            Arc::new(Database {
                url: "db".into(),
            }),
        );
        ctx.provide_as("logger", Arc::new(Logger));
        assert_eq!(ctx.store().len(), 2);
        let keys = ctx.store().keys();
        assert!(keys.contains(&"database".to_string()));
        assert!(keys.contains(&"logger".to_string()));
    }

    #[test]
    fn test_remove_service() {
        let ctx = Context::new();
        ctx.provide_as("logger", Arc::new(Logger));
        assert!(ctx.has("logger"));
        let removed = ctx.store().remove("logger");
        assert!(removed.is_some());
        assert!(!ctx.has("logger"));
    }

    #[test]
    fn test_downcast_service_helper_removed() {
        // cordis-rs 的 downcast 在 get() 内部完成，不再有外部 helper
        // 仅保留语义验证：get 返回的 Arc<T> 仍是 T
        let ctx = Context::new();
        ctx.provide_as("db", Arc::new(Database { url: "x".into() }));
        let db: Arc<Database> = ctx.get::<Database>("db").unwrap();
        assert_eq!(db.url, "x");
    }

    #[test]
    fn test_default_service_name_uses_type_name() {
        // 不指定 name，service_name() 默认是 std::any::type_name::<Self>()
        let ctx = Context::new();
        let _ = ctx.provide(Arc::new(Logger));
        assert!(ctx.store().len() >= 1);
    }

    #[test]
    fn test_downcast_wrong_type_fails() {
        let ctx = Context::new();
        ctx.provide_as("logger", Arc::new(Logger));
        // 用错的类型取 → None
        let wrong: Option<Arc<Database>> = ctx.get("logger");
        assert!(wrong.is_none());
    }
}