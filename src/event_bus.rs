/*!
简单进程内事件总线，用于插件间通信。

插件 A 通过 `ctx.emit("news-published", payload)` 发布事件，
插件 B 通过 `on_event` 钩子接收事件（宿主广播给所有已启用/运行中的插件）。

事件是去耦的：发布方和订阅方不共享任何类型，仅通过 `topic` 字符串和
`serde_json::Value` 负载约定接口。
*/

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

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

/// 进程内事件总线。
///
/// 宿主持有此总线的唯一实例。当插件调用 `ctx.emit()` 时，
/// 宿主将事件推入总线并广播给所有已启用/运行中的插件。
///
/// 当前是一个简单的广播实现——所有订阅方收到所有事件，
/// 按 `topic` 自行过滤。
#[derive(Debug)]
pub struct EventBus {
    /// 历史事件记录（可选的，用于调试 / 新订阅方回放）。
    /// 最多保留 `max_history` 条。
    history: RwLock<Vec<Event>>,
    max_history: usize,
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
            history: RwLock::new(Vec::with_capacity(100)),
            max_history: 100,
        }
    }

    /// 创建一个可配置历史容量的总线。
    pub fn with_max_history(max: usize) -> Self {
        Self {
            history: RwLock::new(Vec::with_capacity(max)),
            max_history: max,
        }
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
}
