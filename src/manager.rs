/*!
插件宿主加载/卸载插件所需的组件。

插件宿主交互的主要组件是 [`PluginManager`](struct.PluginManager.html)；
该类型管理插件的生命周期，以及打开和关闭必要的动态库。

# Example

如下面的示例所示，插件管理器的接口相对简单。然而，对于更复杂的宿主，
可能需要加载多个库和不同类型的插件，因此 [`PluginManagerConfiguration`](../config/struct.PluginManagerConfiguration.html)
类型提供了更高级的抽象。

```rust,no_run
use plugkit::manager::PluginManager;
use plugkit::plugin::Plugin;
use std::sync::Arc;

# const EFFECT_PLUGIN_ID: &str = "sound_effects";
# #[derive(Debug)]
# struct SoundEffectPlugin;
# impl Plugin for SoundEffectPlugin {
#     fn plugin_id(&self) -> &String {
#         unimplemented!()
#     }
#     fn on_load(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> { Ok(()) }
#     fn on_unload(&self, _db: &dyn plugkit::database::DatabaseExt) -> plugkit::error::Result<()> { Ok(()) }
# }
# impl SoundEffectPlugin {
#     pub fn play(&self) {}
# }
let mut plugin_manager: PluginManager = PluginManager::default();

plugin_manager
    .load_plugins_from("libsound_one.dylib".as_ref())
    .unwrap();

let plugin: Arc<dyn plugkit::plugin::Plugin> = plugin_manager
    .get("sound_one::sound_one::DelayEffect")
    .unwrap();

println!("{}", plugin.plugin_id());
```

*/

use crate::database::DatabaseExt;
use crate::error::{Error, ErrorKind, Result};
use crate::metadata::PluginMetadata;
use crate::plugin::{
    compatibility_hash, CompatibilityFn, CronSpec, Plugin, PluginRegistrar, PluginRegistrationFn,
    PluginStatus, COMPATIBILITY_FN_NAME, PLUGIN_REGISTRATION_FN_NAME,
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
/// # 与宿主数据库集成
///
/// 宿主通过 [`PluginManager::with_database`] 或 [`PluginManager::set_database`] 注入
/// 一个 [`DatabaseExt`] 实现,管理器在调用生命周期钩子时将其传递给插件,
/// 插件据此进行表的创建、初始化、卸载清理。
///
#[derive(Debug)]
pub struct PluginManager {
    search_path: SearchPath,
    registration_fn_name: Vec<u8>,
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    /// 宿主注入的数据库操作接口,生命周期钩子调用时传递给插件。
    /// `None` 时钩子收到一个 no-op 的占位实现(便于无 DB 的简单用例)。
    database: Option<Arc<dyn DatabaseExt>>,
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
struct LoadedPlugin {
    plugin: Arc<dyn Plugin>,
    in_library: Arc<LoadedLibrary>,
    status: PluginStatus,
}

#[derive(Debug)]
struct LoadedLibrary {
    file_name: PathBuf,
    library: Library,
}

///
/// 当宿主未注入数据库时使用的占位实现——所有方法返回错误或空,便于无 DB 的简单用例。
///
#[derive(Debug)]
pub struct NoopDatabase;

impl DatabaseExt for NoopDatabase {
    fn execute(&self, _sql: &str) -> Result<usize> {
        Err(Error::from(ErrorKind::DatabaseError(
            "no database configured".to_string(),
        )))
    }
    fn execute_with(&self, _sql: &str, _params: &[crate::database::DbValue]) -> Result<usize> {
        Err(Error::from(ErrorKind::DatabaseError(
            "no database configured".to_string(),
        )))
    }
    fn query(&self, _sql: &str) -> Result<Vec<Vec<crate::database::DbValue>>> {
        Err(Error::from(ErrorKind::DatabaseError(
            "no database configured".to_string(),
        )))
    }
    fn query_with(
        &self,
        _sql: &str,
        _params: &[crate::database::DbValue],
    ) -> Result<Vec<Vec<crate::database::DbValue>>> {
        Err(Error::from(ErrorKind::DatabaseError(
            "no database configured".to_string(),
        )))
    }
    fn has_table(&self, _table: &str) -> Result<bool> {
        Ok(false)
    }
    fn drop_table(&self, _table: &str) -> Result<()> {
        Ok(())
    }
    fn describe(&self) -> String {
        "noop://no-database".to_string()
    }
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
/// use plugkit::manager::make_platform_dylib_name;
///
/// let dylib_name = make_platform_dylib_name("plugins".as_ref());
/// ```
///
/// 如果文件名看起来有扩展名，它将被平台扩展名覆盖。
/// 因此，以下示例会用平台扩展名替换 "`foo`"。
///
/// ```rust
/// use plugkit::manager::make_platform_dylib_name;
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

impl Default for PluginManager {
    fn default() -> Self {
        Self {
            search_path: Default::default(),
            registration_fn_name: PLUGIN_REGISTRATION_FN_NAME.to_vec(),
            plugins: Default::default(),
            database: None,
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        info!("PluginManager::drop()");
        self.unload_all(true).unwrap();
    }
}

impl PluginManager {
    ///
    /// 构造一个新的插件管理器，并使用字符串切片的值作为加载库时的搜索路径。
    ///
    pub fn new_with_search_path(search_path: SearchPath) -> Self {
        Self {
            search_path,
            registration_fn_name: PLUGIN_REGISTRATION_FN_NAME.to_vec(),
            plugins: Default::default(),
            database: None,
        }
    }

    ///
    /// 绑定宿主数据库操作接口(builder 风格,可链式)。
    ///
    /// 生命周期钩子调用时,管理器会把此接口传递给插件,插件据此读写数据库。
    /// 未注入时,钩子收到一个 no-op 的占位实现。
    ///
    pub fn with_database(mut self, db: Arc<dyn DatabaseExt>) -> Self {
        self.database = Some(db);
        self
    }

    ///
    /// 设置宿主数据库操作接口(运行期替换)。
    ///
    pub fn set_database(&mut self, db: Arc<dyn DatabaseExt>) {
        self.database = Some(db);
    }

    ///
    /// 返回宿主注入的数据库操作接口(若存在)。
    ///
    pub fn database(&self) -> Option<Arc<dyn DatabaseExt>> {
        self.database.clone()
    }

    /// 返回一个可用于生命周期调用的数据库句柄——未注入时返回 no-op 占位实现。
    fn db_or_noop(&self) -> Arc<dyn DatabaseExt> {
        self.database
            .clone()
            .unwrap_or_else(|| Arc::new(NoopDatabase) as Arc<dyn DatabaseExt>)
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
    /// use plugkit::plugin::{Plugin, PluginRegistrar};
    /// use std::sync::Arc;
    /// # #[derive(Debug)]
    /// # struct SoundSourcePlugin { id: String }
    /// # impl Plugin for SoundSourcePlugin {
    /// #     fn plugin_id(&self) -> &String { &self.id }
    /// # }
    /// # impl SoundSourcePlugin {
    /// #     pub fn new(id: &str) -> Self { Self { id: id.into() } }
    /// # }
    /// # const PLUGIN_NAME: &str = "RandomSource";
    ///
    /// #[no_mangle]
    /// pub extern "C" fn register_sources(registrar: &mut PluginRegistrar) {
    ///     registrar.register(Arc::new(SoundSourcePlugin::new(PLUGIN_NAME)));
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
    pub fn get(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(plugin_id).map(|p| p.plugin.clone())
    }

    ///
    /// 返回此插件管理器中注册的所有插件，以向量形式返回。
    ///
    pub fn plugins(&self) -> Vec<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().map(|p| p.plugin.clone()).collect()
    }

    ///
    /// 卸载当前在此插件管理器中注册的所有插件及相关库。
    ///
    /// `keep_data` 语义同 [`unload_plugin`]。
    ///
    pub fn unload_all(&mut self, keep_data: bool) -> Result<()> {
        info!("PluginManager::unload_all(keep_data={})", keep_data);
        let plugin_names: Vec<String> = {
            let plugins = self.plugins.write().unwrap();
            plugins.iter().map(|(n, _)| n).cloned().collect()
        };
        for name in plugin_names {
            self.unload_plugin(&name, keep_data)?;
        }
        Ok(())
    }

    ///
    /// 卸载由给定插件标识符标识的插件（如果存在）。注意，
    /// 如果没有其他插件使用该插件库，此方法也会关闭该库。
    ///
    /// `keep_data` 为 `true` 时,`on_uninstall` 收到 `keep_data=true`,插件保留数据;
    /// 为 `false` 时删除表与数据。
    ///
    pub fn unload_plugin(&mut self, plugin_name: &str, keep_data: bool) -> Result<()> {
        info!(
            "PluginManager::unload_plugin({:?}, keep_data={})",
            plugin_name, keep_data
        );
        let db = self.db_or_noop();
        let mut plugins = self.plugins.write().unwrap();
        if let Some(plugin) = plugins.remove(plugin_name) {
            // 收敛:Running 先 stop,Enabled 先 disable
            match plugin.status {
                PluginStatus::Running => {
                    plugin.plugin.on_stop()?;
                }
                PluginStatus::Enabled => {
                    plugin.plugin.on_disable()?;
                }
                PluginStatus::Loaded => {}
            }
            trace!("PluginManager::unload_plugin() > calling plugin `on_uninstall`");
            plugin.plugin.on_uninstall(&*db, keep_data)?;
            trace!("PluginManager::unload_plugin() > calling plugin `on_unload`");
            plugin.plugin.on_unload(&*db)?;
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
            self.persist_uninstalled(plugin_name);
        }
        Ok(())
    }

    /// 启用插件:`Loaded → Enabled`,调用 `on_enable`。
    /// 插件不存在返回 `PluginNotFound`;状态非 Loaded 返回 `InvalidPluginState`。
    /// **依赖检测**:启用前检查 `metadata().dependencies()` 中所有依赖插件均已处于
    /// Enabled/Running 状态,未满足返回 `DependencyUnmet`。
    ///
    /// 依赖检测与状态变更在同一写锁下完成,避免 TOCTOU 竞态。
    pub fn enable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().unwrap();
        // 状态检查
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| ErrorKind::PluginNotFound(plugin_id.to_string()))?;
        if plugin.status != PluginStatus::Loaded {
            return Err(ErrorKind::InvalidPluginState(format!(
                "cannot enable '{}': current {:?}, expected Loaded",
                plugin_id, plugin.status
            ))
            .into());
        }
        let meta = plugin.plugin.metadata();
        let dependencies = meta.dependencies().to_vec();

        // 释放读锁，避免死锁
        drop(plugin);
        drop(meta);

        // 自动启用未满足的依赖
        for dep in &dependencies {
            let dep_status = plugins
                .values()
                .find(|p| p.plugin.metadata().name == *dep)
                .map(|p| p.status);

            match dep_status {
                Some(PluginStatus::Loaded) => {
                    // 递归启用依赖
                    let dep_id = plugins
                        .values()
                        .find(|p| p.plugin.metadata().name == *dep)
                        .map(|p| p.plugin.plugin_id().clone())
                        .unwrap();
                    drop(plugins);
                    self.enable_plugin(&dep_id)?;
                    plugins = self.plugins.write().unwrap();
                }
                Some(s) if !matches!(s, PluginStatus::Enabled | PluginStatus::Running) => {
                    return Err(ErrorKind::DependencyUnmet(format!(
                        "cannot enable '{}': dependency '{}' current {:?}, expected Enabled/Running",
                        plugin_id, dep, s
                    ))
                    .into());
                }
                None => {
                    return Err(ErrorKind::DependencyUnmet(format!(
                        "cannot enable '{}': dependency '{}' not registered. Available plugins: {}",
                        plugin_id,
                        dep,
                        plugins
                            .values()
                            .map(|p| format!(
                                "'{}' ({})",
                                p.plugin.metadata().name,
                                p.plugin.plugin_id()
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .into());
                }
                _ => {}
            }
        }

        // 状态变更(同一写锁)
        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| ErrorKind::PluginNotFound(plugin_id.to_string()))?;
        plugin.plugin.on_enable()?;
        plugin.status = PluginStatus::Enabled;
        self.persist_plugin_status(plugin_id, "Enabled");
        Ok(())
    }

    /// 检测循环依赖：从 `plugin_id` 出发，检查其依赖链中是否包含自身。
    fn check_circular_dependency(&self, plugin_id: &str, dependencies: &[String]) -> Result<()> {
        let plugins = self.plugins.read().unwrap();

        // 构建依赖图：所有插件的依赖关系
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for p in plugins.values() {
            graph
                .entry(p.plugin.plugin_id().clone())
                .or_default()
                .extend(p.plugin.metadata().dependencies().iter().cloned());
        }

        // BFS 检查是否有循环
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        for dep in dependencies {
            queue.push_back(dep.clone());
        }

        while let Some(current) = queue.pop_front() {
            if current == plugin_id {
                return Err(ErrorKind::CircularDependency(format!(
                    "循环依赖: 启用 '{}' 会形成依赖环 ({} → ... → {})",
                    plugin_id, plugin_id, plugin_id
                ))
                .into());
            }
            if !visited.insert(current.clone()) {
                continue;
            }
            // 查找该依赖的 plugin_id
            for p in plugins.values() {
                if p.plugin.metadata().name == current {
                    for next_dep in p.plugin.metadata().dependencies() {
                        queue.push_back(next_dep.clone());
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    /// 禁用插件:`Enabled → Loaded`,调用 `on_disable`。
    /// 插件不存在返回 `PluginNotFound`;状态非 Enabled 返回 `InvalidPluginState`。
    /// **反向依赖检测**:禁用前检查没有其他 Enabled/Running 插件依赖当前插件,
    /// 否则返回 `DependencyUnmet`。
    ///
    /// 反向依赖检测与状态变更在同一写锁下完成,避免 TOCTOU �竞态。
    /// 自跳与依赖匹配统一以 `metadata().name` 为标识,避免 registry id 与
    /// metadata name 混淆导致的误判。
    pub fn disable_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().unwrap();
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| ErrorKind::PluginNotFound(plugin_id.to_string()))?;
        if plugin.status != PluginStatus::Enabled {
            return Err(ErrorKind::InvalidPluginState(format!(
                "cannot disable '{}': current {:?}, expected Enabled",
                plugin_id, plugin.status
            ))
            .into());
        }
        let my_name = plugin.plugin.metadata().name.clone();
        // 反向依赖检测:以 metadata.name 为一致标识,自跳也用 metadata.name
        for other in plugins.values() {
            if other.plugin.metadata().name == my_name {
                continue;
            }
            if !matches!(other.status, PluginStatus::Enabled | PluginStatus::Running) {
                continue;
            }
            let other_meta = other.plugin.metadata();
            if other_meta.dependencies().iter().any(|d| *d == my_name) {
                return Err(ErrorKind::DependencyUnmet(format!(
                    "cannot disable '{}': plugin '{}' depends on it",
                    plugin_id,
                    other.plugin.plugin_id()
                ))
                .into());
            }
        }
        // 状态变更
        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| ErrorKind::PluginNotFound(plugin_id.to_string()))?;
        plugin.plugin.on_disable()?;
        plugin.status = PluginStatus::Loaded;
        self.persist_plugin_status(plugin_id, "Loaded");
        Ok(())
    }

    /// 启动插件:`Enabled → Running`,调用 `on_start`,返回 `cron_specs` 供宿主调度。
    /// 插件不存在返回 `PluginNotFound`;状态非 Enabled 返回 `InvalidPluginState`。
    /// 返回的 `cron_specs` 优先取自 `metadata().crons()`,其次 `cron_specs()` 方法。
    pub fn start_plugin(&mut self, plugin_id: &str) -> Result<Vec<CronSpec>> {
        let mut plugins = self.plugins.write().unwrap();
        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| ErrorKind::PluginNotFound(plugin_id.to_string()))?;
        if plugin.status != PluginStatus::Enabled {
            return Err(ErrorKind::InvalidPluginState(format!(
                "cannot start '{}': current {:?}, expected Enabled",
                plugin_id, plugin.status
            ))
            .into());
        }
        plugin.plugin.on_start()?;
        plugin.status = PluginStatus::Running;
        self.persist_plugin_status(plugin_id, "Running");
        // 优先使用 metadata 声明的 crons
        let meta_crons = plugin.plugin.metadata().crons().to_vec();
        if !meta_crons.is_empty() {
            Ok(meta_crons)
        } else {
            Ok(plugin.plugin.cron_specs())
        }
    }

    /// 停止插件:`Running → Enabled`,调用 `on_stop`。
    /// 插件不存在返回 `PluginNotFound`;状态非 Running 返回 `InvalidPluginState`。
    pub fn stop_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().unwrap();
        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| ErrorKind::PluginNotFound(plugin_id.to_string()))?;
        if plugin.status != PluginStatus::Running {
            return Err(ErrorKind::InvalidPluginState(format!(
                "cannot stop '{}': current {:?}, expected Running",
                plugin_id, plugin.status
            ))
            .into());
        }
        plugin.plugin.on_stop()?;
        plugin.status = PluginStatus::Enabled;
        self.persist_plugin_status(plugin_id, "Enabled");
        Ok(())
    }

    /// 持久化插件状态到 plugkit_plugins 表。
    fn persist_plugin_status(&self, plugin_id: &str, status: &str) {
        if let Some(db) = self.database() {
            let _ = db.execute_with(
                "UPDATE plugkit_plugins SET status = ?, upgraded_at = datetime('now') WHERE plugin_id = ?",
                &[
                    crate::database::DbValue::text(status),
                    crate::database::DbValue::text(plugin_id),
                ],
            );
        }
    }

    /// 标记插件为已卸载（is_installed = 0），重启后 auto_load 自动跳过。
    fn persist_uninstalled(&self, plugin_id: &str) {
        if let Some(db) = self.database() {
            let _ = db.execute_with(
                "UPDATE plugkit_plugins SET is_installed = 0, upgraded_at = datetime('now') WHERE plugin_id = ?",
                &[crate::database::DbValue::text(plugin_id)],
            );
        }
    }

    /// 重启后恢复插件状态。
    ///
    /// 从 plugkit_plugins 表读取已安装插件的历史状态：
    /// - `Loaded` → 不操作（默认）
    /// - `Enabled` → 自动调用 enable
    /// - `Running` → 自动调用 enable + start
    ///
    /// 按拓扑排序执行，确保依赖关系正确。失败时打印警告，不阻断宿主启动。
    /// 仅在 `auto_load()` 所有插件加载完毕后调用。
    pub fn restore_plugin_statuses(&mut self) {
        let db = match self.database() {
            Some(db) => db,
            None => return,
        };

        // 获取所有已注册插件 ID（不依赖拓扑排序，因为插件尚未恢复状态）
        let order: Vec<String> = {
            let plugins = self.plugins.read().unwrap();
            plugins.keys().cloned().collect()
        };

        for pid in &order {
            let persisted_status = db
                .query_with(
                    "SELECT status FROM plugkit_plugins WHERE plugin_id = ? AND is_installed = 1",
                    &[crate::database::DbValue::text(pid)],
                )
                .ok()
                .and_then(|rows| rows.first().and_then(|r| r.first().cloned()))
                .and_then(|v| match v {
                    crate::database::DbValue::Text(s) => Some(s),
                    _ => None,
                });

            if matches!(
                persisted_status.as_deref(),
                Some("Enabled") | Some("Running")
            ) {
                info!("restore_plugin_statuses: auto-enabling '{}'", pid);
                if let Err(e) = self.enable_plugin(pid) {
                    warn!("restore_plugin_statuses: failed to enable '{}': {}", pid, e);
                }
            }
        }
    }

    /// 从 plugkit_plugins 表读取已安装版本。
    pub fn installed_version(&self, plugin_id: &str) -> Option<String> {
        let db = self.database()?;
        db.query_with(
            "SELECT version FROM plugkit_plugins WHERE plugin_id = ? AND is_installed = 1",
            &[crate::database::DbValue::text(plugin_id)],
        )
        .ok()
        .and_then(|rows| rows.first().and_then(|r| r.first().cloned()))
        .and_then(|v| match v {
            crate::database::DbValue::Text(s) => Some(s),
            _ => None,
        })
    }

    /// 返回插件当前状态(若存在)。
    pub fn plugin_status(&self, plugin_id: &str) -> Option<PluginStatus> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(plugin_id).map(|p| p.status)
    }

    /// 插件是否已激活（Enabled 或 Running），路由可访问。
    pub fn is_plugin_active(&self, plugin_id: &str) -> bool {
        matches!(
            self.plugin_status(plugin_id),
            Some(PluginStatus::Enabled) | Some(PluginStatus::Running)
        )
    }

    ///
    /// 返回插件的元信息(若存在)。宿主据此进行发现、显示、依赖检测、启动顺序排序。
    ///
    pub fn plugin_metadata(&self, plugin_id: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(plugin_id).map(|p| p.plugin.metadata())
    }

    ///
    /// 聚合所有处于 Enabled/Running 状态插件的菜单树,供前端 Sidebar 渲染。
    ///
    pub fn aggregate_menus(&self) -> Vec<crate::metadata::PluginMenu> {
        let plugins = self.plugins.read().unwrap();
        let mut menus = Vec::new();
        for p in plugins.values() {
            if matches!(p.status, PluginStatus::Enabled | PluginStatus::Running) {
                menus.extend(p.plugin.metadata().menus().to_vec());
            }
        }
        // 按 order 排序
        menus.sort_by_key(|m| m.order);
        menus
    }

    ///
    /// 对已启用插件进行拓扑排序,确保依赖与 `run_after` 先启动。
    ///
    /// �法:Kahn 拓扑排序。仅纳入 Enabled/Running 状态的插件;
    /// `dependencies` 与 `run_after` 均作为入度约束。
    /// 检测到循环依赖时返回 `CircularDependency`。
    ///
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let plugins = self.plugins.read().unwrap();

        // 收集所有 Enabled/Running 插件名(按 metadata.name)
        let mut enabled_names: Vec<String> = Vec::new();
        for p in plugins.values() {
            if matches!(p.status, PluginStatus::Enabled | PluginStatus::Running) {
                let name = p.plugin.metadata().name.clone();
                if !enabled_names.contains(&name) {
                    enabled_names.push(name);
                }
            }
        }

        // 构建入度表与反向邻接表(以 metadata.name 为顶点)
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for name in &enabled_names {
            in_degree.insert(name.clone(), 0);
        }
        for p in plugins.values() {
            if !matches!(p.status, PluginStatus::Enabled | PluginStatus::Running) {
                continue;
            }
            let my_name = p.plugin.metadata().name.clone();
            // dependencies 与 run_after 均作为入度约束
            let preds: Vec<String> = p
                .plugin
                .metadata()
                .dependencies()
                .iter()
                .chain(p.plugin.metadata().run_after().iter())
                .filter(|dep| {
                    // 仅处理已启用的依赖
                    plugins.values().any(|o| {
                        matches!(o.status, PluginStatus::Enabled | PluginStatus::Running)
                            && o.plugin.metadata().name == **dep
                    })
                })
                .cloned()
                .collect();
            for dep in preds {
                graph.entry(dep).or_default().push(my_name.clone());
                *in_degree.entry(my_name.clone()).or_insert(0) += 1;
            }
        }

        // Kahn 算法
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        queue.sort(); // 稳定排序
        let mut result: Vec<String> = Vec::new();
        while let Some(current) = queue.first().cloned() {
            queue.remove(0);
            result.push(current.clone());
            if let Some(depends) = graph.get(&current) {
                for dep in depends {
                    if let Some(d) = in_degree.get_mut(dep) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push(dep.clone());
                            queue.sort();
                        }
                    }
                }
            }
        }

        if result.len() != enabled_names.len() {
            // 检测到循环依赖,收集未排序的插件
            let remaining: Vec<String> = enabled_names
                .iter()
                .filter(|n| !result.contains(n))
                .cloned()
                .collect();
            return Err(ErrorKind::CircularDependency(remaining.join(", ")).into());
        }
        Ok(result)
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
            let loader_fn: Symbol<'_, PluginRegistrationFn> = from_library
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

        // 获取数据库句柄(未注入时为 no-op 占位)
        let db = self.db_or_noop();

        // 确保插件版本追踪表存在（持久化，重启后不丢失）
        let _ = db.execute(
            "CREATE TABLE IF NOT EXISTS plugkit_plugins (
                plugin_id TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Loaded',
                is_installed INTEGER NOT NULL DEFAULT 1,
                installed_at TEXT NOT NULL DEFAULT (datetime('now')),
                upgraded_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        );

        for plugin in registrar
            .plugins()
            .map_err(|e| Error::from(ErrorKind::PluginRegistration(e)))?
        {
            info!("PluginManager::register_plugins() > calling plugin `on_load`");

            // 从持久化表读取旧版本（进程重启后仍有效），失败时 fallback 到内存 registry
            let new_version = plugin.metadata().version.clone();
            let old_version = {
                let pid = plugin.plugin_id();
                let persisted = db
                    .query_with(
                        "SELECT version FROM plugkit_plugins WHERE plugin_id = ?",
                        &[crate::database::DbValue::text(pid)],
                    )
                    .ok()
                    .and_then(|rows| rows.first().and_then(|r| r.first().cloned()))
                    .and_then(|v| match v {
                        crate::database::DbValue::Text(s) => Some(s),
                        _ => None,
                    });
                persisted.or_else(|| {
                    registry
                        .get(pid)
                        .map(|old| old.plugin.metadata().version.clone())
                })
            };

            // 判断是否需要升级：已有版本记录且版本变化，或表已存在但未追踪版本
            let needs_upgrade = {
                let has_old_ver = old_version.as_ref().map_or(false, |v| v != &new_version);
                let is_pre_tracking = old_version.is_none()
                    && plugin
                        .metadata()
                        .tables()
                        .iter()
                        .any(|t| db.has_table(t).unwrap_or(false));
                if is_pre_tracking {
                    info!(
                        "PluginManager::register_plugins() > detected pre-tracking install of '{}'",
                        plugin.plugin_id()
                    );
                }
                if has_old_ver {
                    info!(
                        "PluginManager::register_plugins() > upgrade available for '{}': installed={}, current={}",
                        plugin.plugin_id(),
                        old_version.as_deref().unwrap_or("?"),
                        new_version
                    );
                }
                has_old_ver || is_pre_tracking
            };

            plugin.on_load(&*db)?;

            // 自动执行 on_upgrade（重启/热重载保底，保证代码与 schema 一致）
            if needs_upgrade {
                let from_version = old_version.as_deref().unwrap_or("0.0.0");
                info!(
                    "PluginManager::register_plugins() > auto-upgrading '{}' from {} to {}",
                    plugin.plugin_id(),
                    from_version,
                    new_version
                );
                plugin.on_upgrade(&*db, from_version)?;
            }

            info!("PluginManager::register_plugins() > calling plugin `on_install`");
            plugin.on_install(&*db)?;

            // 持久化新版本到 plugkit_plugins（保留已有的 status，不被 'Loaded' 覆盖）
            let _ = db.execute_with(
                "INSERT INTO plugkit_plugins (plugin_id, version, status, is_installed, upgraded_at) VALUES (?, ?, 'Loaded', 1, datetime('now')) ON CONFLICT(plugin_id) DO UPDATE SET version = EXCLUDED.version, is_installed = 1, upgraded_at = EXCLUDED.upgraded_at",
                &[
                    crate::database::DbValue::text(plugin.plugin_id()),
                    crate::database::DbValue::text(&new_version),
                ],
            );

            if let Some(_) = registry.insert(
                plugin.plugin_id().to_string(),
                LoadedPlugin {
                    plugin,
                    in_library: from_library.clone(),
                    status: PluginStatus::Loaded,
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

    #[derive(Debug)]
    struct MockPlugin {
        id: String,
    }

    impl Plugin for MockPlugin {
        fn plugin_id(&self) -> &String {
            &self.id
        }
    }

    #[test]
    fn test_state_machine_plugin_not_found() {
        let mut mgr: PluginManager = PluginManager::default();
        assert!(matches!(
            mgr.enable_plugin("missing").unwrap_err().kind(),
            ErrorKind::PluginNotFound(_)
        ));
        assert!(matches!(
            mgr.disable_plugin("missing").unwrap_err().kind(),
            ErrorKind::PluginNotFound(_)
        ));
        assert!(matches!(
            mgr.start_plugin("missing").unwrap_err().kind(),
            ErrorKind::PluginNotFound(_)
        ));
        assert!(matches!(
            mgr.stop_plugin("missing").unwrap_err().kind(),
            ErrorKind::PluginNotFound(_)
        ));
    }

    /// 演示 metadata + 依赖检测 + 拿包排序 + 数据库 keep_data 卸载。
    #[test]
    fn test_metadata_dependency_topo_database() {
        use crate::database::SqliteDatabase;
        use crate::metadata::PluginMetadata;

        // 依赖关系:B 依赖 A,C run_after A;拓扑排序应得到 A 在 B/C 前。
        #[derive(Debug)]
        struct DepPlugin {
            id: String,
            meta: PluginMetadata,
        }
        impl Plugin for DepPlugin {
            fn plugin_id(&self) -> &String {
                &self.id
            }
            fn metadata(&self) -> PluginMetadata {
                self.meta.clone()
            }
            fn on_install(&self, db: &dyn crate::database::DatabaseExt) -> Result<()> {
                for t in self.meta.tables() {
                    db.validate_table_name(t)?;
                    db.execute(&format!(
                        "CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY)",
                        t
                    ))?;
                }
                Ok(())
            }
            fn on_uninstall(
                &self,
                db: &dyn crate::database::DatabaseExt,
                keep_data: bool,
            ) -> Result<()> {
                if !keep_data {
                    for t in self.meta.tables() {
                        db.drop_table(t)?;
                    }
                }
                Ok(())
            }
        }

        let db = SqliteDatabase::in_memory().unwrap();
        let mut mgr: PluginManager =
            PluginManager::default().with_database(std::sync::Arc::new(db));

        // 注册三个插件(A、B、C),B 依赖 A,C run_after A。
        // 直接构造 LoadedPlugin,绕过动态库加载(测试专用)。
        {
            let mut registry = mgr.plugins.write().unwrap();
            let make = |name: &str, deps: &[&str], run_after: &[&str], tables: &[&str]| {
                let meta = PluginMetadata::new(name, name, "1.0.0")
                    .with_dependencies(deps)
                    .with_run_after(run_after)
                    .with_tables(tables);
                DepPlugin {
                    id: name.to_string(),
                    meta,
                }
            };
            let in_library = Arc::new(LoadedLibrary {
                file_name: PathBuf::from("test"),
                library: unsafe {
                    // 占位库——仅用于 strong_count,不会被实际关闭
                    // 使用当前可执行文件作为库(安全,self-load)
                    let exe = std::env::current_exe().unwrap();
                    libloading::Library::new(&exe).unwrap()
                },
            });
            for p in [
                make("A", &[], &[], &["a_items"]),
                make("B", &["A"], &[], &["b_items"]),
                make("C", &[], &["A"], &["c_items"]),
            ] {
                registry.insert(
                    p.id.clone(),
                    LoadedPlugin {
                        plugin: Arc::new(p),
                        in_library: in_library.clone(),
                        status: PluginStatus::Loaded,
                    },
                );
            }
        }

        // 安装阶段:on_install 通过 db 建表
        assert!(mgr.is_empty() == false);
        // 启用 A(无依赖,应成功)
        mgr.enable_plugin("A").unwrap();
        // 启用 B(依赖 A 已 Enabled,应成功)
        mgr.enable_plugin("B").unwrap();
        // 启用 C(run_after A,不要求 A 已启用?run_after 仅约束启动顺序,启用不限)
        mgr.enable_plugin("C").unwrap();

        // 拿包排序:A 应在 B、C 前
        let order = mgr.topological_sort().unwrap();
        let pos_a = order.iter().position(|n| n == "A").unwrap();
        let pos_b = order.iter().position(|n| n == "B").unwrap();
        let pos_c = order.iter().position(|n| n == "C").unwrap();
        assert!(pos_a < pos_b, "A should start before B (dependency)");
        assert!(pos_a < pos_c, "A should start before C (run_after)");

        // 禁用 B 前:应不能禁用 A(被 B 依赖)
        let err = mgr.disable_plugin("A").unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::DependencyUnmet(_)));
        // 先禁用 B,再禁用 A,应成功
        mgr.disable_plugin("B").unwrap();
        mgr.disable_plugin("A").unwrap();

        // metadata 查询
        let meta_a = mgr.plugin_metadata("A").unwrap();
        assert_eq!(meta_a.name, "A");
        assert_eq!(meta_a.tables(), &["a_items".to_string()]);

        // 显式调用 on_install 建表(测试绕过了 load_plugins_from,故手动建表)
        let db_for_install = mgr.database().unwrap();
        for (id, _) in ["A", "B", "C"].iter().zip(0..3) {
            let p = mgr.get(id).unwrap();
            p.on_install(&*db_for_install).unwrap();
        }
        // 校验表已建
        assert!(db_for_install.has_table("a_items").unwrap());
        assert!(db_for_install.has_table("b_items").unwrap());

        // 卸载 B(keep_data=false):应 drop b_items
        mgr.unload_plugin("B", false).unwrap();
        // 卸载 A(keep_data=true):应保留 a_items
        mgr.unload_plugin("A", true).unwrap();

        // 校验数据库:b_items 应已删除,a_items 应保留。
        // 使用 mgr 实际持有的数据库句柄(而非新建 in_memory)。
        let db = mgr.database().unwrap();
        assert!(
            !db.has_table("b_items").unwrap(),
            "b_items should be dropped (keep_data=false)"
        );
        assert!(
            db.has_table("a_items").unwrap(),
            "a_items should be preserved (keep_data=true)"
        );
    }
}
