/*!
新闻机构插件演示的共享类型。

定义所有新闻机构插件库将实例化和注册的插件类型 [`NewsAgencyPlugin`]，
以及由 `publish()` 方法产生的 [`NewsArticle`]。
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
/// 代表新闻机构的插件。每个机构库使用自己的风格（路透社、法新社、美联社……）
/// 注册此类型的一个实例。
///
#[derive(Debug)]
pub struct NewsAgencyPlugin {
    id: String,
    agency_name: String,
    format_fn: fn(headline: &str, body: &str) -> NewsArticle,
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
    ///
    pub fn new(
        id: &str,
        agency_name: &str,
        format_fn: fn(headline: &str, body: &str) -> NewsArticle,
    ) -> Self {
        Self {
            id: id.to_string(),
            agency_name: agency_name.to_string(),
            format_fn,
        }
    }

    /// 返回人类可读的机构名称。
    pub fn agency_name(&self) -> &str {
        &self.agency_name
    }

    ///
    /// 发布一篇新闻文章。格式（标题风格、电头、模板文本）
    /// 由创建插件时传入的 `format_fn` 决定。
    ///
    pub fn publish(&self, headline: &str, body: &str) -> NewsArticle {
        let mut article = (self.format_fn)(headline, body);
        article.agency = self.agency_name.clone();
        article
    }
}

// ------------------------------------------------------------------------------------------------
// 预定义的格式化风格函数，供插件库使用
// ------------------------------------------------------------------------------------------------

/// 路透社风格：简洁纪实，"[REUTERS] 标题" 前缀，电头 "LONDON"。
pub fn reuters_format(headline: &str, body: &str) -> NewsArticle {
    NewsArticle {
        headline: format!("[REUTERS] {}", headline),
        body: format!("{} — Reporting by Reuters correspondents.", body),
        dateline: "LONDON".to_string(),
        agency: String::new(), // 由 publish() 填充
    }
}

/// 法新社风格："标题 — AFP"，电头 "PARIS"。
pub fn afp_format(headline: &str, body: &str) -> NewsArticle {
    NewsArticle {
        headline: format!("{} — AFP", headline),
        body: format!("{} [AFP correspondents worldwide]", body),
        dateline: "PARIS".to_string(),
        agency: String::new(),
    }
}

/// 美联社风格："AP News: 标题"，电头 "NEW YORK"。
pub fn ap_format(headline: &str, body: &str) -> NewsArticle {
    NewsArticle {
        headline: format!("AP News: {}", headline),
        body: format!("{} (The Associated Press)", body),
        dateline: "NEW YORK".to_string(),
        agency: String::new(),
    }
}

/// 塔斯社风格："标题 — TASS"，电头 "MOSCOW"。
pub fn tass_format(headline: &str, body: &str) -> NewsArticle {
    NewsArticle {
        headline: format!("{} — TASS", headline),
        body: format!("{}, as reported by TASS.", body),
        dateline: "MOSCOW".to_string(),
        agency: String::new(),
    }
}
