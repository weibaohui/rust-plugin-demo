/**
 * 路透社插件 Vue 3 子应用入口（Naive UI 版,qiankun）。
 *
 * vite-plugin-qiankun 框架无关:entry inline script 的 createDeffer 机制
 * 期望 window.moudleQiankunAppLifeCycles[appName],这里手动注入
 * (与 React 版同模式,绕过插件未自动注入的问题)。
 */
import { createApp, type App as VueApp } from 'vue';
import naive from 'naive-ui';
import ReutersPanel from './ReutersPanel.vue';

let app: VueApp | null = null;

function render(): void {
  app = createApp(ReutersPanel);
  app.use(naive);
  app.mount('#sub-app-container');
}

function destroy(): void {
  app?.unmount();
  app = null;
}

export async function bootstrap(): Promise<void> {
  // no-op
}

export async function mount(): Promise<void> {
  render();
}

export async function update(): Promise<void> {
  // Vue 无增量 reconciler,简单 no-op
}

export async function unmount(): Promise<void> {
  destroy();
}

// 独立运行(非 qiankun 环境)
if (!(window as { __POWERED_BY_QIANKUN__?: boolean }).__POWERED_BY_QIANKUN__) {
  render();
}

// 手动注入生命周期到 window.moudleQiankunAppLifeCycles
const QIANKUN_APP_NAME = 'reuters-plugin';
const qiankunWindow = window as unknown as {
  moudleQiankunAppLifeCycles?: Record<string, unknown>;
};
qiankunWindow.moudleQiankunAppLifeCycles = qiankunWindow.moudleQiankunAppLifeCycles ?? {};
qiankunWindow.moudleQiankunAppLifeCycles[QIANKUN_APP_NAME] = { bootstrap, mount, update, unmount };
