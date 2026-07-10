/*!
插件元信息(metadata)声明。

每个插件实现都**应当**返回一个 [`PluginMetadata`] 实例,它参考 npm 的 `package.json`
与 k8m 的 `Meta/Module` 结构,用于在主框架内**统一规约**地进行:

* 发现(discovery)——按名称/作者/主页识别插件
* 显示(display)——前端列表渲染标题/图标/说明/版本
* 依赖检测(dependency check)——`dependencies`/`run_after` 声明
* 启动顺序(ordered start)——拓扑排序
* 卸载清理(uninstall)——`tables` 列表配合数据库接口

# 示例

```ignore
use plugkit::metadata::{PluginMetadata, PluginMenu};

static META: PluginMetadata = PluginMetadata::new(
    "afp_plugin",
    "Agence France-Presse",
    env!("CARGO_PKG_VERSION"),
)
.with_description("法新社新闻机构插件,AFP 格式化风格")
.with_author("AtomGit <noreply@atomgit.com>")
.with_homepage("https://github.com/weibaohui/rust-plugin-demo")
.with_license("MIT")
.with_icon("📡")
.with_tables(&["afp_items"])
.with_dependencies(&[])
.with_run_after(&[])
.with_menus(vec![
    PluginMenu {
        key: "afp".into(),
        title: "法新社".into(),
        icon: Some("📡".into()),
        route: None,
        order: 100,
        children: vec![],
    },
]);
```

*/

// ------------------------------------------------------------------------------------------------
// 模块依赖
// ------------------------------------------------------------------------------------------------

#[cfg(feature = "config_serde")]
use serde::Serialize;

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 插件元信息,参考 npm `package.json` + k8m `Meta/Module`。
///
/// 所有字段在编译期由插件开发者声明,运行期不可变;宿主据此进行发现、显示、依赖检测、
/// 启动顺序排序、卸载清理。
///
/// **规约**:
/// * `name` —— 系统级唯一标识(必填),与 `plugin_id` 的最后一段建议一致
/// * `version` —— 语义化版本号(必填),用于触发 `on_upgrade`
/// * `description` / `author` / `homepage` / `license` —— 展示与归档信息
/// * `icon` —— emoji 或 CSS class,前端菜单/列表渲染
/// * `dependencies` —— 强依赖,**启用前**必须确保都已启用
/// * `run_after` —— 启动顺序约束,非依赖,但必须在它们之后启动
/// * `tables` —— 插件使用的数据库表名列表,卸载时据此清理(配合 `keep_data` 选项)
/// * `menus` —— 菜单树,宿主聚合后交前端 Sidebar 渲染
/// * `crons` —— 定时任务规格(秒级间隔),宿主在 `on_start` 后据此调度
///
#[derive(Debug, Clone)]
#[cfg_attr(feature = "config_serde", derive(Serialize))]
pub struct PluginMetadata {
    /// 插件唯一标识(系统级唯一)。**必填**。
    pub name: String,
    /// 人类可读的展示名称。**必填**。
    pub title: String,
    /// 语义化版本号(如 `"1.0.0"`)。**必填**。版本变化触发 `on_upgrade`。
    pub version: String,
    /// 功能描述(可选)。
    pub description: Option<String>,
    /// 作者(可选),格式如 `"Name <email>"`。
    pub author: Option<String>,
    /// 主页 URL(可选)。
    pub homepage: Option<String>,
    /// 许可证(可选),如 `"MIT"` / `"Apache-2.0"`。
    pub license: Option<String>,
    /// 图标(可选):emoji 或 CSS class,如 `"📡"` 或 `"fa-solid fa-cube"`。
    pub icon: Option<String>,
    /// 强依赖插件名列表;启用前必须确保都已启用。
    dependencies: Vec<String>,
    /// 启动顺序约束;非依赖,但必须在它们之后启动。
    run_after: Vec<String>,
    /// 插件使用的数据库表名列表,卸载时据此清理。
    tables: Vec<String>,
    /// 菜单树,宿主聚合后交前端渲染。
    menus: Vec<PluginMenu>,
    /// 定时任务规格(秒级间隔),宿主在 `on_start` 后据此调度。
    crons: Vec<CronSpec>,
}

///
/// 菜单项,参考 k8m `Menu`。宿主聚合所有已启用插件的菜单后交前端 Sidebar 渲染。
///
/// 菜单内容静态,**可见性**由插件是否处于 `Enabled`/`Running` 状态决定。
///
#[derive(Debug, Clone)]
#[cfg_attr(feature = "config_serde", derive(Serialize))]
pub struct PluginMenu {
    /// 菜单唯一标识(插件内唯一)。
    pub key: String,
    /// 展示标题。
    pub title: String,
    /// 图标(emoji 或 CSS class),可选。
    pub icon: Option<String>,
    /// 点击跳转的路由;`None` 表示纯分组节点(仅展开子菜单)。
    pub route: Option<String>,
    /// 排序权重,越小越靠前。
    pub order: i32,
    /// 子菜单(树形)。
    pub children: Vec<PluginMenu>,
}

///
/// 定时任务规格。宿主在 `on_start` 后据此调度,`on_stop` 时注销。
///
/// **注意**:plugkit 框架自身不依赖 async,定时调度由宿主实现(如 `tokio::spawn` sleep loop)。
/// 框架仅负责传递规格与触发 `on_cron`。
///
#[derive(Debug, Clone)]
#[cfg_attr(feature = "config_serde", derive(Serialize))]
pub struct CronSpec {
    /// 任务名(插件内唯一)。
    pub name: String,
    /// 执行间隔(秒)。
    pub interval_secs: u64,
}

// ------------------------------------------------------------------------------------------------
// 实现
// ------------------------------------------------------------------------------------------------

impl PluginMetadata {
    ///
    /// 创建一个新的插件元信息。**必填**字段为 `name` / `title` / `version`。
    ///
    pub fn new(name: &str, title: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            title: title.to_string(),
            version: version.to_string(),
            description: None,
            author: None,
            homepage: None,
            license: None,
            icon: None,
            dependencies: Vec::new(),
            run_after: Vec::new(),
            tables: Vec::new(),
            menus: Vec::new(),
            crons: Vec::new(),
        }
    }

    /// 设置功能描述(builder 风格,可链式)。
    pub fn with_description(mut self, v: &str) -> Self {
        self.description = Some(v.to_string());
        self
    }

    /// 设置作者(builder 风格,可链式)。
    pub fn with_author(mut self, v: &str) -> Self {
        self.author = Some(v.to_string());
        self
    }

    /// 设置主页 URL(builder 风格,可链式)。
    pub fn with_homepage(mut self, v: &str) -> Self {
        self.homepage = Some(v.to_string());
        self
    }

    /// 设置许可证(builder 风格,可链式)。
    pub fn with_license(mut self, v: &str) -> Self {
        self.license = Some(v.to_string());
        self
    }

    /// 设置图标(builder 风格,可链式)。
    pub fn with_icon(mut self, v: &str) -> Self {
        self.icon = Some(v.to_string());
        self
    }

    /// 设置强依赖插件名列表(builder 风格,可链式)。
    pub fn with_dependencies(mut self, deps: &[&str]) -> Self {
        self.dependencies = deps.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置启动顺序约束(builder 风格,可链式)。
    pub fn with_run_after(mut self, deps: &[&str]) -> Self {
        self.run_after = deps.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置插件使用的数据库表名列表(builder 风格,可链式)。
    pub fn with_tables(mut self, tables: &[&str]) -> Self {
        self.tables = tables.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 设置菜单树(builder 风格,可链式)。
    pub fn with_menus(mut self, menus: Vec<PluginMenu>) -> Self {
        self.menus = menus;
        self
    }

    /// 设置定时任务规格(builder 风格,可链式)。
    pub fn with_crons(mut self, crons: Vec<CronSpec>) -> Self {
        self.crons = crons;
        self
    }

    /// 设置强依赖插件名列表(接受 `Vec<String>` 的 builder 风格)。
    pub fn with_dependencies_owned(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// 设置启动顺序约束(接受 `Vec<String>` 的 builder 风格)。
    pub fn with_run_after_owned(mut self, deps: Vec<String>) -> Self {
        self.run_after = deps;
        self
    }

    /// 设置插件使用的数据库表名列表(接受 `Vec<String>` 的 builder �风格)。
    pub fn with_tables_owned(mut self, tables: Vec<String>) -> Self {
        self.tables = tables;
        self
    }

    /// 返回强依赖插件名列表。
    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    /// 返回启动顺序约束列表。
    pub fn run_after(&self) -> &[String] {
        &self.run_after
    }

    /// 返回插件使用的数据库表名列表。
    pub fn tables(&self) -> &[String] {
        &self.tables
    }

    /// 返回菜单树。
    pub fn menus(&self) -> &[PluginMenu] {
        &self.menus
    }

    /// 返回定时任务规格。
    pub fn crons(&self) -> &[CronSpec] {
        &self.crons
    }
}

impl CronSpec {
    /// 创建一个新的定时任务规格。
    pub fn new(name: &str, interval_secs: u64) -> Self {
        Self {
            name: name.to_string(),
            interval_secs,
        }
    }
}

// ------------------------------------------------------------------------------------------------
// 单元测试
// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_builder() {
        let meta = PluginMetadata::new("demo", "演示插件", "1.0.0")
            .with_description("测试")
            .with_author("Atom")
            .with_homepage("https://example.com")
            .with_license("MIT")
            .with_icon("📦")
            .with_dependencies(&["leader"])
            .with_run_after(&["gateway"])
            .with_tables(&["demo_items"])
            .with_menus(vec![PluginMenu {
                key: "demo".into(),
                title: "演示".into(),
                icon: None,
                route: None,
                order: 0,
                children: vec![],
            }])
            .with_crons(vec![CronSpec::new("heartbeat", 30)]);

        assert_eq!(meta.name, "demo");
        assert_eq!(meta.title, "演示插件");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.description.as_deref(), Some("测试"));
        assert_eq!(meta.dependencies(), &["leader".to_string()]);
        assert_eq!(meta.run_after(), &["gateway".to_string()]);
        assert_eq!(meta.tables(), &["demo_items".to_string()]);
        assert_eq!(meta.menus().len(), 1);
        assert_eq!(meta.crons().len(), 1);
        assert_eq!(meta.crons()[0].interval_secs, 30);
    }
}
