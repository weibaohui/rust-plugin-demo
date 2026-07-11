/** qiankun 微前端运行时 */
export { loadMicroApp, registerMicroApps, start, initGlobalState } from 'qiankun';

/**
 * 计算插件的 qiankun 入口 URL。
 */
export function qiankunEntryFor(plugin: { ui_entry?: string | null }, origin: string): string | null {
  if (!plugin.ui_entry) return null;
  return `${origin}${plugin.ui_entry}`;
}

let started = false;

/**
 * 将当前已加载插件的 qiankun 子应用注册到 qiankun 运行时。
 * 必须在 `#plugin-mount` 容器已渲染到 DOM 后调用。
 */
export async function registerLoadedPlugins(
  plugins: { id: string; name: string; has_ui: boolean; qiankunEntry?: string }[],
  _origin: string,
): Promise<void> {
  const { registerMicroApps, start } = await import('qiankun');

  if (!started) {
    started = true;
    start({ sandbox: { experimentalStyleIsolation: true } });
  }

  const apps = plugins
    .filter(p => p.has_ui && p.qiankunEntry)
    .map(p => ({
      name: p.id,
      entry: p.qiankunEntry!,
      container: '#plugin-mount',
      activeRule: (location: Location) => location.pathname === `/plugin/${encodeURIComponent(p.id)}`,
      props: { pluginName: p.name },
    }));

  if (apps.length === 0) return;

  registerMicroApps(apps, {
    beforeLoad: [async () => console.log('[qiankun] before load', apps.map(a => a.name))],
    afterMount: [async () => console.log('[qiankun] after mount', apps.map(a => a.name))],
  });
}