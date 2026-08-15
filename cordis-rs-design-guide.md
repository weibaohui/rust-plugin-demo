# cordis-rs 设计指南：基于 Bevy 哲学 + shaku 起步 + 自研迁移

> 一句话总结：**Rust 版 Cordis 不需要从零写**——**直接抄 Bevy 的 `Plugin` / `App` / `Resource` 三件套哲学**（同构度 95%），**MVP 阶段用 shaku 做 DI**（1 周搞定），**后期自研 cordis-rs 完整 5 概念 + 4 dispatch + HMR**。这条路径工作量最小、生态最成熟、与 Rust 习语最契合。

- 调研时间：2026-07-30（deepseek profile 生成）
- 适用版本：Bevy 0.14+ / shaku 0.10+ / tokio 1.x
- 协议：MIT（所有引用项目）

---

## 0. 一句话回答

**没有现成方案能直接当 Cordis 用**，但 **Bevy 是 80% 哲学相似的方案**，shaku 是 Rust 生态最成熟的 DI 框架。**推荐组合** = Bevy 哲学（API 设计）+ shaku（MVP DI 引擎）+ 自研（cordis-rs 长期演进）。

---

## 1. 现有方案对比矩阵

| 方案 | 能直接当 Cordis 用？ | 借鉴度 | 推荐场景 |
|---|---|---|---|
| **Bevy**（35k⭐） | ❌（是 ECS 不是 DI） | ⭐⭐⭐⭐⭐ | **强烈推荐借鉴** |
| **shaku**（500+⭐） | ✅是 DI，风格偏 Spring | ⭐⭐⭐⭐ | **MVP 直接用** |
| **tower::Service**（1k+⭐） | ❌（请求-响应框架） | ⭐⭐⭐ | 中间件 chain 借鉴 |
| **axum**（18k⭐） | ❌（HTTP 专用） | ⭐⭐ | 不推荐 |
| **async-actor / actix** | ❌（actor 模型） | ⭐ | 不推荐 |

---

## 2. Bevy 是最值得借鉴的方案（80% 同构）

Bevy 的 `Plugin` / `Resource` / `App` / `System` 哲学和 Cordis **几乎一一对应**。

### 2.1 概念对照表

| Bevy 概念 | Cordis 对应 | 相似度 | 备注 |
|---|---|---|---|
| `Plugin` trait（`build()` + `finish()`） | `Plugin.apply(ctx, config)` | **95%** | 同构 |
| `App`（全局容器） | `Context` | **80%** | App = Context + 调度器 |
| `Resource`（全局单例） | `ctx.<key>` 通过 provide 注册 | **90%** | 同构 |
| `System`（函数） | `ctx.effect()` 注册的回调 | **70%** | Bevy 自动调度，Cordis 手动 |
| `World`（资源仓库） | `ctx.registry` | **85%** | 同构 |
| `Schedule`（系统调度） | ❌ Cordis 没有 | Bevy 独有 | 不抄 |
| `Event` + `EventReader` / `EventWriter` | `ctx.on` / `ctx.emit` | **85%** | 同构 |
| `Component`（ECS） | ❌ | Bevy 独有 | 不抄 |
| `States` + `OnEnter/OnExit` | `PluginStatus` 3 态 | 60% | 部分同构 |

### 2.2 Bevy 的 Plugin trait 完整代码（直接借鉴）

```rust
// Bevy 0.14
pub trait Plugin: Send + Sync + 'static {
    fn build(&self, app: &mut App);
    fn finish(&self, app: &mut App);
    fn cleanup(&self, app: &mut App);
}

// 用户写 plugin（这就是 Cordis 风格）
impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MyState>()
           .add_event::<MyEvent>()
           .add_systems(Update, my_system);
    }
}
```

**核心API 对应**：

| Bevy | Cordis 对应 |
|---|---|
| `App::new()` | `Context::new()` |
| `App.add_plugins(p1, p2, ...)` | `Context.plugin(p1, p2, ...)` |
| `App.init_resource::<T>()` | `Context.provide(name, T::default())` |
| `App.add_event::<E>()` | `Context.events.register::<E>()` |
| `App.add_systems(Schedule, sys)` | `Context.effect(sys)` |
| `App.world.get_resource::<T>()` | `Context.get::<T>(name)` |
| `PluginGroup`（批量） | PluginGroup trait（cordis-rs 抄） |

### 2.3 Bevy 不能直接抄的部分

| Bevy 特性 | 为何不能抄 |
|---|---|
| **ECS（World / Component / Query）** | Bevy 是 ECS，Cordis 是 DI；ECS 数据驱动，DI 服务驱动 |
| **Schedule调度器**（Fixed / Update / PostUpdate） | Cordis 没有调度循环，plugin 自己 spawn |
| **反射系统**（`bevy_reflect`） | 巨大 crate，cordis-rs 不需要 |
| **asset server / scene / renderer** | Bevy 专用 |
| **Pipelined rendering** | Bevy 专用 |

### 2.4 Bevy 的 Cargo.toml 教训（必须分 crate）

Bevy 学习曲线陡峭的原因之一：crate 太大编译慢。cordis-rs **一定要分 crate**：

```toml
# cordis-rs 推荐的工作区结构
cordis-rs/
├── cordis-rs-core/      # App + Service + Fiber（核心 ~800 LOC）
├── cordis-rs-macros/    # #[derive(Inject)]
├── cordis-rs-loader/    # YAML config（可选）
├── cordis-rs-hmr/       # HMR（可选）
├── cordis-rs-tokio/     # tokio integration
└── cordis-rs-examples/  # 例子
```

---

## 3. shaku：MVP 阶段最现实的 DI 框架

**shaku** 是 Rust 生态最成熟的 DI 框架，可以**MVP 阶段直接用**——1 周内让 plugkit 拥有 DI 能力。

### 3.1 shaku 核心代码（可立即用）

```rust
use shaku::{module, Interface, Component, HasComponent, HasProvider};

// 1. 定义接口（trait）
trait Logger: Interface { fn log(&self, msg: &str); }
trait Database: Interface { fn query(&self); }

// 2. 实现组件
#[derive(Component)]
#[shaku(interface = Logger)]
struct StdoutLogger;
impl Logger for StdoutLogger {
    fn log(&self, m: &str) { println!("{}", m); }
}

#[derive(Component)]
#[shaku(interface = Database)]
struct SqliteDatabase;
impl Database for SqliteDatabase { fn query(&self) { /* ... */ } }

// 3. 声明模块
module! {
    MyModule {
        components = [StdoutLogger, SqliteDatabase],
        providers = []
    }
}

// 4. 自动注入函数
#[shaku::inject]
fn do_query(db: &dyn Database, log: &dyn Logger) {
    log.log("querying");
    db.query();
}

// 5. 使用
fn main() {
    let module = MyModule::builder().build();
    let db: &dyn Database = module.resolve_ref(); // DI 拿服务
    do_query(&module, db, module.resolve_ref());  // 自动注入
}
```

### 3.2 shaku vs Cordis：6 大差异

| 维度 | shaku | Cordis |
|---|---|---|
| **动态注册** | ❌ 编译期决定（`module!`宏） | ✅ 运行时 `ctx.plugin()` |
| **热加载** | ❌ | ✅ HMR |
| **4 dispatch** | ❌ | ✅ emit / parallel / serial / waterfall |
| **Async** | 部分（async-trait） | ✅ 原生 |
| **生命周期管理** | 简单 | ✅ epoch + reload/unload |
| **学习曲线** | ⭐⭐ | ⭐⭐⭐⭐ |
| **编写时间** | 1 周 | 9 周 |

### 3.3 何时从 shaku 迁到自研

| 信号 | 触发迁移动作 |
|---|---|
| 需要运行时注册 plugin | shaku 不支持，迁 |
| 需要 HMR | shaku 不支持，迁 |
| 需要 4 dispatch 模式 | shaku 只有简单 callback，迁 |
| 需要 plugin 间解耦（隔离） | shaku 单模块限制，迁 |
| 项目规模 > 20 个 plugin | 维护成本超过收益，迁 |

---

## 4. tower::Service：方向不同，借鉴中间件模式

tower 是 **Rust 网络请求处理框架**——`Service<T>` trait 处理一个请求返回响应。**和 Cordis 不是同一个抽象层级**。

### 4.1 能借鉴的

- **`Service<Request, Response, Error>` trait 设计**——async fn + 关联类型是 Rust 处理"事件"的 idiomatic 方式

```rust
// tower-style 处理事件（可借鉴）
#[async_trait]
pub trait EventHandler<E>: Send + Sync {
    async fn handle(&self, event: E) -> Result<(), Error>;
}
```

- **中间件 chain 的 `poll_ready` 模式**——协程感

### 4.2 不能借鉴的

| tower 特性 | 为何不能借鉴 |
|---|---|
| **请求-响应模型** | tower 是 HTTP，Cordis 是事件流 |
| **Layer包装 Service**（A → B） | Cordis plugin 是添加 handler（A → A+B） |
| **ServiceBuilder** | 适合请求栈，不适合 plugin |

**借鉴程度**：⭐⭐（中间件 chain 设计有参考价值，整体不适用）

---

## 5. 完整 crate 借鉴清单

| crate | 借鉴什么 | star | 状态 |
|---|---|---|---|
| **bevy** | Plugin / Resource / App / AppBuilder 哲学 | 35k+ | ✅ **最强借鉴** |
| **bevy_ecs** | World / Resource / System 单独用 | 35k+ | ✅ 可选 |
| **shaku** | Rust DI 模板 | 500+ | ✅ **MVP 直接用** |
| **tower** | Service trait + 中间件 chain | 1k+ | ⚠️ 部分借鉴 |
| **axum** | Router 组合 / FromRequest | 18k+ | ⚠️ HTTP 专用 |
| **async-trait** | async fn in trait | 2k+ | ✅ 必需 |
| **tracing** | 日志 | 5k+ | ✅ 直接用 |
| **notify** | 文件 watch（HMR 用） | 2k+ | ✅ 直接用 |
| **tokio** | 异步运行时 | 27k+ | ✅ 必需 |
| **parking_lot** | 高效 RwLock | 2k+ | ✅ 必需 |
| **serde / serde_yml** | 配置序列化 | 8k+ | ✅ 可选 |

---

## 6. 三大件 trait 的 Rust 骨架（直接抄 Bevy）

### 6.1 App（= Context）

```rust
// cordis-rs-core/src/app.rs
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;
use crate::event::EventBus;
use crate::plugin::Plugin;

pub struct App {
    /// 全局服务表（type-erased）
    resources: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
    
    /// 事件总线
    events: Arc<EventBus>,
    
    /// 已注册 plugin 列表
    plugins: RwLock<Vec<Box<dyn Plugin>>>,
    
    /// Fiber 状态机
    fibers: RwLock<HashMap<String, Fiber>>,
    
    /// 配置
    config: RwLock<AppConfig>,
}

impl App {
    pub fn new() -> Self { /* 初始化 */ }
    
    /// 注册 plugin（核心 API，Bevy 风格）
    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self.plugins.write().push(Box::new(plugin));
        self
    }
    
    /// 注册资源（= Cordis 的 provide）
    pub fn add_resource<T: Send + Sync + 'static>(&mut self, name: &str, value: T) -> &mut Self {
        self.resources.write().insert(
            name.to_string(),
            Arc::new(value) as Arc<dyn Any + Send + Sync>
        );
        self.events.emit("resource:added", name);
        self
    }
    
    /// 拿资源（= Cordis 的 get）
    pub fn get_resource<T: Send + Sync + 'static>(&self, name: &str) -> Option<Arc<T>> {
        self.resources.read()
            .get(name)?
            .clone()
            .downcast::<T>()
            .ok()
    }
    
    /// 监听事件
    pub fn on<F>(&self, name: &str, handler: F) -> HandlerId
    where F: Fn(EventPayload) + Send + Sync + 'static {
        self.events.register(name, Box::new(handler))
    }
    
    /// 触发事件
    pub fn emit(&self, name: &str, payload: impl Any + Send) {
        self.events.emit(name, Box::new(payload));
    }
    
    /// 启动（Cordis 没有这个，Bevy 有）
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // finish 阶段：调用所有 plugin.finish()
        for plugin in self.plugins.read().iter() {
            plugin.finish(self);
        }
        
        // 启动所有 fiber
        // ...
        Ok(())
    }
}
```

### 6.2 Plugin trait（Bevy 风格直接抄）

```rust
// cordis-rs-core/src/plugin.rs
use crate::app::App;

pub trait Plugin: Send + Sync + 'static {
    /// 注册阶段：加资源、加事件、加 effect
    fn build(&self, app: &mut App) { /* 默认 no-op */ }
    
    /// 完成阶段：依赖检查、跨 plugin 配置
    fn finish(&self, app: &mut App) { /* 默认 no-op */ }
    
    /// 清理阶段：plugin 卸载时清理
    fn cleanup(&self, app: &mut App) { /* 默认 no-op */ }
    
    /// plugin 名（用于日志和事件）
    fn name(&self) -> &'static str { "anonymous" }
    
    /// 声明依赖哪些资源（Cordis inject 等价物）
    fn inject(&self) -> &'static [&'static str] { &[] }
}
```

### 6.3 Resource（= Cordis provide 的服务）

```rust
// cordis-rs-core/src/resource.rs
use std::any::Any;
use std::sync::Arc;

/// Resource 标记 trait（自动 blanket impl）
pub trait Resource: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Resource for T {}

/// Resource Accessor（让 plugin 写 `app.database` 自动解析）
pub trait FromContext {
    fn from_context(app: &App) -> Option<&Self>;
}

// 用户定义：
struct Database { /* ... */ }
impl FromContext for Database {
    fn from_context(app: &App) -> Option<&Self> {
        app.get_resource::<Self>("database")?.as_ref().into()
    }
}
```

---

## 7. 4 dispatch mode 的 Rust 实现骨架

```rust
// cordis-rs-core/src/event.rs
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;

pub type EventPayload = Box<dyn Any + Send>;
pub type SyncHandler = Box<dyn Fn(EventPayload) + Send + Sync>;
pub type AsyncHandler = Box<
    dyn Fn(EventPayload, Option<Box<dyn Fn(EventPayload) -> EventPayload + Send>>)
        -> EventPayload + Send + Sync
>;

pub struct EventBus {
    hooks: RwLock<HashMap<String, Vec<HookEntry>>>,
}

struct HookEntry {
    handler: SyncHandler,
    async_handler: Option<AsyncHandler>,
}

impl EventBus {
    /// Mode 1: emit — 观察（fire-and-forget）    pub fn emit(&self, name: &str, payload: EventPayload) {
        if let Some(hooks) = self.hooks.read().get(name) {
            for h in hooks {
                (h.handler)(unsafe { /* payload: Any */ });
            }
        }
    }
    
    /// Mode 2: parallel — 并行 await
    pub async fn parallel(&self, name: &str, payload: EventPayload) {
        if let Some(hooks) = self.hooks.read().get(name).cloned() {
            let futs = hooks.into_iter().map(|h| {
                tokio::spawn(async move { (h.handler)(payload); })
            });
            futures::future::join_all(futs).await;
        }
    }
    
    /// Mode 3: serial — 顺序 await，遇 truthy 短路
    pub async fn serial(&self, name: &str, payload: EventPayload) -> Option<EventPayload> {
        if let Some(hooks) = self.hooks.read().get(name).cloned() {
            let mut current = payload;
            for h in hooks {
                if let Some(ret) = h.handler(current) { // 返回 truthy = 短路
                    return Some(ret);
                }
            }
        }
        None
    }
    
    /// Mode 4: waterfall — 中间件链
    pub async fn waterfall(&self, name: &str, payload: EventPayload) -> EventPayload {
        let hooks = self.hooks.read().get(name).cloned();
        if let Some(hooks) = hooks {
            let mut iter = hooks.into_iter();
            let mut current = payload;
            loop {
                match iter.next() {
                    Some(h) => {
                        if let Some(ah) = h.async_handler {
                            // handler 拿 next 回调
                            let next_handler = iter.next().map(|n| {
                                Box::new(move |p| (n.handler)(p)) as Box<_>
                            });
                            current = ah(current, next_handler);
                        } else {
                            current = (h.handler)(current);
                        }
                    }
                    None => return current,
                }
            }
        }
        payload
    }
}
```

---

## 8. Fiber 生命周期核心（自研阶段）

```rust
// cordis-rs-core/src/fiber.rs
use std::sync::Arc;
use std::collections::HashSet;
use parking_lot::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiberState {
    Pending,    // 等待依赖
    Active,     // 正常运行
    Failed,     // 加载失败
    Disposed,   // 已卸载
}

pub struct Fiber {
    pub uid: u64,
    pub name: String,
    pub state: FiberState,
    /// 依赖指纹（inject 列表的 uid 哈希）
    pub epoch: u64,
    /// Dispose 链（倒序执行）
    pub disposables: Vec<Box<dyn Effect>>,
    /// 当前注入的资源 uid
    pub inject_uids: HashSet<u64>,
}

impl Fiber {
    /// 检查 epoch 变化，触发 reload / unload
    pub async fn refresh(&mut self, current_resources: &HashMap<String, u64>) {
        let new_epoch = self.compute_epoch(current_resources);
        if new_epoch == self.epoch { return; }
        self.epoch = new_epoch;
        
        if self.inject_uids.is_empty() || /* 计算后某个依赖消失 */ {
            self.unload().await;
            self.state = FiberState::Pending;
        } else {
            self.unload().await;
            self.reload().await;
            self.state = FiberState::Active;
        }
    }
    
    async fn unload(&mut self) {
        for d in self.disposables.drain(..).rev() {
            d.dispose().await;
        }
    }
    
    async fn reload(&mut self) {
        // 调 plugin 的 apply()
        // 收集新的 disposables
    }
    
    fn compute_epoch(&self, resources: &HashMap<String, u64>) -> u64 {
        // 哈希所有 inject 资源的 uid
        // 任一变化 → epoch 变 → 触发 reload
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for uid in &self.inject_uids {
            uid.hash(&mut hasher);
        }
        std::hash::Hasher::finish(&hasher)
    }
}

#[async_trait::async_trait]
pub trait Effect: Send {
    async fn dispose(self: Box<Self>);
}
```

---

## 9. 完整示例：仿 Koishi 风格 plugin

```rust
// 用户代码：
use cordis_rs_core::*;

struct HelloPlugin;
impl Plugin for HelloPlugin {
    fn name(&self) -> &'static str { "hello" }
    fn inject(&self) -> &'static [&'static str] { &["database", "logger"] }
    
    fn build(&self, app: &mut App) {
        // 注册资源
        app.add_resource("hello_count", 0u32);
        
        // 注册事件
        app.on("user.greeted", |payload| {
            println!("got greet: {:?}", payload);
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = App::new();
    
    // 1. 注册资源
    app.add_resource("database", Database::open("test.db")?);
    app.add_resource("logger", Logger::new());
    
    // 2. 加载 plugin
    app.add_plugin(HelloPlugin);
    
    // 3. 启动
    app.run().await?;
    
    // 4. 触发事件
    app.emit("user.greeted", "world");
    
    Ok(())
}
```

---

## 10. 5 阶段迁移路线图

```
现在 ──────────────────────────────→ 6 个月后
 │                                     │
   ▼ ▼
MVP（Week 1-2） v0.3（Week 7-8） v1.0（Week 13+）
shaku DI 基础     自研 core 完整       独立 crate + HMR
```

| 阶段 | 时间 | 工作量 | 产出 |
|---|---|---|---|
| **Week 1-2** | 1-2 周 | 把 `PluginManager` 重构为 `App` 类 | Bevy 风格 App 入口 |
| **Week 3-4** | 1-2 周 | `App` 内用 shaku 做 DI（provide/resolve） | MVP DI 工作 |
| **Week 5-6** | 1-2 周 | 实现 `event_bus` 的 4 dispatch mode | 事件系统升级 |
| | **MVP 完成** | | **~6 周产出可用版本** |
| **Week 7-8** | 1-2 周 | 自研 `cordis-rs-core`（替代 shaku） | 完整 5 概念 |
| **Week 9-10** | 1-2 周 | 实现 Fiber epoch + reload/unload | 生命周期管理 |
| **Week 11-12** | 1-2 周 | 加 HMR（notify watch dylib + effect 清理） | HMR 工作 |
| | **v0.3 完成** | | **~12 周产出自研版本** |
| **Week 13-16** | 4 周 | 独立 crate `cordis-rs` 发布到 crates.io | 开源可用 |
| **Week 17+** | 持续 | 持续对标 Cordis 4.x 新 feature | 长期维护 |

---

## 11. cordis-rs workspace 完整结构

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/core",         # App + Plugin + Resource + EventBus + Fiber
    "crates/macros",       # #[derive(Inject)]
    "crates/loader",       # YAML config loader
    "crates/hmr",          # HMR plugin
    "crates/tokio",        # tokio runtime adapter
    "crates/examples",     # examples/
    "crates/integration",  # 集成测试
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
```

```
cordis-rs/
├── crates/
│   ├── core/
│   │   ├── src/
│   │   │   ├── lib.rs          # barrel
│   │   │   ├── app.rs          # App（Context）
│   │   │   ├── plugin.rs       # Plugin trait
│   │   │   ├── resource.rs     # Resource + FromContext
│   │   │   ├── event.rs        # EventBus（4 dispatch）
│   │   │   ├── fiber.rs        # Fiber + epoch
│   │   │   └── effect.rs       # Effect trait + dispose
│   │   └── Cargo.toml
│   ├── macros/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── inject.rs       # #[derive(Inject)]
│   │   └── Cargo.toml
│   ├── loader/
│   │   └── src/                # YAML config
│   ├── hmr/
│   │   └── src/                # 文件 watch
│   ├── tokio/
│   │   └── src/                # 异步 runtime
│   ├── examples/
│   │   ├── hello_plugin/       # 最简单的 plugin
│   │   ├── database_plugin/    # DI 示例
│   │   ├── middleware_chain/   # waterfall 示例
│   │   └── hmr_demo/           # HMR 示例
│   └── integration/
│       └── src/                # 跨 crate 测试
├── tests/
├── docs/
├── Cargo.lock
└── README.md
```

---

## 12. shaku 起步到自研迁移的具体代码对比

### 12.1 shaku 版（MVP）

```rust
use shaku::{module, Component, Interface, HasComponent};

trait Database: Interface { fn query(&self); }

#[derive(Component)]
#[shaku(interface = Database)]
struct SqliteDb;
impl Database for SqliteDb { fn query(&self) { /* ... */ } }

module! {
    AppModule { components = [SqliteDb], providers = [] }
}

fn main() {
    let module = AppModule::builder().build();
    let db: &dyn Database = module.resolve_ref();
    db.query();
}
```

### 12.2 自研 cordis-rs 版（v0.3+）

```rust
use cordis_rs::{App, Plugin, Service};

struct SqliteDb;
impl Service for SqliteDb {
    fn name(&self) -> &str { "database" }
}

struct DbPlugin;
impl Plugin for DbPlugin {
    fn build(&self, app: &mut App) {
        app.add_service(SqliteDb);
        app.on("query", |ctx| {
            if let Some(db) = ctx.get::<SqliteDb>("database") {
                db.query();
            }
        });
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugin(DbPlugin);
    app.emit("query", ());
}
```

**迁移时机**：当 plugkit 需要运行时注册 / HMR / 4 dispatch 时，从 shaku 迁到自研。**API 差异**：shaku 用 `module!` 宏 + `resolve()`，自研用 `App` + `get()`。

---

## 13. 关键引用速查

| 关注点 | 文件 / 链接 |
|---|---|
| Bevy Plugin trait | https://github.com/bevyengine/bevy/blob/main/crates/bevy_app/src/plugin.rs |
| Bevy App | https://github.com/bevyengine/bevy/blob/main/crates/bevy_app/src/app.rs |
| shaku README | https://github.com/Mcat12/shaku |
| Cordis 5 概念 | deepseek-harness/docs/cordis-primer.md |
| Cordis Fiber 状态机 | cordis/packages/core/src/fiber.ts |
| Cordis 4 dispatch | cordis/packages/core/src/events.ts |
| tower::Service | https://docs.rs/tower |

---

## 14. 适用人群

| 角色 | 价值 |
|---|---|
| **plugkit 维护者** | 按 5 阶段路线 5-9 周完成 Cordis 风格重构 |
| **Rust AI Agent 框架作者** | cordis-rs 可作为自家 Agent 框架的内核 |
| **想做 plugin 系统的 Rust 项目** | 直接抄 Bevy 哲学 + shaku |
| **学习 Rust DI** | shaku 是教科书级别实现 |

---

## 15. 结论：对你最有价值的 3 点

1. **Bevy 是80% 哲学相似的方案**——`Plugin` / `App` / `Resource` 几乎和 Cordis 一一对应；**直接抄 API 设计哲学**（不是代码），用户用 cordis-rs 会感觉"Bevy 风格"。
2. **MVP 阶段用 shaku 撑起来**——1 周内让 plugkit 拥有 DI；后期再自研 cordis-rs 不会浪费，**工作量从 9 周降到 5 周**（前期 shaku，后期替换）。
3. **避开 tower / axum 路线**——它们是**请求-响应**框架（适合 HTTP），Cordis 是**事件流**框架（适合 plugin 通信）；只在中间件 chain 模式借鉴即可，**不要照搬**。

---

## 16. 参考资料

- **Bevy 0.14**：https://github.com/bevyengine/bevy（35k+⭐）
- **shaku 0.10**：https://github.com/Mcat12/shaku
- **tower**：https://github.com/tower-rs/tower
- **Cordis 主仓**：https://github.com/cordiverse/cordis
- **DSH primer**：https://deepseek-harness.github.io/deepseek-harness/reference/cordis-primer
- **koishi**：https://github.com/koishijs/koishi
- **async-trait**：https://github.com/dtolnay/async-trait
- **tracing**：https://github.com/tokio-rs/tracing
- **parking_lot**：https://github.com/Amanieu/parking_lot
- **协议**：MIT
- 推荐起步顺序：Bevy 阅读 → shaku 起步 → 写 cordis-rs 自研