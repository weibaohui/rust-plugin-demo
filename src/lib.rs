/*!
为 Rust 提供 _动态通用插件_（Dynamic Generic PlugIns）支持，基于动态库的插件机制。

该 crate 实现了一个简单的插件模型，允许在运行时从外部动态库加载实现。

1. 插件 _宿主_ 定义一个具体类型，即插件 _类型_。
   1. 插件 _类型_ **必须** 实现 [`Plugin`](plugin/trait.Plugin.html) trait。
   1. **建议**将插件 _类型_ 定义在一个单独的插件 _API_ crate 中，宿主和提供者都依赖它。
1. 插件 _提供者_（或 _库_）crate **必须**在 Cargo 配置中将 crate-type 设为 `"dylib"` 和 `"rlib"`。
1. 插件 _提供者_ **必须**实现一个名为 `register_plugins` 的函数，该函数接收一个
   注册器（registrar）对象，用于注册插件 _类型_ 的任何实例。
   1. 插件 _提供者_ 可以使用替代的注册函数名称，但必须通过
      [`set_registration_fn_name`](manager/struct.PluginManager.html#method.set_registration_fn_name)
      方法告知插件管理器。
1. 插件 _宿主_ 然后使用 [`PluginManager`](manager/struct.PluginManager.html) 来加载库
   并注册与插件 _类型_ 相同类型的插件。
1. 插件 _宿主_ 可以使用插件管理器的 [`get`](manager/struct.PluginManager.html#method.get)
   方法按 _id_ 获取特定插件，**或**使用
   插件管理器的 [`plugins`](manager/struct.PluginManager.html#method.plugins) 方法遍历所有插件。

重写插件注册函数允许插件 _宿主_ 通过为每种类型使用单独的注册函数，来提供不同类型的插件。

# 示例

下面的示例展示了插件管理器从特定库加载插件，然后从已加载的集合中按 ID 检索单个插件。

```rust,no_run
use plugkit::manager::PluginManager;
use plugkit::plugin::Plugin;
use std::sync::Arc;
# const EFFECT_PLUGIN_ID: &str = "sound_effects";
# #[derive(Debug)]
# struct SoundEffectPlugin;
# impl Plugin for SoundEffectPlugin {
#     fn plugin_id(&self) -> &String {
#         todo!()
#     }
#     fn on_load(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> { Ok(()) }
#     fn on_unload(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> { Ok(()) }
# }
# impl SoundEffectPlugin {
#     pub fn play(&self) {}
# }

fn main() {
    let mut plugin_manager: PluginManager = PluginManager::default();

    plugin_manager
        .load_plugins_from("libsound_one.dylib".as_ref())
        .unwrap();

    let plugin: Arc<dyn plugkit::plugin::Plugin> = plugin_manager
        .get("sound_one::sound_one::DelayEffect")
        .unwrap();

    println!("{}", plugin.plugin_id());
}
```

# 特性

`config_serde`：为 [`PluginManagerConfiguration`](config/struct.PluginManagerConfiguration.html) 类型
添加 [Serde](https://serde.rs/) 的 `Serialize` 和 `Deserialize` trait，使其可用于配置文件。

```toml
[plugins]
source = ["analog_oscillator", "lfo"]
effect = ["delay", "reverb"]
```

*/

#![warn(
    // ---------- 代码风格
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    // ---------- 公开 API
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    // ---------- 不安全代码
    unsafe_code,
    // ---------- 未使用的
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
)]

#[macro_use]
extern crate log;

// ------------------------------------------------------------------------------------------------
// 模块
// ------------------------------------------------------------------------------------------------

pub mod config;

pub mod error;

pub mod plugin;

pub mod manager;

pub mod metadata;

pub mod database;

/// 通用插件宿主模块。提供 axum HTTP 路由用于插件加载/卸载/生命周期/cron/UI 服务。
pub mod host;
