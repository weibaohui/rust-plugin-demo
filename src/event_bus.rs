//! Re-export of cordis-rs-event types.
//!
//! 老代码 `plugkit::event_bus::EventBus` 仍可用。
//! 新代码推荐 `use cordis_rs_event::*`。
//!
//! 实现：https://github.com/weibaohui/cordis-rs

pub use cordis_rs_event::{
    Event, EventBus, AsyncHandler, SyncHandler, WaterfallHandler,
};

// =====================================================================
// 集成测试 —— 验证 cordis-rs 类型在 plugkit 上下文中正常工作
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_emit_fire_and_forget() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        bus.on_emit(Arc::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        }));
        bus.emit(Event::new("user.login", json!({})));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_parallel_fan_out() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();
        let c3 = counter.clone();
        bus.on_parallel(Arc::new(move |e| {
            c1.fetch_add(1, Ordering::SeqCst);
            e
        }));
        bus.on_parallel(Arc::new(move |e| {
            c2.fetch_add(1, Ordering::SeqCst);
            e
        }));
        bus.on_parallel(Arc::new(move |e| {
            c3.fetch_add(1, Ordering::SeqCst);
            e
        }));
        let results = bus.parallel(Event::new("x", json!({})));
        assert_eq!(results.len(), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_serial_short_circuit() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();
        let c3 = counter.clone();
        bus.on_serial(Arc::new(move |e| {
            c1.fetch_add(1, Ordering::SeqCst);
            e
        }));
        bus.on_serial(Arc::new(move |mut e| {
            c2.fetch_add(1, Ordering::SeqCst);
            e.payload = json!({"short_circuit": true});
            e
        }));
        bus.on_serial(Arc::new(move |e| {
            c3.fetch_add(1, Ordering::SeqCst);
            e
        }));
        let result = bus.serial(Event::new("x", json!(null)));
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(result.payload["short_circuit"], true);
    }

    #[test]
    fn test_waterfall_middleware_chain() {
        let bus = EventBus::new();
        bus.on_waterfall(Arc::new(|e, next| {
            let mut e = e;
            e.payload["chain"] = json!(["h1"]);
            next(e)
        }));
        bus.on_waterfall(Arc::new(|e, next| {
            let mut e = e;
            if let Some(arr) = e.payload["chain"].as_array_mut() {
                arr.push(json!("h2"));
            }
            e.payload["replaced"] = json!(true);
            next(e)
        }));
        bus.on_waterfall(Arc::new(|mut e, _next| {
            if let Some(arr) = e.payload["chain"].as_array_mut() {
                arr.push(json!("h3_short_circuit"));
            }
            e
        }));
        let result = bus.waterfall(Event::new("x", json!({})));
        let chain = result.payload["chain"].as_array().unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], "h1");
        assert_eq!(chain[1], "h2");
        assert_eq!(chain[2], "h3_short_circuit");
        assert_eq!(result.payload["replaced"], true);
    }

    #[test]
    fn test_existing_publish_still_works() {
        // 回归测试：EventBus::publish / history 仍可用
        let bus = EventBus::new();
        bus.publish(Event::new("regression", json!({"x": 1})));
        assert_eq!(bus.history().len(), 1);
        assert_eq!(bus.history()[0].topic, "regression");
    }

    #[test]
    fn test_eventbus_clone_shares_dispatch() {
        // Clone 后两个 bus 共享 dispatch table
        let bus1 = EventBus::new();
        let bus2 = bus1.clone();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        bus1.on_emit(Arc::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        }));
        // 通过 bus2 触发，handler 是在 bus1 注册的
        bus2.emit(Event::new("x", json!({})));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}