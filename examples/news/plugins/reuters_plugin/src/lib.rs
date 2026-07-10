/*!
路透社新闻机构插件 — 完全独立，仅依赖 `plugkit` 框架。

不依赖任何其他插件或宿主 crate。自己的类型自己定义。
编译期将 `ui/dist/` 嵌入到本插件的 `.so`/`.dylib` 中，使宿主可直接从内存服务前端。
*/

use plugkit::database::DatabaseExt;
use plugkit::host::HostContext;
use plugkit::metadata::{CronSpec, PluginMenu, PluginMetadata};
use plugkit::plugin::{Plugin, PluginRegistrar};
use include_dir::{include_dir, Dir};
use serde::Serialize;
use std::sync::Arc;

// ------------------------------------------------------------------------------------------------
// 编译期嵌入的 `ui/dist/` 目录
// ------------------------------------------------------------------------------------------------

pub static UI_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

// ------------------------------------------------------------------------------------------------
// Reuters 自己的类型（不与他人共享）
// ------------------------------------------------------------------------------------------------

/// 由 Reuters 插件 publish 产生的新闻文章。
#[derive(Debug, Clone, Serialize)]
struct NewsArticle {
    headline: String,
    body: String,
    dateline: String,
    agency: String,
}

/// Reuters 新闻机构插件实例。
#[derive(Debug)]
pub struct ReutersPlugin {
    id: String,
    ui_base_dir: Option<String>,
    ui_dist: Option<&'static Dir<'static>>,
}

impl ReutersPlugin {
    const AGENCY: &'static str = "Reuters";

    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            ui_base_dir: None,
            ui_dist: None,
        }
    }

    fn with_ui_dist(mut self, base_dir: &str, dist: &'static Dir<'static>) -> Self {
        self.ui_base_dir = Some(base_dir.to_string());
        self.ui_dist = Some(dist);
        self
    }

    /// 发布一篇 Reuters 风格新闻。
    fn publish(&self, ctx: &dyn HostContext, headline: &str, body: &str) -> NewsArticle {
        ctx.log_message(&format!("Reuters formatting news: {}", headline));
        NewsArticle {
            headline: format!("[REUTERS] {}", headline),
            body: format!(
                "{} — Reporting by Reuters correspondents.\n\n---\n[Host: {} v{} | {} | {} plugins loaded]",
                body,
                ctx.server_name(),
                ctx.server_version(),
                ctx.server_time(),
                ctx.plugin_count(),
            ),
            dateline: "LONDON".to_string(),
            agency: Self::AGENCY.to_string(),
        }
    }
}

impl Plugin for ReutersPlugin {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(&self.id, Self::AGENCY, env!("CARGO_PKG_VERSION"))
            .with_icon("📰")
            .with_description("路透社新闻机构插件,Reuters 格式化风格")
            .with_author("AtomGit <noreply@atomgit.com>")
            .with_homepage("https://github.com/weibaohui/rust-plugin-demo")
            .with_license("MIT")
            .with_tables_owned(vec!["reuters_items".to_string()])
            .with_menus(vec![PluginMenu {
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
            }])
    }

    fn on_load(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[reuters] loaded");
        Ok(())
    }
    fn on_unload(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[reuters] unloaded");
        Ok(())
    }
    fn on_install(&self, db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        db.validate_table_name("reuters_items")?;
        db.execute("CREATE TABLE IF NOT EXISTS reuters_items (id INTEGER PRIMARY KEY, headline TEXT, body TEXT)")?;
        Ok(())
    }
    fn on_uninstall(&self, db: &dyn DatabaseExt, keep_data: bool) -> plugkit::error::Result<()> {
        if !keep_data {
            db.drop_table("reuters_items")?;
        }
        Ok(())
    }
    fn on_enable(&self) -> plugkit::error::Result<()> {
        eprintln!("[reuters] enabled: 初始化资源,菜单可见");
        Ok(())
    }
    fn on_disable(&self) -> plugkit::error::Result<()> {
        eprintln!("[reuters] disabled: 菜单隐藏,收敛能力");
        Ok(())
    }
    fn on_start(&self) -> plugkit::error::Result<()> {
        eprintln!("[reuters] started: 后台任务就绪,heartbeat 30s");
        Ok(())
    }
    fn on_stop(&self) -> plugkit::error::Result<()> {
        eprintln!("[reuters] stopped: 后台任务停止");
        Ok(())
    }
    fn on_cron(&self, name: &str) -> plugkit::error::Result<()> {
        eprintln!("[reuters] cron tick: {}", name);
        Ok(())
    }
    fn cron_specs(&self) -> Vec<CronSpec> {
        vec![CronSpec { name: "heartbeat".to_string(), interval_secs: 30 }]
    }
    fn ui_base_dir(&self) -> Option<&str> {
        self.ui_base_dir.as_deref()
    }
    fn has_ui(&self) -> bool {
        self.ui_dist.is_some()
    }
}

// ------------------------------------------------------------------------------------------------
// 注册入口（dylib 符号）
// ------------------------------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar) {
    registrar.register(Arc::new(
        ReutersPlugin::new(PLUGIN_ID).with_ui_dist("reuters_plugin/ui", &UI_DIST),
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
    "ReutersAgency"
);