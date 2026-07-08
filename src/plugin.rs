/*!
定义插件 API 所需的组件。

该模块中定义的类型是定义插件 API 所必需的。

# 示例 - 定义插件

```rust
use dygpi::plugin::Plugin;

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

    fn on_load(&self) -> dygpi::error::Result<()> {
        // 连接到音频引擎
        // 加载媒体流
        Ok(())
    }

    fn on_unload(&self) -> dygpi::error::Result<()> {
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
use dygpi::plugin::PluginRegistrar;
# use dygpi::plugin::Plugin;
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
#     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
#     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
# }
# impl SoundEffectPlugin {
#     pub fn new(id: &str) -> Self { unimplemented!() }
#     pub fn play(&self) {}
# }

const PLUGIN_ID: &str = concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::DelayEffect");

#[no_mangle]
pub extern "C" fn register_plugins<MyPlugin>(
    registrar: &mut PluginRegistrar<SoundEffectPlugin>
) {
    registrar.register(SoundEffectPlugin::new(PLUGIN_ID));
}
```

*/

use crate::error::Result;
use std::any::Any;
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "config_serde", derive(serde::Serialize))]
pub struct CronSpec {
    /// 任务名(插件内唯一)。
    pub name: String,
    /// 执行间隔(秒)。
    pub interval_secs: u64,
}

///
/// 任何插件类型都必须实现此 trait。它不仅提供插件 ID，还提供了实现者可用来管理
/// 插件所拥有资源的生命周期方法。
pub trait Plugin: Any + Debug + Sync + Send {
    ///
    /// 返回此实例的插件标识符。通常一种既唯一又具有调试/跟踪价值的格式是使用
    /// 包/模块路径，如下所示。
    ///
    /// ```rust
    /// const PLUGIN_ID: &str = concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::MyPlugin");
    /// ```
    fn plugin_id(&self) -> &String;

    /// 由插件管理器在注册过程完成后调用。默认 no-op。
    fn on_load(&self) -> Result<()> {
        Ok(())
    }

    /// 由插件管理器在插件被注销后、库关闭前调用。默认 no-op。
    fn on_unload(&self) -> Result<()> {
        Ok(())
    }

    /// 首次安装:数据初始化(幂等)。`load` 时调用。默认 no-op。
    fn on_install(&self) -> Result<()> {
        Ok(())
    }

    /// 卸载清理。`unload` 时调用。默认 no-op。
    fn on_uninstall(&self) -> Result<()> {
        Ok(())
    }

    /// 版本迁移:`load` 时若已安装版本与当前不同则调用。默认 no-op(留作扩展)。
    fn on_upgrade(&self, _from_version: &str) -> Result<()> {
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
    fn cron_specs(&self) -> Vec<CronSpec> {
        Vec::new()
    }
}

///
/// 插件提供者**必须**在其库中包含的注册函数类型。
/// 该函数构造插件实例，并使用注册器作为回调将插件注册到插件管理器。
///
/// ```rust
/// use dygpi::plugin::PluginRegistrar;
/// # use dygpi::plugin::Plugin;
///
/// # #[derive(Debug)] struct SoundEngine;
/// # #[derive(Debug)] struct MediaStream;
/// # #[derive(Debug)]
/// # struct SoundEffectPlugin {
/// #     id: String,
/// #     engine: SoundEngine,
/// #     media: MediaStream,
/// # };
/// # impl Plugin for SoundEffectPlugin {
/// #     fn plugin_id(&self) -> &String {
/// #         &self.id
/// #     }
/// #     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
/// #     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
/// # }
/// # impl SoundEffectPlugin {
/// #     pub fn new(id: &str) -> Self { unimplemented!() }
/// #     pub fn play(&self) {}
/// # }
/// # const PLUGIN_ID: &str = concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::DelayEffect");
/// #[no_mangle]
/// pub extern "C" fn register_plugins<MyPlugin>(registrar: &mut PluginRegistrar<SoundEffectPlugin>) {
///     registrar.register(SoundEffectPlugin::new(PLUGIN_ID));
/// }
/// ```
///
pub type PluginRegistrationFn<T> = fn(registrar: &mut PluginRegistrar<T>);

///
/// 注册函数的必需名称（参见 [`PluginRegistrationFn`](type.PluginRegistrationFn.html) 类型）。
///
pub const PLUGIN_REGISTRATION_FN_NAME: &[u8] = b"register_plugins\0";

///
/// 注册器由插件管理器创建，并提供给库的注册函数，用于注册其拥有的插件。
///
#[derive(Debug)]
pub struct PluginRegistrar<T>
where
    T: Plugin,
{
    plugins: Vec<Arc<T>>,
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
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn compatibility_hash() -> u64 {
    const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    const RUSTC_VERSION: &str = env!("RUSTC_VERSION");

    debug!(
        "compatibility_hash() -> Hash({:?}, {:?})",
        CARGO_PKG_VERSION, RUSTC_VERSION
    );

    let mut s = DefaultHasher::new();
    CARGO_PKG_VERSION.hash(&mut s);
    RUSTC_VERSION.hash(&mut s);
    s.finish()
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl<T> PluginRegistrar<T>
where
    T: Plugin,
{
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
    pub fn register(&mut self, plugin: T) {
        if self.error.is_none() {
            self.plugins.push(Arc::new(plugin));
        }
    }

    ///
    /// 向注册器报告错误；注意如果记录了多个错误，只有最后一个会传播到插件管理器。
    ///
    pub fn error(&mut self, error: Box<dyn std::error::Error>) {
        self.error = Some(error);
    }

    pub(crate) fn plugins(self) -> std::result::Result<Vec<Arc<T>>, Box<dyn std::error::Error>> {
        match self.error {
            None => Ok(self.plugins),
            Some(error) => Err(error),
        }
    }
}
