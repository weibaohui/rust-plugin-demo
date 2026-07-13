/**
 * data_plugin 插件 React 子应用入口（qiankun）。
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
        pluginId={typeof props.pluginId === 'string' ? (props.pluginId as string) : 'data_plugin.DataPlugin'}
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
const QIANKUN_APP_NAME = 'data-plugin';
const qiankunWindow = window as unknown as {
  moudleQiankunAppLifeCycles?: Record<string, unknown>;
};
qiankunWindow.moudleQiankunAppLifeCycles = qiankunWindow.moudleQiankunAppLifeCycles ?? {};
qiankunWindow.moudleQiankunAppLifeCycles[QIANKUN_APP_NAME] = { bootstrap, mount, update, unmount };
