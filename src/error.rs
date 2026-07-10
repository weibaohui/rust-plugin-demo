/*!
提供该 crate 中使用的 [`Error`](struct.Error.html)、[`ErrorKind`](enum.ErrorKind.html) 和
[`Result`](enum.Result.html) 类型。
*/

use std::fmt::{Display, Formatter};

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 该 crate 中函数和方法返回的错误类型。
///
#[derive(Debug)]
pub enum ErrorKind {
    ///
    /// 加载指定文件名的动态库失败。
    /// 第一个参数是库路径，第二个参数是底层系统错误。
    ///
    LibraryOpenFailed(String, Box<dyn std::error::Error>),
    ///
    /// 关闭动态库并释放资源失败。
    /// 第一个参数是库路径，第二个参数是底层系统错误。
    ///
    LibraryCloseFailed(String, Box<dyn std::error::Error>),
    ///
    /// 在动态库中查找符号失败。
    /// 第一个参数是库路径，第二个参数是底层系统错误。
    ///
    SymbolNotFound(String, Box<dyn std::error::Error>),
    ///
    /// 插件宿主与插件库版本不兼容。
    /// 参数为不兼容的库路径。
    ///
    IncompatibleLibraryVersion(String),
    ///
    /// 尝试注册插件时，插件库报告了错误。
    /// 参数为插件库提供给注册器的错误。
    ///
    PluginRegistration(Box<dyn std::error::Error>),
    ///
    /// 配置中找不到指定的插件管理器类型。
    /// 参数为未找到的插件类型标识符。
    ///
    UnknownPluginManagerType(String),
    /// 插件不存在(未加载或已卸载)。参数为插件 ID。
    ///
    PluginNotFound(String),
    /// 插件状态不符合操作要求(如非 Loaded 时 enable)。参数为说明。
    ///
    InvalidPluginState(String),
    /// 数据库操作失败。参数为说明(含底层错误信息)。
    ///
    DatabaseError(String),
    /// 插件依赖未满足(启用前依赖未启用,或禁用前被其他插件依赖)。参数为说明。
    ///
    DependencyUnmet(String),
    /// 检测到循环依赖,无法进行拓扑排序。参数为参与循环的插件名列表。
    ///
    CircularDependency(String),
}

///
/// 使用 [`ErrorKind`](enum.ErrorKind.html) 的 `std::error::Error` 实现。
///
#[derive(Debug)]
pub struct Error(ErrorKind);

///
/// `std::result::Result` 的别名，始终返回该 crate 的 [`Error`](struct.Error.html) 类型。
///
pub type Result<T> = std::result::Result<T, Error>;

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ErrorKind::LibraryOpenFailed(path, error) =>
                    format!("Library '{}' failed to close; error: '{}'", path, error),
                ErrorKind::SymbolNotFound(name, in_library) => format!(
                    "Could not find symbol '{}' in library '{}'",
                    name, in_library
                ),
                ErrorKind::LibraryCloseFailed(path, error) =>
                    format!("Library '{}' failed to close; error: '{}'", path, error),
                ErrorKind::IncompatibleLibraryVersion(path) =>
                    format!("Library '{}' has incompatible version", path),
                ErrorKind::PluginRegistration(error) =>
                    format!("Plugin(s) failed to register; error: '{}'", error),
                ErrorKind::UnknownPluginManagerType(plugin_type) =>
                    format!("No Configured plugins for type '{}'", plugin_type),
                ErrorKind::PluginNotFound(id) => format!("Plugin '{}' not found", id),
                ErrorKind::InvalidPluginState(msg) => format!("Invalid plugin state: {}", msg),
                ErrorKind::DatabaseError(msg) => format!("Database error: {}", msg),
                ErrorKind::DependencyUnmet(msg) => format!("Dependency unmet: {}", msg),
                ErrorKind::CircularDependency(msg) => format!("Circular dependency: {}", msg),
            }
        )
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ErrorKind> for Error {
    fn from(v: ErrorKind) -> Self {
        Self(v)
    }
}

impl Error {
    /// 返回错误类别。
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorKind::LibraryOpenFailed(_, error) => Some(error.as_ref()),
            ErrorKind::LibraryCloseFailed(_, error) => Some(error.as_ref()),
            ErrorKind::PluginRegistration(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

// ------------------------------------------------------------------------------------------------
// 私有函数
// ------------------------------------------------------------------------------------------------

// ------------------------------------------------------------------------------------------------
// 模块
// ------------------------------------------------------------------------------------------------
