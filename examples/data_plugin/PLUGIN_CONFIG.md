# 插件配置一致性说明

插件开发中，多个文件中需保持一致的标识符。以下逐一列出**定义位置**与**依赖关系**，修改时务必同步更新。

---

## 1. 插件 ID（核心标识）

**定义位置：** `src/lib.rs`

```rust
pub(crate) const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),   // → data_plugin
    "::",
    module_path!(),            // → data_plugin
    "::",
    "DataPlugin",
);
// 结果: "data_plugin::data_plugin::DataPlugin"
```

**依赖方：**
| 文件 | 用途 |
|------|------|
| `src/lib.rs` | `register_plugins` 中构造 `DataPlugin::new(PLUGIN_ID)` |
| `src/metadata/mod.rs` | 菜单路由 `format!("/plugin/{}", PLUGIN_ID)` |

**前端对应：**
| 文件 | 位置 | 值 |
|------|------|-----|
| `ui/src/main.tsx` | `pluginId` 默认值 | `'data_plugin::data_plugin::DataPlugin'` |
| `ui/src/Panel.tsx` | `pluginId` 参数默认值 | 同上 |

> 修改 PLUGIN_ID 时，前端的 `main.tsx` 和 `Panel.tsx` 中默认值也需同步。

---

## 2. API 路由路径

**定义位置：** `src/routes/mod.rs`

```rust
pub fn routes() -> Vec<PluginRoute> {
    vec![
        route(Method::GET,    "/items", handler::handle_list_items),
        route(Method::POST,   "/items", handler::handle_create_item),
        route(Method::PUT,    "/items", handler::handle_update_item),
        route(Method::DELETE, "/items", handler::handle_delete_item),
    ]
}
```

**实际 URL：** `/plugin-api/<PLUGIN_ID>/items`

**前端对应：** `ui/src/Panel.tsx`
```ts
const apiBase = useMemo(() => `/plugin-api/${pluginId}/items`, [pluginId]);
```

> 路由路径 `/items` 与前端 `apiBase` 中的 `/items` 必须一致。

---

## 3. 数据库表名

**定义位置：** 多处

| 文件 | 位置 | 值 |
|------|------|-----|
| `src/metadata/mod.rs` | `tables_owned` | `vec!["data_items"]` |
| `src/trait_impl.rs` | `on_install` SQL | `CREATE TABLE IF NOT EXISTS data_items (...)` |
| `src/trait_impl.rs` | `on_uninstall` | `db.drop_table("data_items")` |
| `src/model/mod.rs` | `#[sea_orm(table_name = "...")]` | `"data_items"` |
| `src/plugin.rs` | `generate_record` | `SELECT ... FROM data_items` |

> 改表名时，以上 5 处全部需要同步。

---

## 4. 前端 qiankun 应用名

**定义位置：** `ui/vite.config.ts` + `ui/src/main.tsx`

```ts
// vite.config.ts — 静态资源 base 路径
base: '/plugin-files/data_plugin/ui/dist/',

// vite.config.ts — qiankun 插件注册
plugins: [react(), qiankun('data-plugin', ...)],

// main.tsx — 生命周期注入
const QIANKUN_APP_NAME = 'data-plugin';
```

**依赖方：** `src/metadata/mod.rs`
```rust
.with_menus(vec![PluginMenu {
    route: Some(format!("/plugin/{}", PLUGIN_ID)),
    ...
}])
```

> `qiankun('data-plugin', ...)` 中的名称不影响后端路由，但 `vite.config.ts` 的 `base` 路径影响前端资源加载。

---

## 5. 元数据（图标、菜单、作者等）

**定义位置：** `src/metadata/mod.rs`

```rust
pub fn metadata() -> PluginMetadata {
    PluginMetadata::new(PLUGIN_ID, "Data Plugin", env!("CARGO_PKG_VERSION"))
        .with_icon("🗄️")
        .with_description("数据 CRUD 插件")
        .with_author("plugkit <plugkit@example.com>")
        .with_license("MIT")
        .with_tables_owned(vec!["data_items".to_string()])
        .with_menus(menus())
}

pub fn menus() -> Vec<PluginMenu> {
    vec![PluginMenu {
        key: "data_panel".into(),
        title: "数据管理".into(),
        icon: Some("🗄️".into()),
        route: Some(format!("/plugin/{}", PLUGIN_ID)),  // ← 依赖 PLUGIN_ID
        order: 200,
        children: vec![],
    }]
}
```

---

## 6. 版本号

**定义位置：** `Cargo.toml`

```toml
version = "0.1.0"
```

**依赖方：** `src/metadata/mod.rs`
```rust
PluginMetadata::new(..., env!("CARGO_PKG_VERSION"))
```

> 版本号从 `Cargo.toml` 自动读取，无需手动同步。

---

## 快速检查清单

修改功能时，确认以下文件已同步：

- [ ] `src/lib.rs` — PLUGIN_ID
- [ ] `src/metadata/mod.rs` — 表名、菜单、图标
- [ ] `src/routes/mod.rs` — API 路径
- [ ] `src/model/mod.rs` — SeaORM 表名
- [ ] `src/trait_impl.rs` — SQL 建表/删表语句
- [ ] `src/plugin.rs` — SQL 查询语句
- [ ] `ui/src/main.tsx` — 前端 pluginId 默认值
- [ ] `ui/src/Panel.tsx` — API base 路径
- [ ] `ui/vite.config.ts` — base 路径
- [ ] `Cargo.toml` — 版本号
