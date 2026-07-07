/*!
插件宿主加载/卸载插件所需的组件。

插件宿主交互的主要组件是 [`PluginManager`](struct.PluginManager.html)；
该类型管理插件的生命周期，以及打开和关闭必要的动态库。

# Example

如下面的示例所示，插件管理器的接口相对简单。然而，对于更复杂的宿主，
可能需要加载多个库和不同类型的插件，因此 [`PluginManagerConfiguration`](../config/struct.PluginManagerConfiguration.html)
类型提供了更高级的抽象。

```rust,no_run
use dygpi::manager::PluginManager;
use dygpi::plugin::Plugin;
use std::sync::Arc;

# const EFFECT_PLUGIN_ID: &str = "sound_effects";
# #[derive(Debug)]
# struct SoundEffectPlugin;
# impl Plugin for SoundEffectPlugin {
#     fn plugin_id(&self) -> &String {
#         unimplemented!()
#     }
#     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
#     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
# }
# impl SoundEffectPlugin {
#     pub fn play(&self) {}
# }
let mut plugin_manager: PluginManager<SoundEffectPlugin> = PluginManager::default();

plugin_manager
    .load_plugins_from("libsound_one.dylib".as_ref())
    .unwrap();

let plugin: Arc<SoundEffectPlugin> = plugin_manager
    .get("sound_one::sound_one::DelayEffect")
    .unwrap();

println!("{}", plugin.plugin_id());

plugin.play();
```

*/

use crate::error::{Error, ErrorKind, Result};
use crate::plugin::{
    compatibility_hash, CompatibilityFn, Plugin, PluginRegistrar, PluginRegistrationFn,
    COMPATIBILITY_FN_NAME, PLUGIN_REGISTRATION_FN_NAME,
};
use libloading::{Library, Symbol};
use search_path::SearchPath;
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 插件管理器负责从动态库加载和卸载插件，根据需要动态打开和关闭库。
///
#[derive(Debug)]
pub struct PluginManager<T>
where
    T: Plugin,
{
    search_path: SearchPath,
    registration_fn_name: Vec<u8>,
    plugins: RwLock<HashMap<String, LoadedPlugin<T>>>,
}

#[cfg(target_os = "macos")]
/// 动态库常用的文件扩展名。
pub const PLATFORM_DYLIB_EXTENSION: &str = "dylib";

#[cfg(target_os = "linux")]
/// 动态库常用的文件扩展名。
pub const PLATFORM_DYLIB_EXTENSION: &str = "so";

#[cfg(target_os = "windows")]
/// 动态库常用的文件扩展名。
pub const PLATFORM_DYLIB_EXTENSION: &str = "dll";

#[cfg(target_os = "windows")]
/// 动态库的前缀（如果有）。
pub const PLATFORM_DYLIB_PREFIX: &str = "";

#[cfg(not(target_os = "windows"))]
/// 动态库的前缀（如果有）。
pub const PLATFORM_DYLIB_PREFIX: &str = "lib";

// ------------------------------------------------------------------------------------------------
// 私有类型
// ------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct LoadedPlugin<T>
where
    T: Plugin,
{
    plugin: Arc<T>,
    in_library: Arc<LoadedLibrary>,
}

#[derive(Debug)]
struct LoadedLibrary {
    file_name: PathBuf,
    library: Library,
}

// ------------------------------------------------------------------------------------------------
// 公开函数
// ------------------------------------------------------------------------------------------------

///
/// 给定一个文件名或包含文件名的路径，返回按照常见平台约定格式化后的新路径。
/// `PluginManager` 不会直接使用此函数，由客户端决定是否在将文件路径传递给管理器之前使用它。
///
/// # 示例
///
/// 以下示例在 macOS 上返回 "`libplugins.dylib`"，在 Linux 上返回 "`libplugins.so`"，
/// 在 Windows 上返回 "`plugins.dll`"。
///
/// ```rust
/// use dygpi::manager::make_platform_dylib_name;
///
/// let dylib_name = make_platform_dylib_name("plugins".as_ref());
/// ```
///
/// 如果文件名看起来有扩展名，它将被平台扩展名覆盖。
/// 因此，以下示例会用平台扩展名替换 "`foo`"。
///
/// ```rust
/// use dygpi::manager::make_platform_dylib_name;
///
/// let dylib_name = make_platform_dylib_name("plugins/aplugin.foo".as_ref());
/// ```
///
pub fn make_platform_dylib_name(file_path: &Path) -> PathBuf {
    if let Some(file_stem) = file_path.file_stem() {
        let file_name = if !PLATFORM_DYLIB_PREFIX.is_empty() {
            let mut prefixed = OsString::from(PLATFORM_DYLIB_PREFIX);
            prefixed.push(file_stem);
            prefixed
        } else {
            file_stem.to_os_string()
        };
        let mut file_path = file_path.to_path_buf();
        file_path.set_file_name(file_name);
        let _ = file_path.set_extension(PLATFORM_DYLIB_EXTENSION);
        file_path
    } else {
        file_path.to_path_buf()
    }
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

const UTF8_STRING_PANIC: &str = "将 UTF8 符号名转换为字符串时出错";

// ------------------------------------------------------------------------------------------------

impl<T> Default for PluginManager<T>
where
    T: Plugin,
{
    fn default() -> Self {
        Self {
            search_path: Default::default(),
            registration_fn_name: PLUGIN_REGISTRATION_FN_NAME.to_vec(),
            plugins: Default::default(),
        }
    }
}

impl<T> Drop for PluginManager<T>
where
    T: Plugin,
{
    fn drop(&mut self) {
        info!("PluginManager::drop()");
        self.unload_all().unwrap();
    }
}

impl<T> PluginManager<T>
where
    T: Plugin,
{
    ///
    /// 构造一个新的插件管理器，并使用字符串切片的值作为加载库时的搜索路径。
    ///
    pub fn new_with_search_path(search_path: SearchPath) -> Self {
        Self {
            search_path,
            registration_fn_name: PLUGIN_REGISTRATION_FN_NAME.to_vec(),
            plugins: Default::default(),
        }
    }

    ///
    /// 从指定环境变量中列出的库加载所有插件。
    ///
    /// 环境变量的值被视为以冒号 `':'` 分隔的路径列表。
    ///
    pub fn load_all_plugins_from_env(&mut self, env_var: &str) -> Result<()> {
        info!("PluginManager::load_all_plugins_from_env({:?})", env_var);
        if let Ok(env_value) = env::var(env_var) {
            for file_name in env_value.split(":") {
                self.load_plugins_from(&PathBuf::from(file_name))?;
            }
        } else {
            warn!("Failed to find environment variable '{}'", env_var);
        }
        Ok(())
    }

    ///
    /// 从字符串切片中指定的所有库加载插件，每个值都是一个文件路径。
    ///
    pub fn load_plugins_from_all(&mut self, file_names: &[&Path]) -> Result<()> {
        info!("PluginManager::load_all_plugins_from({:?})", file_names);
        for file_name in file_names {
            self.load_plugins_from(file_name)?;
        }
        Ok(())
    }

    ///
    /// 从具有给定文件名/路径的单个库中加载所有插件。
    ///
    #[allow(unsafe_code)]
    pub fn load_plugins_from(&mut self, file_name: &Path) -> Result<()> {
        info!("PluginManager::load_plugins_from({:?})", file_name);

        let file_name = if (file_name.is_absolute() || file_name.parent().is_some())
            && !self.search_path.is_empty()
        {
            self.find_library(file_name)
        } else {
            file_name.to_path_buf()
        };

        trace!("PluginManager::load_plugins_from() > opening library");
        let library = unsafe {
            Library::new(&file_name).map_err(|e| {
                Error::from(ErrorKind::LibraryOpenFailed(
                    file_name.to_string_lossy().to_string(),
                    Box::new(e),
                ))
            })?
        };

        let loaded_library = LoadedLibrary { file_name, library };

        trace!("PluginManager::load_plugins_from() > checking compatibility");
        self.check_compatibility(&loaded_library)?;

        trace!("PluginManager::load_plugins_from() > registering the plugins");
        self.register_plugins(loaded_library)?;

        Ok(())
    }

    ///
    /// 覆盖默认的注册函数名称 [`PLUGIN_REGISTRATION_FN_NAME`](../plugin/const.PLUGIN_REGISTRATION_FN_NAME.html)。
    ///
    /// 此函数**必须**符合 [`PluginRegistrationFn`](../plugin/function.PluginRegistrationFn.html) 类型，
    /// 并且必须像标准注册函数一样标记为 `#[no_mangle] pub extern "C"`。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use dygpi::plugin::{Plugin, PluginRegistrar};
    /// # #[derive(Debug)]
    /// # struct SoundSourcePlugin;
    /// # impl Plugin for SoundSourcePlugin {
    /// #     fn plugin_id(&self) -> &String {
    /// #         unimplemented!()
    /// #     }
    /// #     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
    /// #     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
    /// # }
    /// # impl SoundSourcePlugin {
    /// #     pub fn new(id: &str) -> Self { Self {} }
    /// # }
    /// # #[derive(Debug)]
    /// # struct SoundEffectPlugin;
    /// # impl Plugin for SoundEffectPlugin {
    /// #     fn plugin_id(&self) -> &String {
    /// #         unimplemented!()
    /// #     }
    /// #     fn on_load(&self) -> dygpi::error::Result<()> { Ok(()) }
    /// #     fn on_unload(&self) -> dygpi::error::Result<()> { Ok(()) }
    /// # }
    /// # impl SoundEffectPlugin {
    /// #     pub fn new(id: &str) -> Self { Self {} }
    /// # }
    /// # const PLUGIN_NAME: &str = "RandomSource";
    /// # const OTHER_PLUGIN_NAME: &str = "DelayEffect";
    ///
    /// #[no_mangle]
    /// pub extern "C" fn register_sources(registrar: &mut PluginRegistrar<SoundSourcePlugin>) {
    ///     registrar.register(SoundSourcePlugin::new(PLUGIN_NAME));
    /// }
    ///
    /// #[no_mangle]
    /// pub extern "C" fn register_effects(registrar: &mut PluginRegistrar<SoundEffectPlugin>) {
    ///     registrar.register(SoundEffectPlugin::new(OTHER_PLUGIN_NAME));
    /// }
    /// ```
    ///
    pub fn set_registration_fn_name(&mut self, name: &[u8]) {
        self.registration_fn_name = name.to_vec()
    }

    ///
    /// 如果插件管理器没有注册任何插件，则返回 `true`，否则返回 `false`。
    ///
    pub fn is_empty(&self) -> bool {
        self.plugins.read().unwrap().is_empty()
    }

    ///
    /// 返回此插件管理器中注册的插件数量。
    ///
    pub fn len(&self) -> usize {
        self.plugins.read().unwrap().len()
    }

    ///
    /// 如果此插件管理器具有给定插件标识符的已注册插件，则返回 `true`，否则返回 `false`。
    pub fn contains(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().unwrap();
        plugins.contains_key(plugin_id)
    }

    ///
    /// 返回具有给定插件标识符的插件（如果存在），否则返回 `None`。
    pub fn get(&self, plugin_id: &str) -> Option<Arc<T>> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(plugin_id).map(|p| p.plugin.clone())
    }

    ///
    /// 返回此插件管理器中注册的所有插件，以向量形式返回。
    ///
    pub fn plugins(&self) -> Vec<Arc<T>> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().map(|p| p.plugin.clone()).collect()
    }

    ///
    /// 卸载当前在此插件管理器中注册的所有插件及相关库。
    ///
    pub fn unload_all(&mut self) -> Result<()> {
        info!("PluginManager::unload_all()");
        let plugin_names: Vec<String> = {
            let plugins = self.plugins.write().unwrap();
            plugins.iter().map(|(n, _)| n).cloned().collect()
        };
        for name in plugin_names {
            self.unload_plugin(&name)?;
        }
        Ok(())
    }

    ///
    /// 卸载由给定插件标识符标识的插件（如果存在）。注意，
    /// 如果没有其他插件使用该插件库，此方法也会关闭该库。
    ///
    pub fn unload_plugin(&mut self, plugin_name: &str) -> Result<()> {
        info!("PluginManager::unload_plugin({:?})", plugin_name);
        let mut plugins = self.plugins.write().unwrap();
        if let Some(plugin) = plugins.remove(plugin_name) {
            trace!("PluginManager::unload_plugin() > calling plugin `on_unload`");
            plugin.plugin.on_unload()?;
            if Arc::strong_count(&plugin.in_library) == 1 {
                trace!("PluginManager::unload_plugin() > closing library");
                let in_library = Arc::try_unwrap(plugin.in_library).unwrap();
                if let Err(e) = in_library.library.close() {
                    error!(
                        "Error closing library {:?}; {}",
                        in_library.file_name.to_string_lossy().to_string(),
                        e
                    );
                    return Err(ErrorKind::LibraryCloseFailed(
                        in_library.file_name.to_string_lossy().to_string(),
                        Box::new(e),
                    )
                    .into());
                }
            }
        }
        Ok(())
    }

    // --------------------------------------------------------------------------------------------
    // 私有方法

    fn find_library(&self, file_name: &Path) -> PathBuf {
        trace!("PluginManager::find_library() > checking search path for library");
        self.search_path
            .find_file(file_name)
            .unwrap_or(file_name.to_path_buf())
    }

    #[allow(unsafe_code)]
    fn check_compatibility(&self, library: &LoadedLibrary) -> Result<()> {
        let compatibility_fn = unsafe {
            let loader_fn: Symbol<'_, CompatibilityFn> =
                library.library.get(COMPATIBILITY_FN_NAME).map_err(|e| {
                    Error::from(ErrorKind::SymbolNotFound(
                        String::from_utf8(COMPATIBILITY_FN_NAME.to_vec()).expect(UTF8_STRING_PANIC),
                        Box::new(e),
                    ))
                })?;
            loader_fn
        };
        trace!("PluginManager::check_compatibility() > fetching library compatibility hash");
        let lib_compatibility_hash: u64 = compatibility_fn();
        trace!("PluginManager::check_compatibility() > fetching local compatibility hash");
        let local_compatibility_hash: u64 = compatibility_hash();
        if lib_compatibility_hash != local_compatibility_hash {
            error!(
                "Version incompatibility {:?} != {:?}",
                lib_compatibility_hash, local_compatibility_hash
            );
            return Err(ErrorKind::IncompatibleLibraryVersion(
                library.file_name.to_string_lossy().to_string(),
            )
            .into());
        }
        trace!("PluginManager::check_compatibility() > compatibility version check passed");
        Ok(())
    }

    #[allow(unsafe_code)]
    fn register_plugins(&mut self, from_library: LoadedLibrary) -> Result<()> {
        trace!(
            "PluginManager::register_plugins(_, {:?})",
            &from_library.file_name
        );
        let load_fn = unsafe {
            let loader_fn: Symbol<'_, PluginRegistrationFn<T>> = from_library
                .library
                .get(self.registration_fn_name.as_slice())
                .map_err(|e| {
                    Error::from(ErrorKind::SymbolNotFound(
                        String::from_utf8(self.registration_fn_name.clone())
                            .expect(UTF8_STRING_PANIC),
                        Box::new(e),
                    ))
                })?;
            loader_fn
        };

        trace!(
            "PluginManager::register_plugins() > calling `{}`",
            String::from_utf8(self.registration_fn_name.clone()).expect(UTF8_STRING_PANIC)
        );
        let mut registrar = PluginRegistrar::default();
        load_fn(&mut registrar);

        let mut registry = self.plugins.write().unwrap();

        let from_library = Arc::new(from_library);

        for plugin in registrar
            .plugins()
            .map_err(|e| Error::from(ErrorKind::PluginRegistration(e)))?
        {
            info!("PluginManager::register_plugins() > calling plugin `on_load`");
            plugin.on_load()?;
            if let Some(_) = registry.insert(
                plugin.plugin_id().to_string(),
                LoadedPlugin {
                    plugin,
                    in_library: from_library.clone(),
                },
            ) {
                warn!("New plugin replaced a plugin with the same ID");
            }
        }

        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// 单元测试
// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    const EXPECTED_FILE: &str = "libmy_lib.dylib";

    #[cfg(target_os = "linux")]
    const EXPECTED_FILE: &str = "libmy_lib.so";

    #[cfg(target_os = "windows")]
    const EXPECTED_FILE: &str = "my_lib.dll";

    #[test]
    fn test_make_dylib_name() {
        let file_name = make_platform_dylib_name("my_lib".as_ref());
        assert_eq!(file_name.to_str().unwrap(), EXPECTED_FILE);
        let file_name = make_platform_dylib_name("my_lib.foo".as_ref());
        assert_eq!(file_name.to_str().unwrap(), EXPECTED_FILE);
    }
}
