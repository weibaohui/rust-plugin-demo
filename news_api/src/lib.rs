/*!
新闻机构插件演示的共享类型。

定义所有新闻机构插件库将实例化和注册的插件类型 [`NewsAgencyPlugin`]，
以及由 `publish()` 方法产生的 [`NewsArticle`]。

定义了 [`HostContext`] trait，让插件可以反向调用宿主的方法并获取宿主信息。
*/

use dygpi::plugin::Plugin;

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
/// 代表新闻机构的插件。每个机构库使用自己的风格（路透社、法新社、美联社……）
/// 注册此类型的一个实例。
///
#[derive(Debug)]
pub struct NewsAgencyPlugin {
    id: String,
    agency_name: String,
    format_fn: fn(ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle,
    /// 自定义 HTML 标签名（Web Component），用于在主框架中渲染插件专属 UI
    ui_tag_name: Option<String>,
    /// 插件 UI 的 JS 文件路径（相对于 static 目录）
    ui_js_path: Option<String>,
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
            ui_tag_name: None,
            ui_js_path: None,
        }
    }

    ///
    /// 设置插件 UI 元数据并返回自身（builder 风格）。
    ///
    /// * `tag_name` — 自定义 HTML 标签名，例如 `"reuters-plugin-ui"`。
    /// * `js_path` — JS 文件路径（相对于 `/static/plugins/`），例如 `"reuters_plugin/ui.js"`。
    ///
    pub fn with_ui(mut self, tag_name: &str, js_path: &str) -> Self {
        self.ui_tag_name = Some(tag_name.to_string());
        self.ui_js_path = Some(js_path.to_string());
        self
    }

    /// 返回插件前端 UI 的自定义 HTML 标签名（若有）。
    pub fn ui_tag_name(&self) -> Option<&str> {
        self.ui_tag_name.as_deref()
    }

    /// 返回插件前端 UI 的 JS 文件路径（若有）。
    pub fn ui_js_path(&self) -> Option<&str> {
        self.ui_js_path.as_deref()
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
