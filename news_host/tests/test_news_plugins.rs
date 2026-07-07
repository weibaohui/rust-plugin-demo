use dygpi::manager::{PluginManager, PLATFORM_DYLIB_EXTENSION, PLATFORM_DYLIB_PREFIX};
use news_api::NewsAgencyPlugin;
use std::path::PathBuf;
use std::sync::Arc;

// ------------------------------------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------------------------------------

fn dylib_name(base: &str) -> PathBuf {
    let name = PathBuf::from(format!(
        "{}{}.{}",
        PLATFORM_DYLIB_PREFIX, base, PLATFORM_DYLIB_EXTENSION
    ));
    println!("  🔧 构建动态库文件名 -> {:?}", name);
    name
}

fn print_article(plugin_name: &str, article: &news_api::NewsArticle) {
    println!("    ┌──────────────────────────────────────────────────────────────");
    println!("    │  📰 新闻内容");
    println!("    │  机构 : {}", plugin_name);
    println!("    │  地点 : {}", article.dateline);
    println!("    │  标题 : {} ", article.headline);
    println!("    │  正文 : {}", article.body);
    println!("    └──────────────────────────────────────────────────────────────");
}

fn init_logger() {
    // 初始化日志系统，让 dygpi 框架内部的 info!/trace! 日志也能显示出来
    let _ = pretty_env_logger::try_init();
}

// ------------------------------------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------------------------------------

#[test]
fn test_reuters_plugin() {
    init_logger();
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  测试1：单独加载路透社插件                                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // 第1步：创建 PluginManager
    println!("📌 [第1步] 创建 PluginManager<NewsAgencyPlugin>");
    println!("    PluginManager 是泛型管理器，T = NewsAgencyPlugin");
    println!("    这意味着这个管理器只能管理「新闻机构」类型的插件");
    let mut manager: PluginManager<NewsAgencyPlugin> = PluginManager::default();
    println!("    ✅ PluginManager 创建完成");
    println!();

    // 第2步：加载动态库
    let lib_name = dylib_name("reuters_plugin");
    println!("📌 [第2步] 调用 load_plugins_from() 加载动态库");
    println!("    👉 框架会做以下事情：");
    println!("        ① 打开动态库文件  {:?}", lib_name);
    println!("        ② 检查版本兼容性（对比 dygpi 库版本 + rustc 版本的哈希值）");
    println!("        ③ 在库中查找符号 \"register_plugins\"");
    println!("        ④ 调用该函数，传入 PluginRegistrar");
    println!("        ⑤ 插件库内部调用 registrar.register() 注册插件实例");
    println!("        ⑥ 对每个注册的插件调用 on_load() 生命周期方法");
    println!("        ⑦ 把插件存入管理器的内部注册表");
    println!();

    manager
        .load_plugins_from(&lib_name)
        .expect("❌ 加载 reuters_plugin 动态库失败");

    println!("    ✅ 加载成功！");
    println!();

    // 第3步：检查管理器状态
    println!("📌 [第3步] 检查管理器状态");
    println!("    插件数量: {}", manager.len());
    println!(
        "    是否包含路透社? {}",
        manager.contains("reuters_plugin::reuters_plugin::ReutersAgency")
    );
    assert_eq!(manager.len(), 1);
    assert!(manager.contains("reuters_plugin::reuters_plugin::ReutersAgency"));
    println!();

    // 第4步：按 ID 获取插件
    println!("📌 [第4步] 通过 get(\"reuters_plugin::reuters_plugin::ReutersAgency\") 获取插件");
    println!("    插件 ID 的格式是: 包名::模块路径::插件名");
    println!("    这个 ID 在 reuters_plugin/src/lib.rs 中定义");
    println!();

    let plugin: Arc<NewsAgencyPlugin> = manager
        .get("reuters_plugin::reuters_plugin::ReutersAgency")
        .expect("❌ 按 ID 查找插件失败");

    println!("    ✅ 获取到插件实例");
    println!("    机构名称: {}", plugin.agency_name());
    assert_eq!(plugin.agency_name(), "Reuters");
    println!();

    // 第5步：发布新闻
    println!("📌 [第5步] 调用 plugin.publish() 发布新闻");
    println!("    👉 publish() 内部会调用该插件创建时传入的 format_fn");
    println!("    👉 路透社的 format_fn = reuters_format()");
    println!("    👉 该函数会输出 \"[REUTERS] 标题\" 格式，dateline 设为 LONDON");
    println!();

    let article = plugin.publish(
        "Global Markets Rally on Tech Earnings",
        "Stock markets worldwide surged following strong quarterly earnings from major technology companies.",
    );

    print_article("Reuters", &article);

    assert!(article.headline.starts_with("[REUTERS]"));
    assert_eq!(article.dateline, "LONDON");
    println!("    ✅ 断言通过：headline 以 [REUTERS] 开头，dateline = LONDON");
    println!();
}

#[test]
fn test_afp_plugin() {
    init_logger();
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  测试2：单独加载法新社插件                                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    println!("📌 [第1步] 创建 PluginManager");
    let mut manager: PluginManager<NewsAgencyPlugin> = PluginManager::default();
    println!("    ✅ 创建完成");
    println!();

    let lib_name = dylib_name("afp_plugin");
    println!("📌 [第2步] 加载 afp_plugin 动态库");
    println!("    👉 流程和刚才一样，但这次加载的是不同的 .dylib 文件");
    println!("    👉 AFP 的注册函数内部会创建 NewsAgencyPlugin，");
    println!("        传入的 format_fn = afp_format()");
    println!();

    manager
        .load_plugins_from(&lib_name)
        .expect("❌ 加载 afp_plugin 动态库失败");

    println!("    ✅ 加载成功！");
    println!("    插件数量: {}", manager.len());
    assert_eq!(manager.len(), 1);
    println!();

    println!("📌 [第3步] 获取 AFP 插件实例");
    let plugin: Arc<NewsAgencyPlugin> = manager
        .get("afp_plugin::afp_plugin::AfpAgency")
        .expect("❌ 按 ID 查找插件失败");

    println!("    ✅ 获取到插件，机构名称: {}", plugin.agency_name());
    assert_eq!(plugin.agency_name(), "Agence France-Presse");
    println!();

    println!("📌 [第4步] 发布新闻");
    println!("    👉 AFP 的 format_fn = afp_format()");
    println!("    👉 输出 \"标题 — AFP\" 格式，dateline 设为 PARIS");
    println!();

    let article = plugin.publish(
        "Climate Summit Reaches Historic Agreement",
        "World leaders have committed to binding emissions targets for the first time.",
    );

    print_article("AFP", &article);

    assert!(article.headline.ends_with("— AFP"));
    assert_eq!(article.dateline, "PARIS");
    println!("    ✅ 断言通过：headline 以 — AFP 结尾，dateline = PARIS");
    println!();
}

#[test]
fn test_load_both_plugins() {
    init_logger();
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  测试3：同时加载路透社 + 法新社，对比同一新闻的不同风格           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    println!("📌 [第1步] 创建 PluginManager（同一个管理器）");
    let mut manager: PluginManager<NewsAgencyPlugin> = PluginManager::default();
    println!("    ✅ 创建完成");
    println!();

    println!("📌 [第2步] 第一次加载 —— 加载 reuters_plugin");
    println!("    👉 加载后，管理器内部注册表多了 1 个插件");
    manager
        .load_plugins_from(&dylib_name("reuters_plugin"))
        .expect("❌ 加载 reuters_plugin 失败");
    println!("    ✅ 当前插件数量: {}", manager.len());
    println!();

    println!("📌 [第3步] 第二次加载 —— 加载 afp_plugin（同一个管理器！）");
    println!("    👉 再加载一个不同的 .dylib，管理器里会多 1 个插件");
    println!("    👉 现在管理器里同时有 Reuters 和 AFP 两个插件实例");
    manager
        .load_plugins_from(&dylib_name("afp_plugin"))
        .expect("❌ 加载 afp_plugin 失败");
    println!(
        "    ✅ 当前插件数量: {}（两个机构的插件都在里面了）",
        manager.len()
    );
    assert_eq!(manager.len(), 2);
    println!();

    println!("📌 [第4步] 获取所有插件，用同一篇新闻发稿，看风格差异");
    println!("    👉 plugins() 返回 Vec<Arc<T>>，每个元素是一个插件实例");
    println!("    👉 对每个插件调用 publish()，同一个新闻内容");
    println!("    👉 但因为 format_fn 不同，输出格式完全不同！");
    println!();

    let all_plugins = manager.plugins();
    for (i, plugin) in all_plugins.iter().enumerate() {
        println!("    ─── 插件 #{} ({}) ───", i + 1, plugin.agency_name());
        println!();

        let article = plugin.publish(
            "Central Bank Holds Interest Rates Steady",
            "The central bank maintained its benchmark interest rate at 4.5% amid mixed economic signals.",
        );

        print_article(plugin.agency_name(), &article);
    }

    // 验证两个机构都在
    let names: Vec<String> = all_plugins
        .iter()
        .map(|p| p.agency_name().to_string())
        .collect();
    assert!(names.contains(&"Reuters".to_string()));
    assert!(names.contains(&"Agence France-Presse".to_string()));
    println!("    ✅ 两个机构都找到了");
    println!();
}

#[test]
fn test_custom_registration_fn() {
    init_logger();
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  测试4：自定义注册函数名 — 演示错误处理                           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    println!("📌 [第1步] 创建 PluginManager");
    let mut manager: PluginManager<NewsAgencyPlugin> = PluginManager::default();
    println!();
    println!("📌 [第2步] 设置注册函数名为 \"register_other_plugins\"");
    println!("    👉 默认情况下，框架会在动态库中查找名为 \"register_plugins\" 的符号");
    println!("    👉 set_registration_fn_name() 可以改成其他名字");
    println!("    👉 这里我们改成 \"register_other_plugins\"");
    println!("    👉 但 reuters_plugin 只定义了 \"register_plugins\"，没有这个函数");
    println!();

    manager.set_registration_fn_name(b"register_other_plugins\0");

    println!("📌 [第3步] 尝试加载 reuters_plugin 动态库");
    println!("    👉 框架会去 .dylib 里找 \"register_other_plugins\" 符号");
    println!("    👉 找不到 → 返回 SymbolNotFound 错误");
    println!();

    let result = manager.load_plugins_from(&dylib_name("reuters_plugin"));
    assert!(result.is_err());
    let err = format!("{:?}", result.err().unwrap());
    println!("    ❌ 错误信息: {}", err);
    assert!(err.starts_with("Error(SymbolNotFound"));
    println!();
    println!("    ✅ 符合预期：框架正确地报告了符号未找到的错误");
    println!("    ✅ 说明自定义注册函数名机制在工作");
    println!();
}
