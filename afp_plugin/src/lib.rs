/*!
法新社（AFP）新闻机构插件。

通过 [`afp_format`] 注册一个配置了法新社格式化风格的 [`NewsAgencyPlugin`] 实例。
*/

use dygpi::plugin::PluginRegistrar;
use news_api::{afp_format, NewsAgencyPlugin};

// ------------------------------------------------------------------------------------------------
// 注册
// ------------------------------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar<NewsAgencyPlugin>) {
    registrar.register(NewsAgencyPlugin::new(
        PLUGIN_ID,
        "Agence France-Presse",
        afp_format,
    ));
}

// ------------------------------------------------------------------------------------------------
// 插件标识符
// ------------------------------------------------------------------------------------------------

const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "::",
    module_path!(),
    "::",
    "AfpAgency"
);
