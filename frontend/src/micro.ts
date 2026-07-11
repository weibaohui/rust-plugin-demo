/**
 * 动态注册已加载的 qiankun 微应用子前端。
 *
 * 每个有 UI 的插件编译产物位于 `<plugin_dir>/ui/dist/index.html`，
 * 由宿主的 `/plugin-files/*` 路径服务。
 *
 * activeRule 形如 `/plugin/<id>`：前端使用该路径触发相应子应用挂载。
 */
import type { PluginInfo } from './api';

interface QiankunAppEntry {
  name: string;
  entry: string;
  container: string;
  activeRule: string;
}

/**
 * 返回插件 qiankun 子应用的绝对入口 URL。
 * 入口相对路径由 server 通过 plugin.ui_entry 提供（如
 * "/plugin-files/afp_plugin/ui/dist/index.html"），这里仅拼上 origin。
 * 返回 undefined 表示该插件无可挂载的 UI。
 */
export function qiankunEntryFor(plugin: PluginInfo, origin: string): string | undefined {
  if (!plugin.has_ui || !plugin.ui_entry) return undefined;
  return `${origin}${plugin.ui_entry}`;
}

/**
 * 把已加载的插件数组转换为 qiankun apps 列表（滤掉无 UI 或缺路径的项）。
 */
export function appsForPlugins(plugins: PluginInfo[], origin: string): QiankunAppEntry[] {
  const apps: QiankunAppEntry[] = [];
  for (const p of plugins) {
    const entry = qiankunEntryFor(p, origin);
    if (!entry) continue;
    apps.push({
      name: p.id,
      entry,
      container: '#plugin-mount',
      activeRule: `/plugin/${p.id}`,
    });
  }
  return apps;
}

/**
 * 调用 qiankun.registerMicroApps() 和 start()。
 * 用动态 import 避免在 SSR / 测试环境强制加载 qiankun。
 */
export async function registerLoadedPlugins(plugins: PluginInfo[], origin: string): Promise<void> {
  const apps = appsForPlugins(plugins, origin);
  if (apps.length === 0) return;

  const { registerMicroApps, start } = await import('qiankun');

  registerMicroApps(apps, {
    beforeLoad: [
      async (app: { name: string }) => {
        // eslint-disable-next-line no-console
        console.log('[qiankun] before-load', app.name);
        return Promise.resolve();
      },
    ],
  });

  start({ sandbox: { experimentalStyleIsolation: true } });
}

export type { QiankunAppEntry };
