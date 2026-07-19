/**
 * data_plugin 子应用版主题 hook。
 *
 * 三路输入（按优先级）：
 * 1. qiankun `props.themeMode`（宿主广播的当前模式）
 * 2. qiankun `props.onGlobalStateChange`（运行时切换）
 * 3. `localStorage` + `storage` 事件（跨标签同步）
 * 4. `window` 上的 `app:theme` 自定义事件（兜底）
 *
 * 任一来源变化都更新本地 `mode` state，触发 `<AntdThemeProvider>` 重渲染。
 */
import { useEffect, useState } from 'react';
import {
  THEME_STORAGE_KEY,
  applyThemePalette,
  type ThemeMode,
} from './theme-palette';

export interface UseThemeModeOptions {
  initialMode: ThemeMode;
  /** qiankun 注入的 props（含 themeMode 和 onGlobalStateChange） */
  props?: Record<string, unknown>;
}

export function useThemeMode({ initialMode, props }: UseThemeModeOptions): {
  mode: ThemeMode;
  setMode: (m: ThemeMode) => void;
} {
  const [mode, setMode] = useState<ThemeMode>(initialMode);

  // 写入 CSS 变量（即使在 ConfigProvider 之外的组件也能用 var(--color-bg)）
  useEffect(() => {
    applyThemePalette(mode);
  }, [mode]);

  // 多路订阅
  useEffect(() => {
    const onCustom = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      const m = detail?.mode;
      if (m === 'light' || m === 'dark') setMode(m);
    };
    const onStorage = (e: StorageEvent) => {
      if (e.key !== THEME_STORAGE_KEY) return;
      if (e.newValue === 'light' || e.newValue === 'dark') setMode(e.newValue);
    };
    window.addEventListener('app:theme', onCustom);
    window.addEventListener('storage', onStorage);

    // qiankun live updates
    const onGSC = props?.onGlobalStateChange as
      | ((cb: (state: Record<string, unknown>) => void) => void)
      | undefined;
    let unbind: (() => void) | undefined;
    if (onGSC) {
      onGSC((state) => {
        const m = state?.themeMode;
        if (m === 'light' || m === 'dark') setMode(m);
      });
      unbind = () => {
        // qiankun 没有显式 unbind，靠 mount 卸载整体清掉 effect
      };
    }

    return () => {
      window.removeEventListener('app:theme', onCustom);
      window.removeEventListener('storage', onStorage);
      unbind?.();
    };
  }, [props]);

  return { mode, setMode };
}