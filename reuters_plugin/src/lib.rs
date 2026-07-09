/*!
路透社新闻机构插件。

通过 [`reuters_format`] 注册一个配置了路透社格式化风格的 [`NewsAgencyPlugin`] 实例。
编译期将 `ui/dist/` 嵌入到本插件的 `.so`/`.dylib` 中，使宿主可直接从内存服务前端。
*/

use dygpi::plugin::PluginRegistrar;
use include_dir::{include_dir, Dir};
use news_api::{reuters_format, NewsAgencyPlugin, PluginMenu};

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
            // 将嵌入的 ui/dist 绑定到本插件实例，基目录 "reuters_plugin/ui"，
            // 宿主 news_server 据此从内存服务 qiankun 子应用。
            .with_ui_dist("reuters_plugin/ui", &UI_DIST)
            // 声明左侧菜单：路透社分组 → 控制面板（点击进入 qiankun 子应用）。
            .with_menu(PluginMenu {
                key: "reuters".into(),
                title: "路透社".into(),
                icon: Some("📰".into()),
                route: None,
                order: 100,
                children: vec![PluginMenu {
                    key: "panel".into(),
                    title: "控制面板".into(),
                    icon: None,
                    route: Some(format!("/plugin/{}", PLUGIN_ID)),
                    order: 0,
                    children: vec![],
                }],
            })
            // 生命周期钩子回调(演示用法:每个钩子自定义行为)
            .with_on_enable(on_enable)
            .with_on_start(on_start)
            .with_on_stop(on_stop)
            .with_on_disable(on_disable)
            .with_on_cron(on_cron),
    );
}

// ------------------------------------------------------------------------------------------------
// 插件生命周期钩子
// ------------------------------------------------------------------------------------------------
// 本插件通过 `NewsAgencyPlugin` 实现 `dygpi::plugin::Plugin` trait,钩子在
// `news_api/src/lib.rs` 的 `impl Plugin` 中实现。宿主(news_server)按状态机调用:
//
//   load    → on_load      + on_install   加载库 + 数据初始化(幂等)         → Loaded
//   enable  → on_enable                   菜单可见、API 可访问              → Enabled
//   start   → on_start     + cron 注册    后台任务;本插件声明 heartbeat 30s → Running
//   stop    → on_stop      + cron 注销    停止后台任务                      → Enabled
//   disable → on_disable                  菜单隐藏、收敛能力                → Loaded
//   unload  → on_uninstall + on_unload    清理 + 关库(Running/Enabled 自动先 stop/disable)
//
// cron:`NewsAgencyPlugin::cron_specs()` 返回 `[{ name: "heartbeat", interval_secs: 30 }]`,
// `on_cron("heartbeat")` 由宿主在 Running 时周期调用(打日志)。
//
// 插件开发者如需自定义生命周期行为,在 `news_api` 的 `impl Plugin` 中覆盖对应钩子
// (钩子默认 no-op,只覆盖需要的)。

// ------------------------------------------------------------------------------------------------
// 生命周期钩子回调实现(演示用法)
// ------------------------------------------------------------------------------------------------
// 注意:本 crate 编译为 dylib 动态加载,log crate 的全局 logger 是进程级 static,
// dylib 内未初始化,因此 log::info! 不会有输出。这里用 eprintln! 直接写 stderr,
// 宿主(news_server)可在其标准错误看到。生产中可通过 HostContext.log_message 上报日志。

fn on_enable() {
    eprintln!("[reuters] enabled: 初始化资源,菜单可见");
}

fn on_start() {
    eprintln!("[reuters] started: 后台任务就绪,heartbeat 30s");
}

fn on_stop() {
    eprintln!("[reuters] stopped: 后台任务停止");
}

fn on_disable() {
    eprintln!("[reuters] disabled: 菜单隐藏,收敛能力");
}

fn on_cron() {
    eprintln!("[reuters] heartbeat: 定时检查");
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
