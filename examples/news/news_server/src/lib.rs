/*!
新闻机构插件演示的共享类型（合并自 news_api）。

定义所有新闻机构插件库将实例化和注册的插件类型 [`NewsAgencyPlugin`]，
以及由 `publish()` 方法产生的 [`NewsArticle`]。

定义了 [`HostContext`] trait，让插件可以反向调用宿主的方法并获取宿主信息。
*/

use plugkit::database::DatabaseExt;
use plugkit::metadata::PluginMetadata;
use plugkit::plugin::Plugin;
use serde::Serialize;

// 让插件可以嵌入自身的 `ui/dist/` 目录树，并在宿主需要时回放。
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
    /// 生命周期钩子回调(可选,插件自定义行为;None 时用默认日志)。
    on_enable_fn: Option<fn()>,
    on_start_fn: Option<fn()>,
    on_stop_fn: Option<fn()>,
    on_disable_fn: Option<fn()>,
    on_cron_fn: Option<fn()>,

    // ---- metadata 字段(参考 npm package.json + k8m Meta) ----
    /// 元信息:插件唯一标识(系统级)。建议与 `id` 的最后一段一致。
    metadata_name: String,
    /// 元信息:版本号(语义化)。
    metadata_version: String,
    /// 元信息:功能描述(可选)。
    metadata_description: Option<String>,
    /// 元信息:作者(可选)。
    metadata_author: Option<String>,
    /// 元信息:主页 URL(可选)。
    metadata_homepage: Option<String>,
    /// 元信息:许可证(可选)。
    metadata_license: Option<String>,
    /// 元信息:使用的数据库表名列表(卸载时据此清理)。
    metadata_tables: Vec<String>,
    /// 元信息:强依赖插件名列表(启用前必须确保都已启用)。
    metadata_dependencies: Vec<String>,
    /// 元信息:启动顺序约束(非依赖,但必须在它们之后启动)。
    metadata_run_after: Vec<String>,
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl Plugin for NewsAgencyPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn metadata(&self) -> PluginMetadata {
        let mut meta = PluginMetadata::new(
            &self.metadata_name,
            &self.agency_name,
            &self.metadata_version,
        )
        .with_icon("📰");
        if let Some(desc) = &self.metadata_description {
            meta = meta.with_description(desc);
        }
        if let Some(author) = &self.metadata_author {
            meta = meta.with_author(author);
        }
        if let Some(homepage) = &self.metadata_homepage {
            meta = meta.with_homepage(homepage);
        }
        if let Some(license) = &self.metadata_license {
            meta = meta.with_license(license);
        }
        // 菜单树:PluginMenu 转为 plugkit::metadata::PluginMenu
        if !self.menus.is_empty() {
            meta = meta.with_menus(self.menus.iter().map(convert_menu).collect());
        }
        meta = meta.with_tables_owned(self.metadata_tables.clone());
        meta = meta.with_dependencies_owned(self.metadata_dependencies.clone());
        meta = meta.with_run_after_owned(self.metadata_run_after.clone());
        meta
    }

    fn on_load(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' loaded.", self.agency_name);
        Ok(())
    }

    fn on_unload(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' unloaded.", self.agency_name);
        Ok(())
    }

    fn on_install(&self, db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' installed.", self.agency_name);
        // 演示:按 metadata.tables() 声明逐个建表(幂等)
        for table in &self.metadata_tables {
            db.validate_table_name(table)?;
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY, headline TEXT, body TEXT)",
                table
            );
            db.execute(&sql)?;
        }
        Ok(())
    }

    fn on_uninstall(&self, db: &dyn DatabaseExt, keep_data: bool) -> plugkit::error::Result<()> {
        log::info!(
            "News agency '{}' uninstalled (keep_data={}).",
            self.agency_name,
            keep_data
        );
        if !keep_data {
            for table in &self.metadata_tables {
                db.drop_table(table)?;
            }
        }
        Ok(())
    }

    fn on_enable(&self) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' enabled.", self.agency_name);
        if let Some(f) = self.on_enable_fn {
            f();
        }
        Ok(())
    }

    fn on_disable(&self) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' disabled.", self.agency_name);
        if let Some(f) = self.on_disable_fn {
            f();
        }
        Ok(())
    }

    fn on_start(&self) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' started.", self.agency_name);
        if let Some(f) = self.on_start_fn {
            f();
        }
        Ok(())
    }

    fn on_stop(&self) -> plugkit::error::Result<()> {
        log::info!("News agency '{}' stopped.", self.agency_name);
        if let Some(f) = self.on_stop_fn {
            f();
        }
        Ok(())
    }

    fn on_cron(&self, name: &str) -> plugkit::error::Result<()> {
        log::info!("[{}] cron tick: {}", self.agency_name, name);
        if let Some(f) = self.on_cron_fn {
            f();
        }
        Ok(())
    }

    fn cron_specs(&self) -> Vec<plugkit::metadata::CronSpec> {
        vec![plugkit::metadata::CronSpec {
            name: "heartbeat".to_string(),
            interval_secs: 30,
        }]
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
            on_enable_fn: None,
            on_start_fn: None,
            on_stop_fn: None,
            on_disable_fn: None,
            on_cron_fn: None,
            metadata_name: id.to_string(),
            metadata_version: env!("CARGO_PKG_VERSION").to_string(),
            metadata_description: None,
            metadata_author: None,
            metadata_homepage: None,
            metadata_license: None,
            metadata_tables: vec![],
            metadata_dependencies: vec![],
            metadata_run_after: vec![],
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

    /// 设置 `on_enable` 钩子回调(启用时调用,自定义初始化行为)。
    pub fn with_on_enable(mut self, f: fn()) -> Self {
        self.on_enable_fn = Some(f);
        self
    }

    /// 设置 `on_start` 钩子回调(启动后台任务时调用)。
    pub fn with_on_start(mut self, f: fn()) -> Self {
        self.on_start_fn = Some(f);
        self
    }

    /// 设置 `on_stop` 钩子回调(停止后台任务时调用)。
    pub fn with_on_stop(mut self, f: fn()) -> Self {
        self.on_stop_fn = Some(f);
        self
    }

    /// 设置 `on_disable` 钩子回调(禁用时调用)。
    pub fn with_on_disable(mut self, f: fn()) -> Self {
        self.on_disable_fn = Some(f);
        self
    }

    /// 设置 `on_cron` 钩子回调(定时任务执行时调用)。
    pub fn with_on_cron(mut self, f: fn()) -> Self {
        self.on_cron_fn = Some(f);
        self
    }

    // ---- metadata builders(参考 npm package.json + k8m Meta) ----

    /// 设置元信息:插件唯一标识(系统级)。默认与 `id` 一致。
    pub fn with_metadata_name(mut self, name: &str) -> Self {
        self.metadata_name = name.to_string();
        self
    }

    /// 设置元信息:版本号(语义化)。默认取 `CARGO_PKG_VERSION`。
    pub fn with_metadata_version(mut self, version: &str) -> Self {
        self.metadata_version = version.to_string();
        self
    }

    /// 设置元信息:功能描述。
    pub fn with_metadata_description(mut self, desc: &str) -> Self {
        self.metadata_description = Some(desc.to_string());
        self
    }

    /// 设置元信息:作者。
    pub fn with_metadata_author(mut self, author: &str) -> Self {
        self.metadata_author = Some(author.to_string());
        self
    }

    /// 设置元信息:主页 URL。
    pub fn with_metadata_homepage(mut self, url: &str) -> Self {
        self.metadata_homepage = Some(url.to_string());
        self
    }

    /// 设置元信息:许可证。
    pub fn with_metadata_license(mut self, license: &str) -> Self {
        self.metadata_license = Some(license.to_string());
        self
    }

    /// 设置元信息:使用的数据库表名列表(卸载时据此清理)。
    pub fn with_metadata_tables(mut self, tables: &[&str]) -> Self {
        self.metadata_tables = tables.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置元信息:强依赖插件名列表(启用前必须确保都已启用)。
    pub fn with_metadata_dependencies(mut self, deps: &[&str]) -> Self {
        self.metadata_dependencies = deps.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置元信息:启动顺序约束(非依赖,但必须在它们之后启动)。
    pub fn with_metadata_run_after(mut self, deps: &[&str]) -> Self {
        self.metadata_run_after = deps.iter().map(|s| s.to_string()).collect();
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

// ------------------------------------------------------------------------------------------------
// 私有辅助:PluginMenu → plugkit::metadata::PluginMenu
// ------------------------------------------------------------------------------------------------

fn convert_menu(m: &PluginMenu) -> plugkit::metadata::PluginMenu {
    plugkit::metadata::PluginMenu {
        key: m.key.clone(),
        title: m.title.clone(),
        icon: m.icon.clone(),
        route: m.route.clone(),
        order: m.order,
        children: m.children.iter().map(convert_menu).collect(),
    }
}