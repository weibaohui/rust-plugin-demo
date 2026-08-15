# Cordis vs plugkit（你的 rust-plugin-demo）可行性分析

> 一句话总结：**能合，但 Cordis ≠ plugkit**——Cordis 是**进程内 DI 框架**（`ctx.plugin()`），plugkit 是**跨动态库插件系统**（`libloading` `.dylib`）。**用 Cordis 理念重构 plugkit 完全可以，但要分两层设计**——进程内 plugin 用 Cordis 思路写好挂载 / 依赖 / 生命周期；跨 dylib 边界由 plugkit 当前机制（`#[no_mangle] register_plugins` + `PluginRegistrar`）负责，Cordis 在每个 dylib 内各自跑一个 `Context`。

- 仓库：https://github.com/weibaohui/rust-plugin-demo
- 调研时间：2026-07-30（deepseek profile 生成）

---

## 0. 一句话回答

**支持，但需要分层**：
- **进程内** plugin 生命周期管理 → **用 Cordis 理念重写**（获 DI / inject / 4 dispatch / HMR）
- **跨 dylib** 加载机制 → **保留 plugkit 现有 libloading + `#[no_mangle] register_plugins`**（Cordis 本身不解决这个）
- **dylib 内的 plugin** → 每个 dylib 跑自己的 mini-Cordis Context，宿主进程通过 `PluginRegistrar` 把"外部 ctx"注入到 dylib 内的 plugin 上

---

## 1. plugkit 当前设计（你的）

| 维度 | 数据 |
|---|---|
| 仓库 LOC | 6,476 |
| 核心模块 | plugin (403) / manager (1,296) / event_bus (148) / database (379) / metadata (321) |
| 加载机制 | **`libloading`** 动态库（`#[no_mangle] extern "C" fn register_plugins`） |
| 状态机 | **3 态**：Loaded → Enabled → Running |
| 生命周期 hook | 11 个：`on_load / on_unload / on_install / on_uninstall / on_upgrade / on_enable / on_disable / on_start / on_stop / on_cron / on_event` |
| 服务暴露 | `PluginRoute` 列表 + 挂到 `/plugin-api/<plugin-id>/` |
| 数据库 | `DatabaseExt` trait + `SqliteDatabase` |
| UI | `include_dir!` 编译期嵌入 + `ui_dist() -> &'static Dir` |
| 调度 | cron 注册 / `tokio::spawn` |
| 事件 | `event_bus::Event` + `ctx.emit(topic, payload)` |
| 配置 | TOML 文件 + `PluginManagerConfiguration` |

---

## 2. Cordis 设计

| 维度 | 数据 |
|---|---|
| LOC | **1,848**（核心 9 文件） |
| 加载机制 | **进程内** `ctx.plugin(plugin, config)` |
| 状态机 | **6 态**：PENDING / LOADING / ACTIVE / FAILED / DISPOSED / UNLOADING |
| 生命周期 | `apply(ctx, config)` + `effect()` 注册的所有 dispose |
| 服务暴露 | **`ctx.<key>` 通过 Proxy 自动解析**（DI 核心） |
| 依赖 | `inject: ['database', 'logger']` 数组声明 |
| 事件 | 4 种 dispatch：`emit / parallel / serial / waterfall` |
| 配置 | YAML（`cordis.yml`） + `!!js` 表达式 |
| HMR | `@cordisjs/plugin-hmr` |

---

## 3. 核心差异对照

| 维度 | plugkit | Cordis |
|---|---|---|
| **插件边界** | **跨 .dylib**（ABI 边界） | **进程内**（trait object） |
| **加载单位** | 动态库 | 模块 |
| **类型传递** | `Arc<dyn Plugin>` | `Arc<dyn Any>` + Proxy |
| **状态机** | 3 态（业务） | 6 态（运行时） |
| **依赖声明** | 无显式 inject；plugin 自己访问 db/event_bus | 显式 `inject` 数组 |
| **服务查找** | 自己用 `manager.get(plugin_id)` | `ctx.database` / `ctx.logger` 透明访问 |
| **事件** | `ctx.emit(topic, payload)`（单模式） | 4 种 dispatch mode |
| **HMR** | ❌ | ✅（`plugin-hmr`） |
| **YAML 配置** | ❌（TOML） | ✅（`cordis.yml`） |

---

## 4. 重构方案：分两层

```
┌──────────────────────────────────────────────────────┐
│  宿主进程（你的 main.rs）                              │
│  ┌────────────────────────────────────────────────┐  │
│  │  Cordis-style Context（用 Cordis 理念重写）     │  │
│  │  - Context struct（替代全局 manager）          │  │
│  │  - Service trait + provide()                   │  │
│  │  - inject / effect / 4 dispatch                │  │
│  └────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────┐  │
│  │  plugkit loader（保留）                         │  │
│  │  - libloading Library::new("foo.dylib")        │  │
│  │  - 调 dylib 的 #[no_mangle] register_plugins()  │  │
│  │  - 把宿主 ctx 传给 dylib                       │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
                      │
                      │ 跨 ABI 边界
                      ▼
┌──────────────────────────────────────────────────────┐
│  dylib 内（如 news_api/）                              │
│  ┌────────────────────────────────────────────────┐  │
│  │  Mini-Cordis Context（dylib 内的局部 ctx）      │  │
│  │  - 收到宿主传过来的 ctx                       │  │
│  │  - 把宿主 ctx 当作 "root service" 注入         │  │
│  │  - plugin 用 ctx.database / ctx.event_bus     │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

**关键 trick**：dylib 边界传递的是**一个共享的 trait object**（不是 Context 本身）。具体实现：

```rust
// === 共享 trait（pub crate，所有 plugin 依赖） ===
pub trait HostCtx: Send + Sync {
    fn database(&self) -> &dyn DatabaseExt;
    fn event_bus(&self) -> &dyn EventBus;
    fn get_service(&self, name: &str) -> Option<Arc<dyn Any + Send + Sync>>;
    // ...
}

// === 宿主进程 ===
struct HostCtxImpl {
    inner: Context,  // Cordis 风格的内部 ctx
    db: Arc<dyn DatabaseExt>,
    events: Arc<EventBus>,
}

impl HostCtx for HostCtxImpl { ... }

// === PluginRegistrar 接收 HostCtx trait object ===
pub struct PluginRegistrar {
    host: Arc<dyn HostCtx>,
    plugins: Vec<Arc<dyn Plugin>>,
}

// === 调 dylib 时 ===
let lib = Library::new(path)?;
let registrar: &mut PluginRegistrar = ...;  // 带 host_ctx
unsafe {
    let reg: Symbol<extern "C" fn(&mut PluginRegistrar)> = lib.get(b"register_plugins")?;
    reg(registrar);
}
```

---

## 5. 具体可以借鉴 Cordis 的部分

### 5.1 **可以照搬**（高价值）

| Cordis 概念 | 在 plugkit 里改造 |
|---|---|
| **DI via Proxy** | 新增 `Service` trait + `provide(key, value)` + `ctx.<key>` 自动解析 |
| **`inject` 依赖声明** | 在 `Plugin` trait 加 `inject_deps() -> &[&str]`，manager 加载时自动排序 |
| **4 dispatch mode** | 替换 `event_bus::Event`，实现 `emit / parallel / serial / waterfall` |
| **Effect（异步 dispose）** | `on_load`/`on_unload` 改为返回 `Effect`（`BoxFuture`）而非 `Result` |
| **HMR** | `notify` crate watch dylib 文件，reload |
| **YAML 配置** | 加 `cordis.yml` 解析（可选，向后兼容 TOML） |
| **Isolate 子 ctx** | 用 `Arc<RwLock<...>>` 实现 plugin 组隔离（如 admin plugins vs user plugins） |

### 5.2 **不能照搬**（范式不同）

| Cordis 概念 | 为何不适合 |
|---|---|
| **进程内 `ctx.plugin(plugin)`** | plugkit 是跨 dylib，需要 ABI-safe 注册流程 |
| **TypeScript Proxy 元编程** | Rust 需要显式 `get::<T>("key")` 或 trait accessor |
| **`Service<T>` 子类** | Rust 用 `trait Service: Send + Sync`，无类型擦除魔法 |
| **6 态 Fiber 状态机** | 你已经有 3 态业务状态机，可保留并叠加 6 态运行时态 |

### 5.3 **plugkit 有但 Cordis 没有**（要保留）

| plugkit 特性 | 保留方式 |
|---|---|
| **跨 ABI 加载**（libloading） | Cordis 完全没这个能力，必须保留 |
| **`#[no_mangle] register_plugins`** | 同上 |
| **编译期 UI 嵌入**（`include_dir!`） | Cordis 没有，保留 |
| **TOML 配置**（保留向后兼容） | 加 YAML loader 是可选增强 |
| **3 态业务状态机** | 比 Cordis 6 态更业务友好，可保留 |
| **`PluginRoute` HTTP 路由** | Cordis 没有 axum router 集成，保留 |

---

## 6. 重构方案对比

### 方案 A：纯移植 Cordis 到 Rust（从头重写 plugkit）

| 优势 | 劣势 |
|---|---|
| 代码最干净 | 丢掉现有的 dylib 加载能力（plugkit 核心卖点） |
| 完整 4 dispatch / DI | 工作量大（800-4000 LOC Rust 重写） |
| | 失去 3000+ 行已有代码和测试 |

**评分**：❌ 不推荐

### 方案 B：在 plugkit 上嵌入 mini-Cordis（推荐）✨

**做法**：
1. 把 plugkit 内部的 `PluginManager` 改成 Cordis 风格的 `Context`
2. 新增 `service.rs`：`Service` trait + `provide()` 方法
3. 新增 `inject`：在 `Plugin` trait 加 `inject_deps()`
4. 重写 `event_bus.rs`：实现 4 种 dispatch mode
5. **保留** libloading + `PluginRegistrar` 不动
6. 新增 HMR（用 `notify` crate watch `.dylib`）

**优势**：
- ✅ 渐进重构，每个阶段可发布
- ✅ 保留所有现有能力（dylib / UI / cron / 路由）
- ✅ 获得 Cordis 的 DI / 4 dispatch / HMR
- ✅ 不破坏向后兼容（plugin API 不变）

**评分**：✅ **推荐**

### 方案 C：双层 Context（架构最干净，但工作量大）

**做法**：
1. 抽出 crate `cordis-rs`（Cordis 的 Rust 移植，~1,500 LOC）
2. plugkit 改用 `cordis-rs::Context` 作为核心
3. dylib 内用 `cordis-rs::Context` 的 mini 版本 + trait HostCtx 桥接

**优势**：
- ✅ 最干净：cordis-rs 可独立发布 + 复用
- ✅ 其他项目也能用 cordis-rs
- ✅ 与 DSH 风格的 Rust 版 Cordis 思路一致

**劣势**：
- 工作量大（先做 cordis-rs，再迁移 plugkit）
- 需要重构所有现有 plugin

**评分**：✅ 推荐（如果你计划让 cordis-rs 成为独立 crate）

---

## 7. 推荐实施方案（方案 B 具体路径）

| 阶段 | 工作量 | 内容 |
|---|---|---|
| **Step 1** | 1 周 | 抽出 `cordis-rs` crate：Context + Service trait + provide/get + Effect |
| **Step 2** | 0.5 周 | 把 `event_bus.rs` 改造为 4 dispatch mode（emit / parallel / serial / waterfall） |
| **Step 3** | 0.5 周 | 给 `Plugin` trait 加 `inject_deps()`，manager 加载时按依赖排序 |
| **Step 4** | 1 周 | 保留 libloading，新增 `HostCtx` trait 让 dylib 访问宿主 ctx |
| **Step 5** | 1 周 | 加 HMR（`notify` crate watch dylib） |
| **Step 6** | 0.5 周 | 加 YAML loader（向后兼容 TOML） |
| **合计** | **~5 周** | 全程保持向后兼容（plugin API 不变） |

**Step 1 核心代码预览**：

```rust
// cordis-rs/src/context.rs
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub trait Service: Send + Sync {
    fn name(&self) -> &str;
}

pub struct Context {
    services: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
    events: Arc<EventBus>,
    isolate: Arc<RwLock<HashMap<String, Symbol>>>,
}

impl Context {
    pub fn new() -> Self { ... }

    pub fn provide<T: Service + 'static>(&self, service: T) {
        self.services.write().insert(service.name().to_string(), Arc::new(service));
        self.events.emit("service:added", service.name());
    }

    // 这里有 ergonomic 问题：Rust 无法 ctx.<key>，需要 ctx.get::<T>("key") 或 macro
    pub fn get<T: 'static>(&self, name: &str) -> Option<Arc<T>> {
        let services = self.services.read();
        services.get(name)?.clone().downcast::<T>().ok()
    }

    pub fn emit(&self, topic: &str, payload: Box<dyn Any + Send>) { ... }
    pub fn on(&self, topic: &str, handler: Box<dyn Fn(Box<dyn Any>) + Send + Sync>) -> HandlerId { ... }
    // ... parallel / serial / waterfall ...
}
```

---

## 8. 结论：对你最有价值的 3 点

1. **完全可以重构，但要分两层**——plugkit 跨 dylib 是独特卖点（libloading），Cordis 是进程内 DI；不要互相替代，**用 Cordis 思路写 plugkit 的内部 Context**，dylib 边界保留。
2. **第一步只做 `Context` + `Service` + `provide/get` 就够用**——把当前的 `PluginManager` 内部状态用 Cordis 风格的 Context 重构；plugin 用 `ctx.get::<DatabaseService>("database")` 拿服务；事件系统升级到 4 dispatch；这是最大 ROI 的第一步。
3. **抽出 `cordis-rs` crate 是有价值的**——你可以作为"中国版 Cordis Rust 实现"独立发布；后面做 AI Agent / chat bot 都能复用；与 DSH 的 Rust 版 Cordis 思路不谋而合。

---

## 9. 参考资料

- **你的 plugkit**：https://github.com/weibaohui/rust-plugin-demo
- **Cordis（TS）**：https://github.com/cordiverse/cordis
- **DSH（vendored Cordis）**：https://github.com/deepseek-ai/deepseek-harness
- **Koishi（基于 Cordis 3.x）**：https://github.com/koishijs/koishi
- **shaku（Rust DI 参考）**：https://github.com/Mcat12/shaku
- **bevy（Plugin / Resource 借鉴）**：https://github.com/bevyengine/bevy
- **tower::Service（async trait 参考）**：https://docs.rs/tower