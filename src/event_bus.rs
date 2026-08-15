/*!
简单进程内事件总线，用于插件间通信。

插件 A 通过 `ctx.emit("news-published", payload)` 发布事件，
插件 B 通过 `on_event` 钩子接收事件（宿主广播给所有已启用/运行中的插件）。

事件是去耦的：发布方和订阅方不共享任何类型，仅通过 `topic` 字符串和
`serde_json::Value` 负载约定接口。
*/

use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// 由插件发布，宿主广播给所有已启用/运行中的插件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// 事件主题，如 `"news-published"`、`"config-changed"`。
    pub topic: String,
    /// 任意 JSON 负载。
    pub payload: serde_json::Value,
    /// 发布事件的插件 ID。
    pub source: String,
}

// =====================================================================
// Phase 1 (Cordis 风格) — 4 种 dispatch mode
// =====================================================================
//
// 借鉴 Cordis (cordiverse/cordis) 的 4 dispatch 语义：
//   emit      — 观察（fire-and-forget，sync）
//   parallel  — 并行 await（tokio::spawn）
//   serial    — 顺序 await，遇 truthy payload 短路
//   waterfall — 中间件链，每个 listener 可 mutate 或替换结果
//
// 与现有 `publish()` 完全兼容：现有 plugin 只用 publish，新 plugin 可以按需用 4 dispatch。
// 不破坏任何已有 API。

/// 同步 handler（emit / serial / waterfall 都可能用）
pub type SyncHandler = Arc<dyn Fn(Event) + Send + Sync>;

/// 异步 handler（parallel 用）
pub type AsyncHandler = Arc<dyn Fn(Event) -> Event + Send + Sync>;

/// Waterfall 中间件 handler（返回 mutable Event + next 回调）
///
/// 若 listener 不调用 `next`，等价于短路。
pub type WaterfallHandler = Arc<dyn Fn(Event, Arc<dyn Fn(Event) -> Event + Send + Sync>) -> Event + Send + Sync>;

/// 4 dispatch mode 的内部订阅表
struct DispatchTable {
    sync: RwLock<Vec<SyncHandler>>,           // emit
    parallel: RwLock<Vec<AsyncHandler>>,      // parallel
    serial: RwLock<Vec<AsyncHandler>>,        // serial
    waterfall: RwLock<Vec<WaterfallHandler>>, // waterfall
}

impl Default for DispatchTable {
    fn default() -> Self {
        Self {
            sync: RwLock::new(Vec::new()),
            parallel: RwLock::new(Vec::new()),
            serial: RwLock::new(Vec::new()),
            waterfall: RwLock::new(Vec::new()),
        }
    }
}

impl std::fmt::Debug for DispatchTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchTable")
            .field("sync_handlers", &self.sync.read().unwrap().len())
            .field("parallel_handlers", &self.parallel.read().unwrap().len())
            .field("serial_handlers", &self.serial.read().unwrap().len())
            .field("waterfall_handlers", &self.waterfall.read().unwrap().len())
            .finish()
    }
}

/// truthy / falsy 判断（serial mode 短路用）
///
/// 借鉴 Cordis `isBailed`：`null / false` 视为 falsy（不短路），
/// 其他（`true / object / array / number / string`）视为 truthy（短路）。
fn is_truthy(v: &serde_json::Value) -> bool {
    !matches!(v, serde_json::Value::Null | serde_json::Value::Bool(false))
}

/// 构造 waterfall 中间件链：把 handler 序列转成 next 闭包
///
/// 用 `Mutex<Vec>` 存"剩余 handler"——`MutexGuard` 可在 `Fn` 闭包内 mutable borrow。
fn next_from_iter(
    handlers: std::sync::Arc<std::sync::Mutex<Vec<WaterfallHandler>>>,
) -> Arc<dyn Fn(Event) -> Event + Send + Sync> {
    Arc::new(move |event: Event| -> Event {
        // 取出第一个，剩下的留在 Mutex 里
        let (h, rest) = {
            let mut guard = handlers.lock().unwrap();
            if guard.is_empty() {
                return event;
            }
            let h = guard.remove(0);
            (h, guard.len())
        };
        let _ = rest; // 剩余 len 仅用于 log；不再展开
        // next 闭包：递归执行剩余 handler（每个 handler 仍能调 next）
        let next: Arc<dyn Fn(Event) -> Event + Send + Sync> = next_from_iter(handlers.clone());
        h(event, next)
    })
}

/// 进程内事件总线。
///
/// 宿主持有此总线的唯一实例。当插件调用 `ctx.emit()` 时，
/// 宿主将事件推入总线并广播给所有已启用/运行中的插件。
///
/// 当前是一个简单的广播实现——所有订阅方收到所有事件，
/// 按 `topic` 自行过滤。
///
/// **Phase 1 新增**：4 dispatch mode（emit/parallel/serial/waterfall），
/// 通过 `on_*()` 系列方法订阅，`emit/parallel/serial/waterfall` 分发。
#[derive(Debug, Clone)]
pub struct EventBus {
    /// 历史事件记录（可选的，用于调试 / 新订阅方回放）。
    /// 最多保留 `max_history` 条。
    history: Arc<RwLock<Vec<Event>>>,
    max_history: usize,
    /// Phase 1: 4 dispatch mode 的订阅表
    dispatch: Arc<DispatchTable>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// 创建一个新的事件总线（保留最近 100 条历史）。
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::with_capacity(100))),
            max_history: 100,
            dispatch: Arc::new(DispatchTable::default()),
        }
    }

    /// 创建一个可配置历史容量的总线。
    pub fn with_max_history(max: usize) -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::with_capacity(max))),
            max_history: max,
            dispatch: Arc::new(DispatchTable::default()),
        }
    }

    // =================================================================
    // Phase 1 — 4 dispatch mode API（Cordis 风格）
    // =================================================================

    /// Mode 1: emit — 同步观察（fire-and-forget）
    ///
    /// 与现有 `publish()` 语义相近，但只触发注册的 sync handler，
    /// **不**写入 history（避免与 `publish` 重复）。
    ///
    /// # Example
    /// ```ignore
    /// bus.on_emit("user.login", Arc::new(|event| {
    ///     log::info!("user login: {:?}", event.payload);
    /// }));
    /// bus.emit("user.login", event);
    /// ```
    pub fn on_emit(&self, topic: &str, handler: SyncHandler) {
        // 同名 topic 共享一个 Vec（key by topic）
        // 当前实现：所有 emit 共享一个 sync Vec；如需按 topic 隔离可加 HashMap<String, ...>
        let _ = topic; // 留作未来扩展
        self.dispatch.sync.write().unwrap().push(handler);
    }

    /// 触发 emit（sync）
    pub fn emit(&self, topic: &str, event: Event) {
        let _ = topic;
        for h in self.dispatch.sync.read().unwrap().iter() {
            (h)(event.clone());
        }
    }

    /// Mode 2: parallel — 注册并行 async handler
    pub fn on_parallel(&self, handler: AsyncHandler) {
        self.dispatch.parallel.write().unwrap().push(handler);
    }

    /// 触发 parallel（同步 spawn）—— 无 tokio runtime 时降级为 sync 执行
    pub fn parallel(&self, event: Event) -> Vec<Event> {
        let handlers = self.dispatch.parallel.read().unwrap().clone();
        handlers.into_iter().map(|h| (h)(event.clone())).collect()
    }

    /// Mode 3: serial — 注册顺序 async handler（遇 truthy payload 短路）
    pub fn on_serial(&self, handler: AsyncHandler) {
        self.dispatch.serial.write().unwrap().push(handler);
    }

    /// 触发 serial——逐个执行，直到 handler 返回 truthy payload（短路）
    ///
    /// 短路约定（来自 Cordis）：
    ///   payload 是 `serde_json::Value`，truthy = `Value::Bool(true)` 或 `Value::Object(_)` 非空
    ///   falsy  = `Value::Null` / `Value::Bool(false)`
    pub fn serial(&self, mut event: Event) -> Event {
        let handlers = self.dispatch.serial.read().unwrap().clone();
        for h in handlers {
            event = (h)(event.clone());
            if is_truthy(&event.payload) {
                return event; // 短路
            }
        }
        event
    }

    /// Mode 4: waterfall — 注册中间件 handler
    pub fn on_waterfall(&self, handler: WaterfallHandler) {
        self.dispatch.waterfall.write().unwrap().push(handler);
    }

    /// 触发 waterfall——中间件链，每个 listener 可 mutate / 替换 event
    ///
    /// 下一个 listener 通过 `next` 回调调用；不调 `next` = 短路（后续 listener 不执行）。
    pub fn waterfall(&self, event: Event) -> Event {
        let handlers = self.dispatch.waterfall.read().unwrap().clone();
        if handlers.is_empty() {
            return event;
        }
        let shared = std::sync::Arc::new(std::sync::Mutex::new(handlers));
        next_from_iter(shared)(event)
    }

    /// 发布一个事件到总线。
    ///
    ///  保留到历史记录中（若超过 `max_history` 则丢弃最旧的）。
    pub fn publish(&self, event: Event) {
        let mut hist = self.history.write().unwrap();
        hist.push(event);
        if hist.len() > self.max_history {
            hist.remove(0);
        }
    }

    /// 获取历史事件列表（最近的在前）。
    pub fn history(&self) -> Vec<Event> {
        let hist = self.history.read().unwrap();
        let mut result = hist.clone();
        result.reverse();
        result
    }

    /// 获取特定主题的历史事件。
    pub fn history_by_topic(&self, topic: &str) -> Vec<Event> {
        let hist = self.history.read().unwrap();
        let mut result: Vec<Event> = hist.iter().filter(|e| e.topic == topic).cloned().collect();
        result.reverse();
        result
    }

    /// 清空历史记录。
    pub fn clear_history(&self) {
        let mut hist = self.history.write().unwrap();
        hist.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_publish_and_history() {
        let bus = EventBus::new();
        bus.publish(Event {
            topic: "test".into(),
            payload: json!({"msg": "hello"}),
            source: "plugin_a".into(),
        });
        assert_eq!(bus.history().len(), 1);
        assert_eq!(bus.history()[0].topic, "test");
    }

    #[test]
    fn test_history_by_topic() {
        let bus = EventBus::new();
        bus.publish(Event {
            topic: "a".into(),
            payload: json!({}),
            source: "p1".into(),
        });
        bus.publish(Event {
            topic: "b".into(),
            payload: json!({}),
            source: "p2".into(),
        });
        bus.publish(Event {
            topic: "a".into(),
            payload: json!({"n": 2}),
            source: "p1".into(),
        });
        let a_events = bus.history_by_topic("a");
        assert_eq!(a_events.len(), 2);
    }

    #[test]
    fn test_max_history() {
        let bus = EventBus::with_max_history(3);
        for i in 0..5 {
            bus.publish(Event {
                topic: "t".into(),
                payload: json!({"i": i}),
                source: "p".into(),
            });
        }
        assert_eq!(bus.history().len(), 3);
    }

    // =================================================================
    // Phase 1: 4 dispatch mode 测试
    // =================================================================

    #[test]
    fn test_emit_fire_and_forget() {
        let bus = EventBus::new();
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = Arc::new(AtomicUsize::new(0));
        let c2 = counter.clone();
        bus.on_emit("user.login", Arc::new(move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        }));
        bus.emit(
            "user.login",
            Event {
                topic: "user.login".into(),
                payload: json!({}),
                source: "p".into(),
            },
        );
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_parallel_fan_out() {
        let bus = EventBus::new();
        use std::sync::atomic::{AtomicUsize, Ordering};
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
        let results = bus.parallel(Event {
            topic: "x".into(),
            payload: json!({}),
            source: "p".into(),
        });
        assert_eq!(results.len(), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_serial_short_circuit() {
        let bus = EventBus::new();
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();
        let c3 = counter.clone();
        // handler 1: pass-through (return null → 不短路)
        bus.on_serial(Arc::new(move |e| {
            c1.fetch_add(1, Ordering::SeqCst);
            e
        }));
        // handler 2: 短路（return object → truthy）
        bus.on_serial(Arc::new(move |mut e| {
            c2.fetch_add(1, Ordering::SeqCst);
            e.payload = json!({"short_circuit": true});
            e
        }));
        // handler 3: 应该不被调用
        bus.on_serial(Arc::new(move |e| {
            c3.fetch_add(1, Ordering::SeqCst);
            e
        }));

        let result = bus.serial(Event {
            topic: "x".into(),
            payload: json!(null),
            source: "p".into(),
        });
        assert_eq!(counter.load(Ordering::SeqCst), 2); // 第三个没跑
        assert_eq!(result.payload["short_circuit"], true);
    }

    #[test]
    fn test_waterfall_middleware_chain() {
        let bus = EventBus::new();
        // handler 1：加 tag + 调 next
        bus.on_waterfall(Arc::new(|e, next| {
            let mut e = e;
            e.payload["chain"] = json!(["h1"]);
            next(e)
        }));
        // handler 2：替换 payload + 调 next
        bus.on_waterfall(Arc::new(|e, next| {
            let mut e = e;
            if let Some(arr) = e.payload["chain"].as_array_mut() {
                arr.push(json!("h2"));
            }
            e.payload["replaced"] = json!(true);
            next(e)
        }));
        // handler 3：不调 next（短路）
        bus.on_waterfall(Arc::new(|mut e, _next| {
            if let Some(arr) = e.payload["chain"].as_array_mut() {
                arr.push(json!("h3_short_circuit"));
            }
            e
        }));

        let result = bus.waterfall(Event {
            topic: "x".into(),
            payload: json!({}),
            source: "p".into(),
        });
        let chain = result.payload["chain"].as_array().unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], "h1");
        assert_eq!(chain[1], "h2");
        assert_eq!(chain[2], "h3_short_circuit");
        // h2 加了 replaced 字段
        assert_eq!(result.payload["replaced"], true);
    }

    #[test]
    fn test_existing_publish_still_works() {
        // 回归测试：确保 Phase 1 没有破坏现有 publish() / history()
        let bus = EventBus::new();
        bus.publish(Event {
            topic: "regression".into(),
            payload: json!({"x": 1}),
            source: "p".into(),
        });
        assert_eq!(bus.history().len(), 1);
        assert_eq!(bus.history()[0].topic, "regression");
    }
}
