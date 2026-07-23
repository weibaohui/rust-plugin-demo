/**
 * 动态注册已加载的 qiankun 微应用子前端。
 *
 * 每个有 UI 的插件编译产物位于 `<plugin_dir>/ui/dist/index.html`，
 * 由宿主的 `/plugin-files/*` 路径服务。
 *
 * activeRule 形如 `/plugin/<id>`：前端使用该路径触发相应子应用挂载。
 *
 * 主题通过 `initGlobalState` + `setGlobalState` 广播给子应用，
 * 子应用通过 `props.onGlobalStateChange` 订阅实时变化。
 */
import type { PluginInfo } from './api';
import type { ThemeMode } from './theme/theme-palette';
import { getToken, getUser } from './auth';

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
    // 仅已启用/运行中的插件注册为子应用
    if (p.status !== 'Enabled' && p.status !== 'Running') continue;
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

// ----------------------------------------------------------------------------
// 主题广播
// ----------------------------------------------------------------------------

/**
 * 主题广播全局状态 — 由 `useThemeMode` hook 调用。
 *
 * 工作机制：
 * 1. 写 `currentTheme`，供后续注册子应用时作为 `props.themeMode` 种子
 * 2. 若 `initGlobalState` 已初始化，调用 `setGlobalState` 广播到所有子应用
 * 3. 派发 `app:theme` 自定义事件（兜底，兼容子应用在非 qiankun 环境下运行）
 */
let themeActions: ReturnType<typeof import('qiankun').initGlobalState> | null = null;
let currentTheme: ThemeMode = 'dark';

export function setGlobalTheme(mode: ThemeMode): void {
  currentTheme = mode;
  themeActions?.setGlobalState({ themeMode: mode });
  window.dispatchEvent(new CustomEvent('app:theme', {
    detail: { mode },
    bubbles: true,
    composed: true,
  }));
}

export function getCurrentTheme(): ThemeMode {
  return currentTheme;
}

/**
 * 调用 qiankun.registerMicroApps() 和 start()。
 * 用动态 import 避免在 SSR / 测试环境强制加载 qiankun。
 */
export async function registerLoadedPlugins(plugins: PluginInfo[], origin: string): Promise<void> {
  const apps = appsForPlugins(plugins, origin);
  if (apps.length === 0) return;

  const qiankun = await import('qiankun');

  themeActions = qiankun.initGlobalState({ themeMode: currentTheme });

  // 子应用通过 window.__plugkit_token__ 获取 token（兼容 qiankun 沙箱）
  const t = getToken();
  if (t) { (window as any).__plugkit_token__ = t; }

  qiankun.registerMicroApps(apps, {
    props: {
      themeMode: currentTheme,
      token: getToken(),
      user: getUser(),
    },
    beforeLoad: [
      async (app: { name: string }) => {
        // eslint-disable-next-line no-console
        console.log('[qiankun] before-load', app.name);
        return Promise.resolve();
      },
    ],
  });

  qiankun.start({ sandbox: { experimentalStyleIsolation: true } });
}

export type { QiankunAppEntry };