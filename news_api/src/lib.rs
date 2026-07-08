/*!
新闻机构插件演示的共享类型。

定义所有新闻机构插件库将实例化和注册的插件类型 [`NewsAgencyPlugin`]，
以及由 `publish()` 方法产生的 [`NewsArticle`]。

定义了 [`HostContext`] trait，让插件可以反向调用宿主的方法并获取宿主信息。
*/

use dygpi::plugin::Plugin;
use serde::Serialize;

// 让插件可以嵌入自身的 `ui/dist/` 目录树，并在宿主需要时回放。
// 宿主 (`news_server`) 也用同一个类型来遍历目录。
pub use include_dir::{Dir, File};

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 由插件的 `publish` 方法产生的新闻文章。
///
#[derive(Debug, Clone)]
pub struct NewsArticle {
    /// 标题。
    pub headline: String,
    /// 正文内容。
    pub body: String,
    /// 电头（城市/地点）。
    pub dateline: String,
    /// 发布此文章的机构。
    pub agency: String,
}

///
/// 宿主上下文 trait。
///
/// 插件可以通过此 trait 从宿主获取信息并调用宿主方法，
/// 实现宿主 ↔ 插件的双向通信。
///
/// 宿主实现此 trait，并在调用 `publish()` 时传入 `&dyn HostContext`。
///
pub trait HostContext: Send + Sync {
    /// 宿主服务器名称。
    fn server_name(&self) -> &str;
    /// 宿主框架版本。
    fn server_version(&self) -> &str;
    /// 已发布文章总数。
    fn article_count(&self) -> usize;
    /// 向宿主日志记录一条消息。
    fn log_message(&self, msg: &str);
    /// 获取当前服务器时间（格式化的时间字符串）。
    fn server_time(&self) -> String;
    /// 当前已加载的插件数量。
    fn plugin_count(&self) -> usize;
}

///
/// 插件向前端声明的菜单项（参考 k8m 的 `Menu`）。
///
/// 插件在 `register_plugins` 时通过 `with_menu(...)` 自描述菜单树；宿主聚合所有
/// 已加载插件的菜单后交前端 Sidebar 渲染。菜单内容静态，可见性由插件是否加载决定。
///
#[derive(Debug, Clone, Serialize)]
pub struct PluginMenu {
    /// 菜单唯一标识（插件内唯一）。
    pub key: String,
    /// 展示标题。
    pub title: String,
    /// 图标（emoji 或 CSS class），可选。
    pub icon: Option<String>,
    /// 点击跳转的路由；`None` 表示纯分组节点（仅展开子菜单）。
    pub route: Option<String>,
    /// 排序权重，越小越靠前。
    pub order: i32,
    /// 子菜单（树形）。
    pub children: Vec<PluginMenu>,
}

///
/// 代表新闻机构的插件。每个机构库使用自己的风格（路透社、法新社、美联社……）
/// 注册此类型的一个实例。
///
#[derive(Debug)]
pub struct NewsAgencyPlugin {
    id: String,
    agency_name: String,
    format_fn: fn(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle,
    /// 插件 UI 的基目录（相对项目根，如 `"afp_plugin/ui"`），其下应有 qiankun
    /// 子应用产物 `dist/`。宿主据此把 `/plugin-files/...` URL 映射回本插件。
    ui_base_dir: Option<String>,
    /// 嵌入到插件 .so 中的 `ui/dist/` 目录（编译期嵌入）。
    /// 当设置后，宿主可以从内存中直接服务前端静态文件，无需再访问磁盘。
    ui_dist: Option<&'static Dir<'static>>,
    /// 插件向前端声明的菜单树。
    menus: Vec<PluginMenu>,
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl Plugin for NewsAgencyPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn on_load(&self) -> dygpi::error::Result<()> {
        log::info!("News agency '{}' loaded.", self.agency_name);
        Ok(())
    }

    fn on_unload(&self) -> dygpi::error::Result<()> {
        log::info!("News agency '{}' unloaded.", self.agency_name);
        Ok(())
    }
}

impl NewsAgencyPlugin {
    ///
    /// 创建一个新的新闻机构插件。
    ///
    /// * `id` — 唯一的插件标识符（通常是 crate::module 路径）。
    /// * `agency_name` — 机构的人类可读名称。
    /// * `format_fn` — 将标题和正文格式化为最终 `NewsArticle` 的函数。
    ///   该函数接收 `&dyn HostContext`，插件可通过它访问宿主能力和信息。
    ///
    pub fn new(
        id: &str,
        agency_name: &str,
        format_fn: fn(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle,
    ) -> Self {
        Self {
            id: id.to_string(),
            agency_name: agency_name.to_string(),
            format_fn,
            ui_base_dir: None,
            ui_dist: None,
            menus: vec![],
        }
    }

    ///
    /// 将编译期嵌入的 `ui/dist/` 目录绑定到本插件，并声明其基目录。
    ///
    /// `base_dir` 是 UI 相对项目根的目录（如 `"afp_plugin/ui"`），其下应有
    /// qiankun 子应用产物 `dist/index.html`。宿主据此把 `/plugin-files/...`
    /// URL 映射回本插件，并优先从内存服务。
    ///
    /// 配合 `include_dir!` 宏使用，例如：
    ///
    /// ```ignore
    /// pub static UI_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");
    /// NewsAgencyPlugin::new(...).with_ui_dist("afp_plugin/ui", &UI_DIST)
    /// ```
    ///
    /// 一旦绑定，宿主 `news_server` 会优先从内存中服务插件前端，
    /// 即使磁盘上的 `ui/dist/` 被删除也能正常工作。
    ///
    pub fn with_ui_dist(mut self, base_dir: &str, dist: &'static Dir<'static>) -> Self {
        self.ui_base_dir = Some(base_dir.to_string());
        self.ui_dist = Some(dist);
        self
    }

    ///
    /// 声明一个菜单项（builder 风格，可链式多次调用）。
    ///
    /// 插件自描述菜单树，宿主聚合后交前端 Sidebar 渲染。例如：
    ///
    /// ```ignore
    /// NewsAgencyPlugin::new(...)
    ///     .with_menu(PluginMenu {
    ///         key: "afp".into(), title: "法新社".into(), icon: Some("📡".into()),
    ///         route: None, order: 100,
    ///         children: vec![PluginMenu {
    ///             key: "panel".into(), title: "控制面板".into(), icon: None,
    ///             route: Some(format!("/plugin/{}", PLUGIN_ID)),
    ///             order: 0, children: vec![],
    ///         }],
    ///     })
    /// ```
    ///
    pub fn with_menu(mut self, menu: PluginMenu) -> Self {
        self.menus.push(menu);
        self
    }

    /// 返回插件 UI 的基目录（如 `"afp_plugin/ui"`），未声明 UI 则为 None。
    pub fn ui_base_dir(&self) -> Option<&str> {
        self.ui_base_dir.as_deref()
    }

    /// 返回编译期嵌入到插件二进制中的 `ui/dist/` 目录（若有）。
    /// 宿主优先用此目录服务前端静态文件。
    pub fn ui_dist(&self) -> Option<&'static Dir<'static>> {
        self.ui_dist
    }

    /// 插件是否声明了前端 UI（即绑定了嵌入的 `ui_dist`）。
    pub fn has_ui(&self) -> bool {
        self.ui_dist.is_some()
    }

    /// 返回插件声明的菜单树。
    pub fn menus(&self) -> &[PluginMenu] {
        &self.menus
    }

    /// 返回人类可读的机构名称。
    pub fn agency_name(&self) -> &str {
        &self.agency_name
    }

    ///
    /// 发布一篇新闻文章。格式（标题风格、电头、模板文本）
    /// 由创建插件时传入的 `format_fn` 决定。
    ///
    /// `ctx` 提供宿主的上下文信息和回调能力，供 `format_fn` 内部使用。
    ///
    pub fn publish(&self, ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle {
        let mut article = (self.format_fn)(ctx, headline, body);
        article.agency = self.agency_name.clone();
        article
    }
}

// ------------------------------------------------------------------------------------------------
// 预定义的格式化风格函数，供插件库使用
// ------------------------------------------------------------------------------------------------

/// 路透社风格：简洁纪实，"[REUTERS] 标题" 前缀，电头 "LONDON"。
/// 使用宿主上下文获取服务器时间并记录日志。
pub fn reuters_format(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle {
    ctx.log_message(&format!("Reuters formatting news: {}", headline));
    NewsArticle {
        headline: format!("[REUTERS] {}", headline),
        body: format!(
            "{} — Reporting by Reuters correspondents.\n\n---\n[Host: {} v{} | {} | {} plugins loaded, {} articles]",
            body,
            ctx.server_name(),
            ctx.server_version(),
            ctx.server_time(),
            ctx.plugin_count(),
            ctx.article_count(),
        ),
        dateline: "LONDON".to_string(),
        agency: String::new(), // 由 publish() 填充
    }
}

/// 法新社风格："标题 — AFP"，电头 "PARIS"。
/// 使用宿主上下文获取服务器时间并记录日志。
pub fn afp_format(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle {
    ctx.log_message(&format!("AFP formatting news: {}", headline));
    NewsArticle {
        headline: format!("{} — AFP", headline),
        body: format!(
            "{} [AFP correspondents worldwide]\n\n---\n[Host: {} v{} | {} | {} plugins loaded, {} articles]",
            body,
            ctx.server_name(),
            ctx.server_version(),
            ctx.server_time(),
            ctx.plugin_count(),
            ctx.article_count(),
        ),
        dateline: "PARIS".to_string(),
        agency: String::new(),
    }
}

/// 美联社风格："AP News: 标题"，电头 "NEW YORK"。
/// 使用宿主上下文获取服务器时间并记录日志。
pub fn ap_format(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle {
    ctx.log_message(&format!("AP formatting news: {}", headline));
    NewsArticle {
        headline: format!("AP News: {}", headline),
        body: format!(
            "{} (The Associated Press)\n\n---\n[Host: {} v{} | {} | {} plugins loaded, {} articles]",
            body,
            ctx.server_name(),
            ctx.server_version(),
            ctx.server_time(),
            ctx.plugin_count(),
            ctx.article_count(),
        ),
        dateline: "NEW YORK".to_string(),
        agency: String::new(),
    }
}

/// 塔斯社风格："标题 — TASS"，电头 "MOSCOW"。
/// 使用宿主上下文获取服务器时间并记录日志。
pub fn tass_format(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle {
    ctx.log_message(&format!("TASS formatting news: {}", headline));
    NewsArticle {
        headline: format!("{} — TASS", headline),
        body: format!(
            "{}, as reported by TASS.\n\n---\n[Host: {} v{} | {} | {} plugins loaded, {} articles]",
            body,
            ctx.server_name(),
            ctx.server_version(),
            ctx.server_time(),
            ctx.plugin_count(),
            ctx.article_count(),
        ),
        dateline: "MOSCOW".to_string(),
        agency: String::new(),
    }
}
