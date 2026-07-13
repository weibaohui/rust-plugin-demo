/*!
插件脚手架生成器 — `plugkit new <name>` 一条命令生成全套独立插件骨架。

生成的插件：
- 后端：dylib crate，仅依赖 plugkit，实现 Plugin trait + register_plugins
- 前端：React + antd + vite-plugin-qiankun（官方推荐技术栈）
- 构建脚本：Makefile 一键构建前端 + 后端

也提供 `plugkit package <dylib_path> [--ui-dir <path>]` 命令，
将构建好的 dylib 和前端打包成 `.plugin` 单文件，便于分发和上传安装。
*/

use std::fs;
use std::io::{self, Write};
use std::path::Path;
///
/// `plugin_name` 同时用作 crate 名和目录名（仅允许 `[a-z0-9_]`）。
/// 生成失败返回 IO 错误。
pub fn generate_plugin(base_dir: &Path, plugin_name: &str) -> io::Result<()> {
    validate_name(plugin_name)?;

    let plugin_dir = base_dir.join(plugin_name);
    if plugin_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("目录已存在: {}", plugin_dir.display()),
        ));
    }

    fs::create_dir_all(plugin_dir.join("src"))?;
    fs::create_dir_all(plugin_dir.join("ui/src"))?;

    // 后端
    fs::write(plugin_dir.join("Cargo.toml"), cargo_toml(plugin_name))?;
    fs::write(plugin_dir.join("src/lib.rs"), lib_rs(plugin_name))?;
    // 占位 dist 目录，让 include_dir! 在首次 cargo build 时能编译
    fs::create_dir_all(plugin_dir.join("ui/dist"))?;
    fs::write(plugin_dir.join("ui/dist/index.html"), placeholder_html())?;

    // 前端
    fs::write(
        plugin_dir.join("ui/package.json"),
        package_json(plugin_name),
    )?;
    fs::write(plugin_dir.join("ui/tsconfig.json"), tsconfig_root())?;
    fs::write(plugin_dir.join("ui/tsconfig.app.json"), tsconfig_app())?;
    fs::write(
        plugin_dir.join("ui/vite.config.ts"),
        vite_config(plugin_name),
    )?;
    fs::write(plugin_dir.join("ui/index.html"), dev_html(plugin_name))?;
    fs::write(plugin_dir.join("ui/src/main.tsx"), main_tsx(plugin_name))?;
    fs::write(plugin_dir.join("ui/src/Panel.tsx"), panel_tsx(plugin_name))?;

    // 构建脚本
    fs::write(plugin_dir.join("Makefile"), makefile(plugin_name))?;

    println!();
    println!("✓ 插件骨架已生成: {}", plugin_dir.display());
    println!();
    println!("下一步:");
    println!("  cd {}/", plugin_dir.display());
    println!("  make            # 构建前端 + 后端");
    println!(
        "  cp target/debug/lib{}.dylib <宿主>/bin/plugins/   # 安装到宿主插件目录",
        plugin_name
    );
    println!();
    Ok(())
}

fn validate_name(name: &str) -> io::Result<()> {
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "插件名不能为空",
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "插件名仅允许小写字母、数字、下划线",
        ));
    }
    Ok(())
}

fn class_name(plugin_name: &str) -> String {
    // afp_plugin → AfpPlugin
    plugin_name
        .split('_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn qiankun_app_name(plugin_name: &str) -> String {
    // afp_plugin → afp-plugin
    plugin_name.replace('_', "-")
}

fn cargo_toml(name: &str) -> String {
    let t = r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["dylib"]

[dependencies]
plugkit = { version = "0.2", path = "../../../.." }
include_dir = "0.7"
serde = { version = "1", features = ["derive"] }
"#;
    t.replace("{name}", name)
}

fn lib_rs(name: &str) -> String {
    let cls = class_name(name);
    let t = r#"/*!
{name} 插件 — 完全独立，仅依赖 plugkit 框架。

编译期将 ui/dist/ 嵌入到本插件的 dylib 中，宿主可直接从内存服务前端。
*/

use plugkit::database::DatabaseExt;
use plugkit::host::HostContext;
use plugkit::metadata::PluginMetadata;
use plugkit::plugin::{Plugin, PluginRegistrar};
use include_dir::{include_dir, Dir};
use serde::Serialize;
use std::sync::Arc;

// 编译期嵌入的 ui/dist/ 目录
pub static UI_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

/// {cls} 插件实例。
#[derive(Debug)]
pub struct {cls} {
    id: String,
    ui_base_dir: Option<String>,
    ui_dist: Option<&'static Dir<'static>>,
}

impl {cls} {
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
}

impl Plugin for {cls} {
    fn plugin_id(&self) -> &String {
        &self.id
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(&self.id, "{name}", env!("CARGO_PKG_VERSION"))
            .with_icon("🔌")
            .with_description("{name} 插件 — 由 plugkit new 生成")
            .with_author("Your Name <you@example.com>")
            .with_license("MIT")
    }

    fn on_load(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[{name}] loaded");
        Ok(())
    }
    fn on_unload(&self, _db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
        eprintln!("[{name}] unloaded");
        Ok(())
    }
    fn on_enable(&self) -> plugkit::error::Result<()> {
        eprintln!("[{name}] enabled");
        Ok(())
    }
    fn on_disable(&self) -> plugkit::error::Result<()> {
        eprintln!("[{name}] disabled");
        Ok(())
    }
    fn on_start(&self) -> plugkit::error::Result<()> {
        eprintln!("[{name}] started");
        Ok(())
    }
    fn on_stop(&self) -> plugkit::error::Result<()> {
        eprintln!("[{name}] stopped");
        Ok(())
    }

    fn ui_base_dir(&self) -> Option<&str> {
        self.ui_base_dir.as_deref()
    }
    fn has_ui(&self) -> bool {
        self.ui_dist.is_some()
    }
    fn ui_dist(&self) -> Option<&'static Dir<'static>> {
        self.ui_dist
    }
}

// 注册入口（dylib 符号）
#[no_mangle]
pub extern "C" fn register_plugins(registrar: &mut PluginRegistrar) {
    registrar.register(Arc::new(
        {cls}::new(PLUGIN_ID).with_ui_dist("{name}/ui", &UI_DIST),
    ));
}

// 插件标识符：{name}::{name}::{cls}
const PLUGIN_ID: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "::",
    module_path!(),
    "::",
    "{cls}",
);
"#;
    t.replace("{name}", name).replace("{cls}", &cls)
}

fn placeholder_html() -> String {
    r#"<!doctype html><html><head><meta charset="UTF-8"/></head><body><div id="sub-app-container"></div></body></html>"#.to_string()
}

fn dev_html(name: &str) -> String {
    let title = name.replace('_', " ");
    let title = title
        .split(' ')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let t = r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
  </head>
  <body>
    <div id="sub-app-container"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#;
    t.replace("{title}", &title)
}

fn package_json(name: &str) -> String {
    let qk = qiankun_app_name(name);
    let t = r#"{
  "name": "{qk}-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "antd": "^5.22.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "~5.7.0",
    "vite": "^6.0.0",
    "vite-plugin-qiankun": "^1.0.15"
  }
}
"#;
    t.replace("{qk}", &qk)
}

fn tsconfig_root() -> String {
    r#"{
  "files": [],
  "references": [{ "path": "./tsconfig.app.json" }]
}
"#
    .to_string()
}

fn tsconfig_app() -> String {
    r#"{
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.app.tsbuildinfo",
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "preserve",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
"#
    .to_string()
}

fn vite_config(name: &str) -> String {
    let qk = qiankun_app_name(name);
    let t = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import qiankun from 'vite-plugin-qiankun'

export default defineConfig({
  base: '/plugin-files/{name}/ui/dist/',
  plugins: [
    react(),
    qiankun('{qk}', { useDevMode: false }),
  ],
})
"#;
    t.replace("{name}", name).replace("{qk}", &qk)
}

fn main_tsx(name: &str) -> String {
    let qk = qiankun_app_name(name);
    let t = r#"/**
 * {name} 插件 React 子应用入口（qiankun）。
 */
import { StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import Panel from './Panel';

let root: Root | null = null;

function render(props: Record<string, unknown> = {}) {
  const container = document.getElementById('sub-app-container');
  if (!container) return;
  root = createRoot(container);
  root.render(
    <StrictMode>
      <Panel
        pluginId={typeof props.pluginId === 'string' ? (props.pluginId as string) : '{name}'}
      />
    </StrictMode>,
  );
}

function destroy() {
  if (root) {
    root.unmount();
    root = null;
  }
}

export async function bootstrap() {}
export async function mount(props: Record<string, unknown>) { render(props); }
export async function update(props: Record<string, unknown>) { render(props); }
export async function unmount() { destroy(); }

// 独立运行（非 qiankun 环境）
if (!(window as { __POWERED_BY_QIANKUN__?: boolean }).__POWERED_BY_QIANKUN__) {
  render();
}

// 手动注入生命周期到 window.moudleQiankunAppLifeCycles
const QIANKUN_APP_NAME = '{qk}';
const qiankunWindow = window as unknown as {
  moudleQiankunAppLifeCycles?: Record<string, unknown>;
};
qiankunWindow.moudleQiankunAppLifeCycles = qiankunWindow.moudleQiankunAppLifeCycles ?? {};
qiankunWindow.moudleQiankunAppLifeCycles[QIANKUN_APP_NAME] = { bootstrap, mount, update, unmount };
"#;
    t.replace("{name}", name).replace("{qk}", &qk)
}

fn panel_tsx(name: &str) -> String {
    let cls = class_name(name);
    let t = r#"/**
 * {name} 插件 React 面板（Ant Design 版）。
 * 演示控件：Card / Form / Input / Button / Tag
 */
import { useState, type ReactNode } from 'react';
import { Card, Form, Input, Button, Tag, Space, App as AntApp } from 'antd';

interface PanelProps {
  pluginId?: string;
}

function PanelContent({ pluginId = '{name}' }: PanelProps): ReactNode {
  const [note, setNote] = useState('');
  const { message } = AntApp.useApp();

  const handleSave = () => {
    message.success(`备注已保存: ${note}`);
  };

  return (
    <Card title="🔌 {cls} 控制面板" style={{ maxWidth: 720 }}>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Form layout="vertical">
          <Form.Item label="插件 ID">
            <Tag color="blue">{pluginId}</Tag>
          </Form.Item>
          <Form.Item label="备注">
            <Input
              value={note}
              onChange={e => setNote(e.target.value)}
              placeholder="输入备注信息"
              allowClear
            />
          </Form.Item>
          <Button type="primary" onClick={handleSave}>
            💾 保存
          </Button>
        </Form>
      </Space>
    </Card>
  );
}

export function Panel(props: PanelProps): ReactNode {
  return (
    <AntApp>
      <PanelContent {...props} />
    </AntApp>
  );
}

export default Panel;
"#;
    t.replace("{name}", name).replace("{cls}", &cls)
}

fn makefile(name: &str) -> String {
    let t = r#"REPO_ROOT := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

.PHONY: all build build-ui clean help

all: build ## 默认: 构建前端 + 后端

help: ## 列出所有可用目标
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^#^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: build-ui ## 构建前端 + 后端
	@echo "==> 构建后端 dylib"
	@cargo build
	@echo "✓ 构建完成 → target/debug/lib{name}.dylib"

build-ui: ## 构建前端 (ui/dist/)
	@echo "==> 构建前端"
	@cd ui && \
		[ -d node_modules ] || npm install; \
		(node node_modules/typescript/lib/tsc.js -b 2>/dev/null || true); \
		node node_modules/vite/bin/vite.js build
	@echo "✓ 前端构建完成 → ui/dist/"

clean: ## 清理构建产物
  @cargo clean
  @rm -rf ui/dist ui/node_modules
  @echo "✓ 清理完成"
"#;
    t.replace("{name}", name)
}

/// 将构建好的插件 dylib 打包成 `.plugin` 单文件。
///
/// 打包格式为 tar.gz，包含：
/// - `plugin.dylib`（或 `.so` / `.dll`）— 插件动态库
/// - `ui/` — 可选的插件前端目录
///
/// 输出文件为 `<name>.plugin`，可直接在宿主前端上传安装。
///
/// # 参数
///
/// * `dylib_path` — 构建好的 dylib 文件路径
/// * `ui_dir` — 可选的前端 `ui/dist/` 目录路径
/// * `output_path` — 输出 `.plugin` 文件路径
pub fn package_plugin(
    dylib_path: &Path,
    ui_dir: Option<&Path>,
    output_path: &Path,
) -> io::Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs::File;
    use tar::Builder;

    let file = File::create(output_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(encoder);

    // 添加 dylib
    let dylib_name = dylib_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "dylib 路径无效"))?;
    let mut dylib_file = File::open(dylib_path)?;
    tar.append_file(dylib_name, &mut dylib_file)?;

    // 添加 ui/dist/ 目录（若提供）
    if let Some(ui) = ui_dir {
        if ui.exists() {
            tar.append_dir_all("ui", ui)?;
        } else {
            eprintln!("  ⚠️  UI 目录不存在: {:?}，跳过", ui);
        }
    }

    let encoder = tar.into_inner()?;
    encoder.finish()?;

    println!();
    println!("✓ 打包完成: {}", output_path.display());
    println!(
        "  大小: {} bytes",
        output_path.metadata().map(|m| m.len()).unwrap_or(0)
    );
    println!();
    println!(
        "上传到宿主: curl -F \"file=@{}\" http://localhost:3000/api/plugins/install",
        output_path.display()
    );
    println!();

    Ok(())
}
