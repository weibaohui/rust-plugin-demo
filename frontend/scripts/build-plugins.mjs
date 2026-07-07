/**
 * 编译插件 UI TypeScript 源文件为浏览器可用的 ESM JavaScript。
 *
 * 用法: node scripts/build-plugins.mjs
 *
 * 输入: reuters_plugin/ui/panel.tsx, afp_plugin/ui/panel.tsx 等
 * 输出: 同目录的 .js 文件（被后端 /plugin-files/* 路由服务）
 */
import * as esbuild from 'esbuild';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..', '..');

// 插件 UI 入口列表（相对于项目根）
const entries = [
  'reuters_plugin/ui/panel.tsx',
  'afp_plugin/ui/panel.tsx',
];

async function main() {
  for (const entry of entries) {
    const src = resolve(root, entry);
    const out = resolve(root, entry.replace(/\.tsx$/, '.js'));

    await esbuild.build({
      entryPoints: [src],
      outfile: out,
      bundle: true,
      format: 'esm',
      platform: 'browser',
      // 插件依赖由宿主注入 (React, createRoot)，不打包
      external: ['react', 'react-dom', 'react-dom/client'],
      sourcemap: false,
    });

    console.log(`✓ ${entry} → ${entry.replace(/\.tsx$/, '.js')}`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});