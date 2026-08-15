//! Re-export of cordis-rs-core effect + plugin types.
//!
//! 老代码 `plugkit::effect::EffectStack` 仍可用。
//! 新代码推荐 `use cordis_rs_core::{Effect, EffectStack, PluginGroup, ...}`。

pub use cordis_rs_core::{
    ClosureEffect, Effect, EffectStack, PluginGroup, PluginRegistry,
};

// =====================================================================
// 集成测试 —— 验证 cordis-rs 类型在 plugkit 上下文正常工作
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

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
        assert_eq!(disposed.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_effect_stack_dispose_order_lifo() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut stack = EffectStack::new();

        for i in 0..5 {
            let order = order.clone();
            stack.effect(format!("e{}", i), move || {
                order.lock().push(i);
            });
        }
        stack.dispose_all();

        let result = order.lock().clone();
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
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_effect_stack_label_counter() {
        let mut stack = EffectStack::new();
        stack.effect_noop("a");
        stack.effect_noop("a");
        stack.effect_noop("b");
        let last = stack.last_label();
        assert!(last.is_some());
        assert!(last.unwrap().starts_with("b["));
    }

    #[test]
    fn test_closure_effect_label() {
        let e = ClosureEffect::new("my_label", || {});
        assert_eq!(e.label(), "my_label");
        let mut e = e;
        e.dispose_sync();
        // 第二次 no-op
        e.dispose_sync();
    }

    #[test]
    fn test_plugin_group_trait_compile() {
        struct TestGroup;
        impl PluginGroup for TestGroup {
            fn build(
                &mut self,
                _ctx: &mut cordis_rs_core::Context,
                _r: &mut PluginRegistry,
            ) -> Result<(), String> {
                Ok(())
            }
            fn name(&self) -> &str {
                "test_group"
            }
        }
        assert_eq!(TestGroup.name(), "test_group");
    }
}