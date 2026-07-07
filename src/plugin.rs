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

    ///
    /// 由插件管理器在注册过程完成后调用。
    ///
    fn on_load(&self) -> Result<()>;

    ///
    /// 由插件管理器在插件被注销后、库关闭前调用。
    ///
    fn on_unload(&self) -> Result<()>;
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
