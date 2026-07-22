//! 插件元数据声明 — 集中定义名称、图标、菜单、定时任务等。
//!
//! 此文件作为\"配置中心\"，方便统一定义、查看、修改。

use plugkit::metadata::{CronSpec, PluginMenu, PluginMetadata};
use crate::PLUGIN_ID;

/// 构建插件的完整元数据。
pub fn metadata() -> PluginMetadata {
    PluginMetadata::new(PLUGIN_ID, "Data Plugin", env!("CARGO_PKG_VERSION"))
        .with_icon("🗄️")
        .with_description("数据 CRUD 插件 — 演示数据库操作、UI 数据表格、cron 定时任务、创建人/编辑人追踪")
        .with_author("plugkit <plugkit@example.com>")
        .with_license("MIT")
        .with_tables_owned(vec!["data_items".to_string()])
        .with_menus(menus())
}

/// 插件声明的菜单项。
pub fn menus() -> Vec<PluginMenu> {
    vec![PluginMenu {
        key: "data_panel".into(),
        title: "数据管理".into(),
        icon: Some("🗄️".into()),
        route: Some(format!("/plugin/{}", PLUGIN_ID)),
        order: 200,
        children: vec![],
    }]
}

/// 插件声明的定时任务。
pub fn cron_specs() -> Vec<CronSpec> {
    vec![CronSpec {
        name: "generate_data".to_string(),
        interval_secs: 60,
    }]
}
