/*!
路透社新闻机构插件。

通过 [`reuters_format`] 注册一个配置了路透社格式化风格的 [`NewsAgencyPlugin`] 实例。
编译期将 `ui/dist/` 嵌入到本插件的 `.so`/`.dylib` 中，使宿主可直接从内存服务前端。
*/

use dygpi::plugin::PluginRegistrar;
use include_dir::{include_dir, Dir};
use news_api::{reuters_format, NewsAgencyPlugin, PluginModuleType};

// ------------------------------------------------------------------------------------------------
// 编译期嵌入的 `ui/dist/` 目录
// ------------------------------------------------------------------------------------------------

/// 由 `include_dir!` 宏在编译期把整个 `reuters_plugin/ui/dist/` 目录打包进本 .so。
/// 路径在二进制内部是只读的，但可以通过 `Dir` 的 API 按需遍历。
pub static UI_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

// ------------------------------------------------------------------------------------------------
// 注册
// ------------------------------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar<NewsAgencyPlugin>) {
    registrar.register(
        NewsAgencyPlugin::new(PLUGIN_ID, "Reuters", reuters_format)
            .with_ui(
                PluginModuleType::React,
                "react",
                "reuters_plugin/ui/panel.js",
            )
            // 将嵌入的 ui/dist/ 绑定到本插件实例，
            // 宿主 news_server 会优先从内存服务。
            .with_ui_dist(&UI_DIST),
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
