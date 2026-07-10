# 插件生命周期状态机 + 全套钩子(参考 k8m)

## Context
当前 dygpi 只有 `on_load`/`on_unload` 2 个钩子,无状态机、无启用/禁用分离、无后台任务、无定时任务、无菜单可见性联动。参考 k8m 补全:**状态机(Loaded→Enabled→Running)+ 钩子(on_install/on_uninstall/on_upgrade/on_enable/on_disable/on_start/on_stop/on_cron)+ 菜单可见性(仅 Enabled/Running)+ 前端状态徽标 + 操作按钮**。

状态机流转:
```
load → Loaded(on_load + on_install)
  → enable → Enabled(on_enable,菜单可见)
    → start → Running(on_start + cron 注册)
      → stop → Enabled(on_stop + cron 注销)
    → disable → Loaded(on_disable,菜单消失)
  → unload → Unloaded(on_uninstall + on_unload + 关库)
```
任何状态 unload 前自动收敛:Running 先 stop,Enabled 先 disable。

## 1. dygpi 框架(`src/plugin.rs` + `src/manager.rs`)
### Plugin trait 扩展(钩子默认 no-op,不破坏现有实现)
- 保留 `on_load`/`on_unload`(改为默认 `Ok(())`,现有插件实现仍覆盖)
- 新增默认 no-op 钩子:`on_install`(首次安装,数据初始化,幂等)/ `on_uninstall`(卸载清理)/ `on_upgrade(&self, from_version: &str)`(版本迁移,MVP 定义不触发)/ `on_enable`(启用)/ `on_disable`(禁用)/ `on_start`(启动后台)/ `on_stop`(停止后台)/ `on_cron(&self, name: &str)`(定时任务执行)
- 新增 `cron_specs() -> Vec<CronSpec>`(声明定时任务,默认空)

### `PluginStatus` enum + `LoadedPlugin.status`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum PluginStatus { Loaded, Enabled, Running }
```
`LoadedPlugin` 加 `status: PluginStatus` 字段。

### `PluginManager` 状态机方法
- `enable_plugin(id)`: Loaded→Enabled,调 `on_enable`
- `start_plugin(id)`: Enabled→Running,调 `on_start`,返回 `Vec<CronSpec>` 供宿主调度
- `stop_plugin(id)`: Running→Enabled,调 `on_stop`
- `disable_plugin(id)`: Enabled→Loaded,调 `on_disable`
- `unload_plugin`: 任何→Unloaded,先 `on_stop` if Running、`on_disable` if Enabled、`on_uninstall`、`on_unload`,关库
- `load_plugins_from`: 加载后 `status=Loaded`,调 `on_load` + `on_install`
- `plugin_status(id) -> Option<PluginStatus>` accessor

### `CronSpec`(dygpi 定义,框架不依赖 async)
```rust
#[derive(Debug, Clone, Serialize)]
pub struct CronSpec { pub name: String, pub interval_secs: u64 }
```

## 2. news_api
`NewsAgencyPlugin` 继承新钩子(默认 no-op)。无需强制实现。

## 3. afp/reuters 插件(`*/src/lib.rs`)
演示实现:`on_enable`/`on_disable`/`on_start`/`on_stop` 打 `log::info!`;`cron_specs()` 返回一个演示任务(如 `CronSpec { name: "heartbeat", interval_secs: 30 }`);`on_cron` 打日志。体现状态机 + cron。

## 4. news_server(`src/main.rs`)
- `PluginInfo` 加 `status: PluginStatus`
- 新 API 端点(POST):`/api/plugins/:id/enable`、`/start`、`/stop`、`/disable`
- **cron 调度**:`AppContext` 加 `cron_handles: HashMap<String, Vec<CancellationToken>>`;`start` 时为每个 `CronSpec` `tokio::spawn`(sleep loop 调 `on_cron`);`stop` 时 `cancel`
- **菜单可见性**:`plugin_to_info` 仅当 `status==Enabled||Running` 时返回 `menu`(否则 `menu: vec![]`)——Sidebar 菜单随状态显隐
- `list_plugins` 返回 `status`

## 5. 前端
- `api.ts`:`PluginInfo` 加 `status: PluginStatus`;加 `enablePlugin/startPlugin/stopPlugin/disablePlugin` 函数
- `PluginList.tsx`:卡片加状态徽标(Loaded 灰/Enabled 蓝/Running 绿)+ 按钮(根据状态显隐:Loaded→启用,Enabled→启动/禁用,Running→停止)
- Sidebar 菜单可见性靠后端过滤(menu=[] 不渲染),前端无需改

## 关键文件
- `src/plugin.rs`(Plugin trait + CronSpec + PluginStatus)
- `src/manager.rs`(PluginManager 状态机 + enable/start/stop/disable + LoadedPlugin.status)
- `news_api/src/lib.rs`(继承默认钩子)
- `plugins/afp_plugin/src/lib.rs` + `plugins/reuters_plugin/src/lib.rs`(演示钩子 + cron_specs)
- `news_server/src/main.rs`(API + cron 调度 + 菜单可见性 + AppContext cron handles)
- `frontend/src/api.ts`(status + 操作 API)
- `frontend/src/components/PluginList.tsx`(状态徽标 + 按钮)

## 验证
1. `cargo build --release` + `cargo test` + `tsc`
2. 启动 server + vite,加载插件(→Loaded,菜单不显示)
3. API `enable`(→Enabled,菜单出现)→ `start`(→Running,cron 日志周期输出)→ `stop`(→Enabled)→ `disable`(→Loaded,菜单消失)
4. playwright:PluginList 卡片显示状态徽标 + 按钮;Sidebar 菜单随状态显隐
5. cron:Running 时 server 日志周期性 `[Plugin Log] cron heartbeat`
