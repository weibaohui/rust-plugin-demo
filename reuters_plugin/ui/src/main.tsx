/**
 * Reuters plugin sub-app entry (qiankun).
 *
 * - 独立运行时（vite dev / preview / 直接打开 dist/index.html）：
 *   把 <ReutersPanel/> 渲染到 #sub-app-container，便于本地调试。
 * - 在 qiankun 容器中运行时：导出 bootstrap/mount/update/unmount 生命周期。
 */
import { StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import ReutersPanel from './ReutersPanel';

let root: Root | null = null;

function render(props: Record<string, unknown> = {}) {
  const container = document.getElementById('sub-app-container');
  if (!container) return;
  root = createRoot(container);
  root.render(
    <StrictMode>
      <ReutersPanel
        pluginId={typeof props.pluginId === 'string' ? (props.pluginId as string) : 'reuters_plugin'}
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
const QIANKUN_APP_NAME = 'reuters-plugin';
const qiankunWindow = window as unknown as {
  moudleQiankunAppLifeCycles?: Record<string, unknown>;
};
qiankunWindow.moudleQiankunAppLifeCycles = qiankunWindow.moudleQiankunAppLifeCycles ?? {};
qiankunWindow.moudleQiankunAppLifeCycles[QIANKUN_APP_NAME] = { bootstrap, mount, update, unmount };
