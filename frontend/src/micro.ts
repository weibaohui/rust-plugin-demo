/** qiankun 微前端运行时 */
export { loadMicroApp, registerMicroApps, start, initGlobalState } from 'qiankun';

/**
 * 计算插件的 qiankun 入口 URL。
 * 对于 /plugin-files/afp_plugin/ui/dist/index.html → http://localhost:3000/plugin-files/afp_plugin/ui/dist/index.html
 */
export function qiankunEntryFor(plugin: { ui_entry?: string | null }, origin: string): string | null {
  if (!plugin.ui_entry) return null;
  return `${origin}${plugin.ui_entry}`;
}

/**
 * 将当前已加载插件的 qiankun 子应用注册到 qiankun 运行时。
 * 由 App 在插件列表变化时调用；qiankun 内部去重，可重复调用。
 */
export async function registerLoadedPlugins(
  plugins: { id: string; name: string; has_ui: boolean; qiankunEntry?: string }[],
  _origin: string,
): Promise<void> {
  const { registerMicroApps, start } = await import('qiankun');

  const apps = plugins
    .filter(p => p.has_ui && p.qiankunEntry)
    .map(p => ({
      name: p.id,
      entry: p.qiankunEntry!,
      container: '#plugin-mount',
      activeRule: (location: Location) => location.pathname.startsWith(`/plugin/${encodeURIComponent(p.id)}`),
      props: { pluginName: p.name },
    }));

  if (apps.length === 0) return;

  registerMicroApps(apps, {
    beforeLoad: [async () => console.log('[qiankun] before load', apps.map(a => a.name))],
    afterMount: [async () => console.log('[qiankun] after mount', apps.map(a => a.name))],
  });

  if (!(window as any).__QIANKUN_STARTED__) {
    (window as any).__QIANKUN_STARTED__ = true;
    start({ sandbox: { experimentalStyleIsolation: true } });
  }
}