/*!
提供一个配置类型，用于将插件类型标识符映射到库路径列表。
这样，无需在代码中手动加载所有插件提供者路径，即可从配置创建插件管理器实例。

# 示例

以下使用加载的配置文件为 `SoundEffectPlugin` 类型实例化一个插件管理器，
其中包含文件中指定的所有插件库。

```rust
use dygpi::config::PluginManagerConfiguration;
use dygpi::manager::PluginManager;
use dygpi::plugin::Plugin;
# const EFFECT_PLUGIN_ID: &str = "sound_effects";
# #[derive(Debug)]
# struct SoundEffectPlugin;
# impl Plugin for SoundEffectPlugin {
#     fn plugin_id(&self) -> &String {
#         todo!()
#     }
#     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
#     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
# }
# fn load_config_file() -> PluginManagerConfiguration { PluginManagerConfiguration::default() }

let config = load_config_file();

let plugin_manager: PluginManager<SoundEffectPlugin> =
    if config.contains_plugin_type(EFFECT_PLUGIN_ID) {
        config.make_manager_for_type(EFFECT_PLUGIN_ID).unwrap()
    } else {
        PluginManager::default()
    };
```

# 示例 - Serde

给定以下简单的配置，我们可以将其保存为 Serde 支持的任何格式。

```rust
use dygpi::config::PluginManagerConfiguration;

let mut config = PluginManagerConfiguration::default();
let _ = config.insert("sound_effects", &["beep".as_ref(), "boop".as_ref()]);
let _ = config.insert("light_effects", &["bright".as_ref(), "mood".as_ref()]);
```

**TOML 格式：**

```toml
[plugins]
light_effects = ["bright", "mood"]
sound_effects = ["boop", "beep"]
```

**JSON 格式：**

```json
{
    "plugins": {
        "sound_effects": ["beep","boop"],
        "light_effects": ["bright","mood"]
    }
}
```

**YAML 格式：**

```yaml
---
plugins:
  light_effects:
    - bright
    - mood
  sound_effects:
    - beep
    - boop
```

*/

use crate::error::{Error, ErrorKind, Result};
use crate::manager::PluginManager;
use crate::plugin::Plugin;
use std::collections::{HashMap, HashSet};

#[cfg(feature = "config_serde")]
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 插件管理器配置本身。逻辑上是一个从 _插件类型标识符_ 到库路径列表的映射。
/// 类型标识符允许配置对库列表进行分区，以便从同一个配置值或序列化文件中
/// 为不同的插件类型创建多个插件管理器。
///
/// 注意，如果启用了 "config_serde" 特性，此类型将实现 Serde 的 `Deserialize` 和 `Serialize` trait，
/// 因此可以包含在配置文件中。
///
/// ```rust
/// use dygpi::config::PluginManagerConfiguration;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize, Serialize)]
/// pub struct MyAppConfiguration {
///     pub save_path: String,
///     pub template_path: String,
///     pub plugins: Option<PluginManagerConfiguration>,
/// }
/// ```
///
#[cfg_attr(feature = "config_serde", derive(Deserialize, Serialize))]
#[derive(Debug)]
pub struct PluginManagerConfiguration {
    plugins: HashMap<String, HashSet<PathBuf>>,
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl Default for PluginManagerConfiguration {
    fn default() -> Self {
        Self {
            plugins: Default::default(),
        }
    }
}

impl PluginManagerConfiguration {
    /// 如果配置中不包含任何插件类型，则返回 `true`，否则返回 `false`。
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// 返回配置中插件类型的数量，也称为其『长度』。
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// 返回配置中插件类型标识符的迭代器。
    pub fn plugin_types(&self) -> impl Iterator<Item = &String> {
        self.plugins.keys()
    }

    /// 如果配置中有所提供的插件类型标识符的值，则返回 `true`，否则返回 `false`。
    pub fn contains_plugin_type(&self, plugin_type: &str) -> bool {
        self.plugins.contains_key(plugin_type)
    }

    /// 返回所提供插件类型标识符的所有库路径的迭代器。
    /// 如果配置中没有该插件类型的条目，此方法返回 `None`。
    pub fn plugin_libraries_for_type(
        &self,
        plugin_type: &str,
    ) -> Option<impl Iterator<Item = &PathBuf>> {
        self.plugins.get(plugin_type).map(|vs| vs.iter())
    }

    /// 为指定的插件类型插入一个库列表；如果该类型已有条目，则会被替换。
    /// 注意，如果库列表为空，此方法会 panic。
    pub fn insert(
        &mut self,
        plugin_type: &str,
        library_list: &[&Path],
    ) -> Option<HashSet<PathBuf>> {
        assert!(!library_list.is_empty());
        self.plugins.insert(
            plugin_type.to_string(),
            library_list.iter().map(|p| p.to_path_buf()).collect(),
        )
    }

    /// 将一批库合并到配置中指定的插件类型下。如果该类型已有条目，提供的值将添加到列表中；
    /// 如果没有，则行为与 `insert` 完全相同。注意，如果库列表为空，此方法会 panic。
    pub fn merge(&mut self, plugin_type: &str, library_list: &[&Path]) {
        assert!(!library_list.is_empty());
        if let Some(libraries) = self.plugins.get_mut(plugin_type) {
            libraries.extend(library_list.iter().map(|p| p.to_path_buf()))
        } else {
            let _ = self.insert(plugin_type, library_list);
        }
    }

    /// 移除并返回指定插件类型的插件库。
    pub fn remove(&mut self, plugin_type: &str) -> Option<HashSet<PathBuf>> {
        self.plugins.remove(plugin_type)
    }

    /// 构造并返回一个新的 [`PluginManager`](../manager/struct.PluginManager.html)，
    /// 用于插件类型 `T`，使用为指定插件类型标识符配置的库列表。
    /// 注意，如果所提供插件类型没有配置的库列表，此方法将返回错误。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use dygpi::config::PluginManagerConfiguration;
    /// use dygpi::manager::PluginManager;
    /// # use dygpi::plugin::Plugin;
    /// # #[derive(Debug)] struct SoundEngine;
    /// # #[derive(Debug)] struct MediaStream;
    /// # #[derive(Debug)]
    /// # struct SoundEffectPlugin {
    /// #     id: String,
    /// # };
    /// # impl Plugin for SoundEffectPlugin {
    /// #     fn plugin_id(&self) -> &String {
    /// #         &self.id
    /// #     }
    /// #     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
    /// #     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
    /// # }
    /// # impl SoundEffectPlugin {
    /// #     pub fn new(id: &str) -> Self { todo!() }
    /// #     pub fn play(&self) {}
    /// # }
    /// # fn read_config_file() -> String { "[plugins]\nsound_effects = [\"libsound_one.dylib\"]".to_string() }
    ///
    /// let config_as_string = read_config_file();
    /// let config: PluginManagerConfiguration = toml::from_str(&config_as_string).unwrap();
    /// let manager: PluginManager<SoundEffectPlugin> =
    ///     config.make_manager_for_type("sound_effects")
    ///         .unwrap();
    /// ```
    pub fn make_manager_for_type<T>(&self, plugin_type: &str) -> Result<PluginManager<T>>
    where
        T: Plugin,
    {
        if let Some(library_list) = self.plugins.get(plugin_type) {
            let mut manager: PluginManager<T> = PluginManager::default();
            manager.load_plugins_from_all(
                &library_list
                    .iter()
                    .map(|p| p.as_path())
                    .collect::<Vec<&Path>>(),
            )?;
            Ok(manager)
        } else {
            Err(Error::from(ErrorKind::UnknownPluginManagerType(
                plugin_type.to_string(),
            )))
        }
    }
}

// ------------------------------------------------------------------------------------------------
// 单元测试
// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_toml() {
        let mut config = PluginManagerConfiguration::default();
        let _ = config.insert("sound", &["beep".as_ref(), "boop".as_ref()]);
        let _ = config.insert("light", &["bright".as_ref(), "mood".as_ref()]);

        println!("{}", toml::to_string(&config).unwrap());
    }

    #[test]
    fn test_serialize_json() {
        let mut config = PluginManagerConfiguration::default();
        let _ = config.insert("sound", &["beep".as_ref(), "boop".as_ref()]);
        let _ = config.insert("light", &["bright".as_ref(), "mood".as_ref()]);

        println!("{}", serde_json::to_string(&config).unwrap());
    }

    #[test]
    fn test_serialize_yaml() {
        let mut config = PluginManagerConfiguration::default();
        let _ = config.insert("sound", &["beep".as_ref(), "boop".as_ref()]);
        let _ = config.insert("light", &["bright".as_ref(), "mood".as_ref()]);

        println!("{}", serde_yaml::to_string(&config).unwrap());
    }
}
