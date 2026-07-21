/*!
定义插件 API 所需的组件。

该模块中定义的类型是定义插件 API 所必需的。

# 示例 - 定义插件

```rust
use plugkit::plugin::Plugin;

# #[derive(Debug)] struct SoundEngine;
# #[derive(Debug)] struct MediaStream;
#[derive(Debug)]
struct SoundEffectPlugin {
    id: String,
    engine: SoundEngine,
    media: MediaStream,
};

impl Plugin for SoundEffectPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn on_load(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> {
        // 连接到音频引擎
        // 加载媒体流
        Ok(())
    }

    fn on_unload(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> {
        // 卸载媒体流
        // 断开音频引擎连接
        Ok(())
    }
}

impl SoundEffectPlugin {
    pub fn new(id: &str) -> Self { unimplemented!() }
    pub fn play(&self) {}
}
```

# 示例 - 注册插件

```rust
use plugkit::plugin::PluginRegistrar;
use std::sync::Arc;
# use plugkit::plugin::Plugin;
# #[derive(Debug)] struct SoundEngine;
# #[derive(Debug)] struct MediaStream;
# #[derive(Debug)]
# struct SoundEffectPlugin {
#     id: String,
#     engine: SoundEngine,
#     media: MediaStream,
# };
# impl Plugin for SoundEffectPlugin {
#     fn plugin_id(&self) -> &String {
#         &self.id
#     }
#     fn on_load(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> { Ok(()) }
#     fn on_unload(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> { Ok(()) }
# }
# impl SoundEffectPlugin {
#     pub fn new(id: &str) -> Self { unimplemented!() }
#     pub fn play(&self) {}
# }

const PLUGIN_ID: &str = concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::DelayEffect");

#[no_mangle]
pub extern "C" fn register_plugins(
    registrar: &mut PluginRegistrar,
) {
    registrar.register(Arc::new(SoundEffectPlugin::new(PLUGIN_ID)));
}
```

*/

use crate::auth::ctx::RequestCtx;
use crate::database::DatabaseExt;
use crate::error::Result;
use crate::metadata::PluginMetadata;
pub use include_dir::Dir;
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 插件当前的生命周期状态。
///
/// 状态流转:`load → Loaded → enable → Enabled → start → Running`;
/// `stop → Enabled`;`disable → Loaded`;`unload → 移除`。
/// 菜单可见性:仅 `Enabled` / `Running` 时插件菜单对外暴露。
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "config_serde", derive(serde::Serialize))]
pub enum PluginStatus {
    /// 已加载(`on_load` + `on_install`),菜单不可见。
    Loaded,
    /// 已启用(`on_enable`),菜单可见、API 可访问。
    Enabled,
    /// 已启动(`on_start` + cron 注册),后台任务运行。
    Running,
}

///
/// 插件声明的定时任务规格。宿主在 `on_start` 后据此调度,`on_stop` 时注销。
///
/// **注意**:此类型已迁移至 [`crate::metadata::CronSpec`],此处为向后兼容的再导出。
///
pub use crate::metadata::CronSpec;

// ------------------------------------------------------------------------------------------------
// 插件 HTTP 路由类型
// ------------------------------------------------------------------------------------------------

/// 路由 handler 函数指针签名。
///
/// 参数:
/// - `plugin` — 插件实例引用
/// - `ctx` — 请求上下文，包含认证信息、数据库访问、事件总线等
/// - `req` — 标准 HTTP 请求 (body 为 `Vec<u8>`)
///
/// 返回标准 HTTP 响应 (body 为 `Vec<u8>`)。
pub type PluginRouteHandler = fn(
    plugin: &dyn Plugin,
    ctx: &RequestCtx,
    req: http::Request<Vec<u8>>,
) -> http::Response<Vec<u8>>;

/// 插件路由认证要求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthRequirement {
    /// 公开接口，无需认证。
    Public,
    /// 需要登录，但不检查具体权限。
    Authenticated,
    /// 需要特定权限。
    Permission(&'static str),
}

/// 插件路由定义 — 一个功能一个 handler。
#[derive(Debug, Clone)]
pub struct PluginRoute {
    /// HTTP 方法。
    pub method: http::Method,
    /// 插件命名空间下的相对路径，如 "/items"、"/items/:id"。
    pub path: String,
    /// 路由 handler 函数指针。
    pub handler: PluginRouteHandler,
    /// 路由认证要求，宿主 middleware 据此拦截。
    pub auth: AuthRequirement,
}

///
/// 任何插件类型都必须实现此 trait。它不仅提供插件 ID 与元信息,还提供了实现者可用来管理
/// 插件所拥有资源的生命周期方法。
///
/// # 与宿主数据库交互
///
/// 宿主集成了 SQLite 并通过 [`DatabaseExt`] 接口传递给插件。钩子方法中的 `db` 参数
/// 用于表的创建、初始化、卸载清理。**所有数据库操作必须幂等**。
///
pub trait Plugin: Debug + Sync + Send {
    ///
    /// 返回此实例的插件标识符。通常一种既唯一又具有调试/跟踪价值的格式是使用
    /// 包/模块路径,如下所示。
    ///
    /// ```rust
    /// const PLUGIN_ID: &str = concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::MyPlugin");
    /// ```
    fn plugin_id(&self) -> &String;

    ///
    /// 返回此插件的元信息(名称、版本、作者、依赖、菜单等)。
    ///
    /// 宿主据此进行发现、显示、依赖检测、启动顺序排序、卸载清理。
    /// 详见 [`PluginMetadata`](../metadata/struct.PluginMetadata.html)。
    ///
    /// **默认实现**返回空元信息——仅填充 `name`/`title`/`version` 为占位符。
    /// 插件**应当**覆盖此方法以声明真实元信息。
    ///
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(self.plugin_id(), self.plugin_id(), "0.0.0")
    }

    /// 由插件管理器在注册过程完成后调用。默认 no-op。
    ///
    /// `db` 为宿主传递的数据库操作接口,插件可据此初始化表结构。
    fn on_load(&self, _db: &dyn DatabaseExt) -> Result<()> {
        Ok(())
    }

    /// 由插件管理器在插件被注销后、库关闭前调用。默认 no-op。
    ///
    /// `db` 为宿主传递的数据库操作接口,插件可据此清理资源。
    fn on_unload(&self, _db: &dyn DatabaseExt) -> Result<()> {
        Ok(())
    }

    /// 首次安装:数据初始化(幂等)。`load` 时调用。默认 no-op。
    ///
    /// 插件应在此创建表、插入基础数据。`db` 为宿主传递的数据库操作接口。
    fn on_install(&self, _db: &dyn DatabaseExt) -> Result<()> {
        Ok(())
    }

    /// 卸载清理。`unload` 时调用。默认 no-op。
    ///
    /// `keep_data` 为 `true` 时保留数据,仅清理注册信息;为 `false` 时删除表与数据。
    /// `db` 为宿主传递的数据库操作接口,插件可据此 drop 表。
    fn on_uninstall(&self, _db: &dyn DatabaseExt, _keep_data: bool) -> Result<()> {
        Ok(())
    }

    /// 版本迁移:`load` 时若已安装版本与当前不同则调用。默认 no-op(留作扩展)。
    ///
    /// `from_version` 为旧版本号,`db` 为宿主传递的数据库操作接口。
    fn on_upgrade(&self, _db: &dyn DatabaseExt, _from_version: &str) -> Result<()> {
        Ok(())
    }

    /// 启用:暴露能力(菜单可见、API 可访问)。`Loaded → Enabled`。默认 no-op。
    fn on_enable(&self) -> Result<()> {
        Ok(())
    }

    /// 禁用:收敛能力。`Enabled → Loaded`。默认 no-op。
    fn on_disable(&self) -> Result<()> {
        Ok(())
    }

    /// 启动后台任务。`Enabled → Running`。默认 no-op。
    fn on_start(&self) -> Result<()> {
        Ok(())
    }

    /// 停止后台任务。`Running → Enabled`。默认 no-op。
    fn on_stop(&self) -> Result<()> {
        Ok(())
    }

    /// 定时任务执行(由宿主按 `cron_specs` 调度)。默认 no-op。
    fn on_cron(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    /// 声明的定时任务规格;宿主在 `on_start` 后据此调度,`on_stop` 时注销。默认空。
    ///
    /// **注意**:元信息 [`PluginMetadata::crons`] 也声明定时任务规格,两者择一即可。
    /// 优先使用 `metadata().crons()`。
    fn cron_specs(&self) -> Vec<CronSpec> {
        Vec::new()
    }

    /// 插件 UI 的基目录（相对路径，如 `"afp_plugin/ui"`）。
    /// 宿主据此计算 `ui_entry` URL 并从内存/磁盘服务静态文件。默认 None 表示无 UI。
    fn ui_base_dir(&self) -> Option<&str> {
        None
    }

    /// 插件是否有嵌入的前端 UI。
    fn has_ui(&self) -> bool {
        self.ui_base_dir().is_some()
    }

    /// 编译期嵌入的 `ui/dist/` 目录树。插件通过 `include_dir!` 宏嵌入后在此返回。
    /// 宿主据此从内存服务插件 UI 静态文件，无需访问磁盘。
    /// 默认返回 `None`（无嵌入 UI）。
    fn ui_dist(&self) -> Option<&'static Dir<'static>> {
        None
    }

    /// 接收事件总线上的事件。默认 no-op。
    ///
    /// 其他插件通过 `ctx.emit(topic, payload)` 发布事件后，
    /// 宿主会广播给所有已启用/运行中的插件，每个插件通过此方法接收。
    fn on_event(&self, _event: &crate::event_bus::Event) -> Result<()> {
        Ok(())
    }

    /// 声明插件的 HTTP 路由列表 — 一个功能一个 handler。
    ///
    /// 路由挂载在 `/plugin-api/<plugin-id>/` 命名空间下，
    /// `path` 为相对于该命名空间的路径，如 `"/items"`、`"/items/:id"`。
    /// 每个 `PluginRoute` 包含一个独立的 handler 函数指针。
    /// 默认返回空列表。
    fn routes(&self) -> Vec<PluginRoute> {
        vec![]
    }
}

///
/// 插件提供者**必须**在其库中包含的注册函数类型。该函数构造插件实例，
/// 并使用注册器作为回调将插件注册到插件管理器。
///
/// **注意**：`PluginRegistrar` 为非泛型，宿主和 dylib 看到的 笠名完全一致，
/// 避免单态化 ABI 不匹配。插件在 `register` 时通过 `Arc::new(MyPlugin)` 上转为 `Arc<dyn Plugin>`。
///
pub type PluginRegistrationFn = fn(registrar: &mut PluginRegistrar);

///
/// 注册函数的必需名称（参见 [`PluginRegistrationFn`](type.PluginRegistrationFn.html) 类型）。
///
pub const PLUGIN_REGISTRATION_FN_NAME: &[u8] = b"register_plugins\0";

///
/// 注册器由插件管理器创建，并提供给库的注册函数，用于注册其拥有的插件。
///
/// 注册器由插件管理器创建，并提供给库的注册函数，用于注册其拥有的插件。
///
/// **注意**：本类型为**非泛型**，内部统一存储 `Arc<dyn Plugin>`，
/// 以保证宿主与 dylib 看到的内存布局完全一致（避免单态化导致的 ABI 不匹配）。
/// 插件调用 `registrar.register(Arc::new(MyPlugin))` 时，`Arc<MyPlugin>` 会自动上转为 `Arc<dyn Plugin>`。
#[derive(Debug)]
pub struct PluginRegistrar {
    plugins: Vec<Arc<dyn Plugin>>,
    error: Option<Box<dyn std::error::Error>>,
}

// ------------------------------------------------------------------------------------------------
// 公开函数
// ------------------------------------------------------------------------------------------------

pub(crate) type CompatibilityFn = fn() -> u64;

pub(crate) const COMPATIBILITY_FN_NAME: &[u8] = b"compatibility_hash\0";

///
/// 此函数被暴露出来，以便插件提供者中链接的版本可以与插件宿主中的版本进行比较。
///
/// 哈希基于 crate 版本、rustc 版本和 `PluginRouteHandler` 类型指纹计算。
/// 当 handler 签名或相关类型变更时，哈希自动变化，宿主据此拒绝加载旧插件。
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn compatibility_hash() -> u64 {
    const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const RUSTC_VERSION: &str = env!("RUSTC_VERSION");

    debug!(
        "compatibility_hash() -> Hash({:?}, {:?}, PluginRouteHandler)",
        CARGO_PKG_VERSION, RUSTC_VERSION
    );

    let mut s = DefaultHasher::new();
    CARGO_PKG_VERSION.hash(&mut s);
    RUSTC_VERSION.hash(&mut s);
    // 加入 ABI 版本号，确保 PluginRouteHandler 签名变更时哈希变化
    // v1: 初始版本 (plugin, db, req)
    // v2: 2026-07-21 改为 (plugin, ctx, req)
    const ABI_VERSION: u64 = 2;
    ABI_VERSION.hash(&mut s);
    s.finish()
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl PluginRegistrar {
    pub(crate) fn default() -> Self {
        Self {
            plugins: Default::default(),
            error: None,
        }
    }

    ///
    /// 注册一个插件，将其存储在注册器中，直到注册完成。
    /// 注册函数执行完毕后，如果没有报告错误，插件管理器将添加所有插件。
    ///
    /// `plugin` 为 `Arc<T>`（具体插件类型），会在内部自动上转为 `Arc<dyn Plugin>`。
    ///
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        if self.error.is_none() {
            self.plugins.push(plugin);
        }
    }

    ///
    /// 向注册器报告错误；注意如果记录了多个错误，只有最后一个会传播到插件管理器。
    ///
    pub fn error(&mut self, error: Box<dyn std::error::Error>) {
        self.error = Some(error);
    }

    pub(crate) fn plugins(
        self,
    ) -> std::result::Result<Vec<Arc<dyn Plugin>>, Box<dyn std::error::Error>> {
        match self.error {
            None => Ok(self.plugins),
            Some(error) => Err(error),
        }
    }
}
