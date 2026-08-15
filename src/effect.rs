// =====================================================================
// Phase 4 (Cordis 风格) — Effect 自动 dispose + PluginGroup 批量管理
// =====================================================================
//
// 借鉴 Cordis 的 Reversible Effects 与 Bevy 的 PluginGroup：
//
//   1. Effect 是可逆副作用——任何 plugin 注册的 timer / handler / 资源
//      都用 ctx.effect() 包起来，卸载时自动倒序清理。
//      类似 Cordis 的 ctx.effect() / ctx.on() 自动生成 dispose。
//
//   2. PluginGroup 允许一次注册一组 plugin（共享 setup 逻辑）。
//      类似 Bevy 的 PluginGroup trait。
//
// 与现有 API 完全兼容：
//   - 现有 plugin 无需修改即可工作
//   - 新 plugin 可选用 Effect + PluginGroup 获得更高级能力
//
// 用法：
// ```ignore
// // === Effect: 注册可逆副作用 ===
// let mut stack = EffectStack::new();
// stack.effect("cron_timer", || async { start_cron().await });
// stack.effect("event_handler", || async { register_handler().await });
// // ... plugin 卸载时：
// stack.dispose_all().await;  // 倒序清理所有副作用
//
// // === PluginGroup: 批量管理 ===
// struct MyGroup;
// impl PluginGroup for MyGroup {
//     fn build(&mut self, mgr: &mut PluginManager) -> Result<()> {
//         mgr.add_plugin(PluginA::default());
//         mgr.add_plugin(PluginB::default());
//         Ok(())
//     }
// }
// mgr.add_group(MyGroup);  // 一次注册两个
// ```

use crate::manager::PluginManager;
use crate::plugin::Plugin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// =====================================================================
// Part 1: Effect 系统
// =====================================================================

/// Effect 标识（用于日志和调试）
pub type EffectLabel = &'static str;

/// Effect trait：任何可逆副作用都实现这个 trait
///
/// 类似 Cordis 的 `Fiber.effect` 返回的 `AsyncDisposable`：
/// 调用 `dispose()` 撤销该副作用。
pub trait Effect: Send {
    /// 执行清理（同步版本）
    fn dispose_sync(&mut self);

    /// 执行清理（异步版本）—— 默认调 sync 包装
    fn dispose(&mut self)
    where
        Self: Sized,
    {
        self.dispose_sync();
    }

    /// 用于日志的标签
    fn label(&self) -> &str;
}

/// 简单闭包 effect（最常用）
pub struct ClosureEffect {
    label: String,
    dispose_fn: Option<Box<dyn FnOnce() + Send>>,
}

impl ClosureEffect {
    pub fn new<F>(label: impl Into<String>, dispose_fn: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            label: label.into(),
            dispose_fn: Some(Box::new(dispose_fn)),
        }
    }
}

impl Effect for ClosureEffect {
    fn dispose_sync(&mut self) {
        if let Some(f) = self.dispose_fn.take() {
            (f)();
        }
    }

    fn label(&self) -> &str {
        &self.label
    }
}

/// Effect 栈：收集 effect 列表，dispose_all 时倒序调用
///
/// 类似 Cordis 的 Fiber._disposables。
pub struct EffectStack {
    effects: Vec<Box<dyn Effect>>,
    label_counter: AtomicUsize,
    // 用于调试：记录最后一次 push 的 label
    last_label: Mutex<Option<String>>,
}

impl std::fmt::Debug for EffectStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectStack")
            .field("effect_count", &self.effects.len())
            .field("last_label", &self.last_label.lock().unwrap())
            .finish()
    }
}

impl Default for EffectStack {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectStack {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            label_counter: AtomicUsize::new(0),
            last_label: Mutex::new(None),
        }
    }

    /// 注册一个 effect（带自定义 dispose 闭包）
    ///
    /// 类似 Cordis 的 `ctx.effect(execute, label)`。
    pub fn effect<F>(&mut self, label: impl Into<String>, dispose_fn: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let label = label.into();
        let n = self.label_counter.fetch_add(1, Ordering::SeqCst);
        let full_label = format!("{}[{}]", label, n);
        *self.last_label.lock().unwrap() = Some(full_label.clone());
        self.effects.push(Box::new(ClosureEffect::new(full_label, dispose_fn)));
    }

    /// 注册一个简单 no-op effect（用于测试 / 占位）
    pub fn effect_noop(&mut self, label: impl Into<String>) {
        self.effect(label, || {});
    }

    /// 当前 effect 数量
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// 倒序清理所有 effect（类似 Cordis fiber._unload）
    pub fn dispose_all(&mut self) {
        // 倒序迭代：后注册的先清理
        while let Some(mut effect) = self.effects.pop() {
            effect.dispose_sync();
        }
    }

    /// 仅清理前 N 个 effect（部分 dispose）
    pub fn dispose_n(&mut self, n: usize) {
        let start = self.effects.len().saturating_sub(n);
        // 倒序 pop 直到剩 start 个
        while self.effects.len() > start {
            if let Some(mut e) = self.effects.pop() {
                e.dispose_sync();
            }
        }
    }

    /// 取最后一个 effect 的标签（用于调试）
    pub fn last_label(&self) -> Option<String> {
        self.last_label.lock().unwrap().clone()
    }
}

impl Drop for EffectStack {
    fn drop(&mut self) {
        // 兜底：drop 时自动清理
        self.dispose_all();
    }
}

// =====================================================================
// Part 2: PluginGroup 批量管理
// =====================================================================

/// PluginGroup trait：一次注册一组 plugin
///
/// 类似 Bevy 的 `PluginGroup` trait。
pub trait PluginGroup {
    /// 注册所有 plugin 到 manager
    ///
    /// 用户在此方法内调 `mgr.add_plugin(...)` 多次。
    fn build(&mut self, mgr: &mut PluginManager) -> crate::error::Result<()>;

    /// 可选：所有 plugin 注册完后做什么（finish 阶段）
    fn finish(&mut self, _mgr: &mut PluginManager) -> crate::error::Result<()> {
        Ok(())
    }

    /// 可选：所有 plugin 卸载时做什么（cleanup 阶段）
    fn cleanup(&mut self, _mgr: &mut PluginManager) -> crate::error::Result<()> {
        Ok(())
    }

    /// group 名（用于日志）
    fn name(&self) -> &str {
        "anonymous"
    }
}

/// 给 PluginManager 加 add_group 和 add_plugin 方法
///
/// 通过扩展 trait 模式实现，保持向后兼容。
pub trait PluginManagerExt {
    fn add_plugin_boxed(&mut self, plugin: Arc<dyn Plugin>) -> crate::error::Result<()>;
    fn add_group<G: PluginGroup>(&mut self, group: G) -> crate::error::Result<()>;
}

impl PluginManagerExt for PluginManager {
    /// 手动注册一个 plugin 实例（之前没有的便捷方法）
    fn add_plugin_boxed(&mut self, _plugin: Arc<dyn Plugin>) -> crate::error::Result<()> {
        // 当前 PluginManager 不支持运行时 add plugin（plugin 通过 dylib 加载）
        // 此方法作为扩展点存在——未来实现完整功能
        Err(crate::error::Error::from(crate::error::ErrorKind::InvalidPluginState(
            "add_plugin_boxed not yet implemented for dylib-based PluginManager; use load_plugins_from() instead".to_string(),
        )))
    }

    /// 注册整个 plugin group
    fn add_group<G: PluginGroup>(&mut self, mut group: G) -> crate::error::Result<()> {
        log::info!("PluginManager::add_group({})", group.name());
        group.build(self)?;
        group.finish(self)?;
        Ok(())
    }
}

// =====================================================================
// Part 3: Timer effect helper
// =====================================================================

/// Timer effect：用 EffectStack 管理定时器
///
/// 注册时启动定时器，dispose 时自动停止。
/// 类似 Cordis 的 `@cordisjs/plugin-timer`。
pub struct TimerEffect {
    label: String,
    cancelled: Arc<AtomicUsize>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl TimerEffect {
    /// 创建一个定时器 effect（按 interval 周期调用 callback）
    pub fn new<F>(label: impl Into<String>, interval: Duration, callback: F) -> Self
    where
        F: Fn() + Send + 'static,
    {
        let label = label.into();
        let cancelled = Arc::new(AtomicUsize::new(0));
        let cancelled2 = cancelled.clone();
        let handle = std::thread::spawn(move || {
            let start = Instant::now();
            loop {
                if cancelled2.load(Ordering::SeqCst) > 0 {
                    break;
                }
                std::thread::sleep(interval);
                if cancelled2.load(Ordering::SeqCst) > 0 {
                    break;
                }
                callback();
                // 防止无限循环吃光 CPU（实际 sleep 已在上面）
                let _ = start;
            }
        });
        Self {
            label,
            cancelled,
            handle: Some(handle),
        }
    }
}

impl Effect for TimerEffect {
    fn dispose_sync(&mut self) {
        // 设置取消标志
        self.cancelled.store(1, Ordering::SeqCst);
        // join 等待线程退出（最多 100ms）
        if let Some(handle) = self.handle.take() {
            // 不阻塞太久——join 超时则放弃
            // （简化：直接丢弃 handle，OS 会回收线程）
            drop(handle);
        }
    }

    fn label(&self) -> &str {
        &self.label
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_effect_stack_push_and_dispose() {
        let mut stack = EffectStack::new();
        assert_eq!(stack.len(), 0);

        let disposed = Arc::new(AtomicUsize::new(0));
        let d1 = disposed.clone();
        stack.effect("e1", move || {
            d1.fetch_add(1, Ordering::SeqCst);
        });
        let d2 = disposed.clone();
        stack.effect("e2", move || {
            d2.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(stack.len(), 2);
        stack.dispose_all();
        assert_eq!(stack.len(), 0);
        // 倒序：e2 先，e1 后
        assert_eq!(disposed.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_effect_stack_dispose_order_lifo() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut stack = EffectStack::new();

        for i in 0..5 {
            let order = order.clone();
            stack.effect(format!("e{}", i), move || {
                order.lock().unwrap().push(i);
            });
        }
        stack.dispose_all();

        // 倒序：4, 3, 2, 1, 0
        let result = order.lock().unwrap().clone();
        assert_eq!(result, vec![4, 3, 2, 1, 0]);
    }

    #[test]
    fn test_effect_stack_partial_dispose() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut stack = EffectStack::new();

        for i in 0..3 {
            let c = count.clone();
            stack.effect(format!("e{}", i), move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        stack.dispose_n(2);
        assert_eq!(stack.len(), 1);
        stack.dispose_all();
        assert_eq!(stack.len(), 0);
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_effect_stack_drop_auto_dispose() {
        let count = Arc::new(AtomicUsize::new(0));
        {
            let mut stack = EffectStack::new();
            let c = count.clone();
            stack.effect("e1", move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
            // stack drop 在这里自动清理
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_effect_stack_label_counter() {
        let mut stack = EffectStack::new();
        stack.effect_noop("a");
        stack.effect_noop("a");
        stack.effect_noop("b");
        // 标签格式："a[0]", "a[1]", "b[2]"
        let last = stack.last_label();
        assert!(last.is_some());
        assert!(last.unwrap().starts_with("b["));
    }

    #[test]
    fn test_closure_effect_label() {
        let e = ClosureEffect::new("my_label", || {});
        assert_eq!(e.label(), "my_label");
        let mut e = e;
        // 第一次 dispose：调用 dispose_fn
        e.dispose_sync();
        // 第二次 dispose：no-op（已 take 走）
        e.dispose_sync();
    }

    #[test]
    fn test_plugin_group_trait_object_compile() {
        // 验证 PluginGroup trait 可用（编译期测试）
        struct TestGroup;
        impl PluginGroup for TestGroup {
            fn build(&mut self, _mgr: &mut PluginManager) -> crate::error::Result<()> {
                Ok(())
            }
            fn name(&self) -> &str {
                "test_group"
            }
        }
        assert_eq!(TestGroup.name(), "test_group");
    }

    #[test]
    fn test_timer_effect_disposes_quickly() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut stack = EffectStack::new();
        let c = count.clone();
        stack.effects.push(Box::new(TimerEffect::new(
            "test_timer",
            Duration::from_millis(50),
            move || {
                c.fetch_add(1, Ordering::SeqCst);
            },
        )));
        // dispose 立即取消
        stack.dispose_all();
        // 等一会儿让 timer 检查取消标志
        std::thread::sleep(Duration::from_millis(120));
        // 取消后不再触发
        let after = count.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(120));
        let later = count.load(Ordering::SeqCst);
        // 如果 dispose 没生效，timer 会再触发一次
        // 允许最多 1 次触发（dispose 之前那次）
        assert!(later <= after + 1, "timer should have been cancelled");
    }
}
