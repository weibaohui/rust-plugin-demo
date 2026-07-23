/**
 * data_plugin 插件 React 子应用入口（qiankun）。
 *
 * - 独立运行时（vite dev / preview / 直接打开 dist/index.html）：
 *   把 <Panel/> 渲染到 #sub-app-container，便于本地调试。
 * - 在 qiankun 容器中运行时：导出 bootstrap/mount/update/unmount 生命周期。
 *
 * 主题接入：
 * - props.themeMode 由宿主在挂载时通过 qiankun 注入
 * - 运行时通过 props.onGlobalStateChange 接收宿主切换
 * - 同时监听 localStorage 'storage' 事件与 window 'app:theme' 自定义事件（兜底）
 */
import { StrictMode } from 'react';
import type { ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import Panel from './Panel';
import { AntdThemeProvider } from './theme/antd-theme-provider';
import { useThemeMode } from './theme/use-theme-mode';
import { THEME_STORAGE_KEY, type ThemeMode } from './theme/theme-palette';

let root: Root | null = null;

function Shell({ initialMode, props, children }: {
  initialMode: ThemeMode;
  props: Record<string, unknown>;
  children: ReactNode;
}): ReactNode {
  const { mode } = useThemeMode({ initialMode, props });
  // force re-render to keep AntdThemeProvider in sync — useThemeMode returns
  // { mode, setMode }, but AntdThemeProvider needs mode as prop.
  return <AntdThemeProvider mode={mode}>{children}</AntdThemeProvider>;
}

function render(props: Record<string, unknown> = {}) {
  // 从 qiankun props 读取 token 并暴露到全局变量（Panel 通过 window.__plugkit_token__ 读取）
  if (typeof props.token === 'string') {
    (window as any).__plugkit_token__= [redacted]
  }

  const container = document.getElementById('sub-app-container');
  if (!container) return;

  const stored = (typeof window !== 'undefined'
    ? window.localStorage.getItem(THEME_STORAGE_KEY)
    : null) as ThemeMode | null;
  const initialMode: ThemeMode =
    (typeof props.themeMode === 'string' && (props.themeMode === 'light' || props.themeMode === 'dark')
      ? props.themeMode
      : (stored === 'light' || stored === 'dark' ? stored : 'dark'));

  const pluginId =
    typeof props.pluginId === 'string' ? props.pluginId : 'data_plugin.DataPlugin';
  const token =
    typeof props.token === 'string' ? props.token : undefined;
  const user =
    typeof props.user === 'object' && props.user !== null
      ? props.user as { username?: string }
      : undefined;

  root = createRoot(container);
  root.render(
    <StrictMode>
      <Shell initialMode={initialMode} props={props}>
        <Panel pluginId={pluginId} token={token} user={user} />
      </Shell>
    </StrictMode>,
  );
}

function destroy() {
  if (root) {
    root.unmount();
    root = null;
  }
}

// qiankun lifecycle exports
export async function bootstrap() {
  // no-op
}

export async function mount(props: Record<string, unknown>) {
  render(props);
}

export async function update(props: Record<string, unknown>) {
  render(props);
}

export async function unmount() {
  destroy();
}

// Standalone mode (not inside qiankun)
if (!(window as { __POWERED_BY_QIANKUN__?: boolean }).__POWERED_BY_QIANKUN__) {
  render();
}

// vite-plugin-qiankun 1.0.15 + vite 6 未自动把生命周期注入到 window.moudleQiankunAppLifeCycles，
// 手动注入以匹配 entry HTML inline script 的期望（见 dist/index.html 的 createDeffer 机制）。
const QIANKUN_APP_NAME = 'data-plugin';
const qiankunWindow = window as unknown as {
  moudleQiankunAppLifeCycles?: Record<string, unknown>;
};
qiankunWindow.moudleQiankunAppLifeCycles = qiankunWindow.moudleQiankunAppLifeCycles ?? {};
qiankunWindow.moudleQiankunAppLifeCycles[QIANKUN_APP_NAME] = { bootstrap, mount, update, unmount };