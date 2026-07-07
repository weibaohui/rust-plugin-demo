/*!
路透社新闻机构插件。

通过 [`reuters_format`] 注册一个配置了路透社格式化风格的 [`NewsAgencyPlugin`] 实例。
*/

use dygpi::plugin::PluginRegistrar;
use news_api::{reuters_format, NewsAgencyPlugin};

// ------------------------------------------------------------------------------------------------
// 注册
// ------------------------------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar<NewsAgencyPlugin>) {
    registrar.register(
        NewsAgencyPlugin::new(PLUGIN_ID, "Reuters", reuters_format)
            .with_ui("reuters-plugin-ui", "reuters_plugin/ui.js"),
    );
}

// ------------------------------------------------------------------------------------------------
// 插件标识符
// ------------------------------------------------------------------------------------------------

const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "::",
    module_path!(),
    "::",
    "ReutersAgency"
);
