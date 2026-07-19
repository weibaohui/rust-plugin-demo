/**
 * 宿主主题 Context + Hook。
 *
 * 三层职责：
 * 1. `useState` 作为单一可信源（在 Provider 内）
 * 2. `localStorage` 持久化（含 `storage` 事件跨标签同步）
 * 3. 调用 `setGlobalTheme` 广播到 qiankun 子应用 + 派发 `app:theme` 自定义事件
 *
 * CSS 变量由 `applyThemePalette()` 写入 `document.documentElement.style`，
 * 跨 `experimentalStyleIsolation` 自动级联。
 *
 * 用 Context 而非 hook 内部的 useState，确保 App 与 ThemeToggle 共享同一份状态。
 */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import {
  THEME_STORAGE_KEY,
  applyThemePalette,
  type ThemeMode,
} from './theme-palette';
import { setGlobalTheme } from '../micro';

function readInitial(): ThemeMode {
  if (typeof window === 'undefined') return 'dark';
  const saved = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (saved === 'light' || saved === 'dark') return saved;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export interface UseThemeModeResult {
  mode: ThemeMode;
  isDark: boolean;
  toggle: () => void;
  setMode: (m: ThemeMode) => void;
}

const ThemeModeContext = createContext<UseThemeModeResult | null>(null);

export function ThemeModeProvider({ children }: { children: ReactNode }): ReactNode {
  const [mode, setModeState] = useState<ThemeMode>(readInitial);

  // 应用主题：写 CSS 变量、持久化、广播
  useEffect(() => {
    applyThemePalette(mode);
    window.localStorage.setItem(THEME_STORAGE_KEY, mode);
    setGlobalTheme(mode);
  }, [mode]);

  // 跨标签同步
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key !== THEME_STORAGE_KEY) return;
      if (e.newValue === 'light' || e.newValue === 'dark') setModeState(e.newValue);
    };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  }, []);

  const toggle = useCallback(() => {
    setModeState(prev => (prev === 'dark' ? 'light' : 'dark'));
  }, []);

  const value = useMemo<UseThemeModeResult>(
    () => ({ mode, isDark: mode === 'dark', toggle, setMode: setModeState }),
    [mode, toggle],
  );

  return <ThemeModeContext.Provider value={value}>{children}</ThemeModeContext.Provider>;
}

export function useThemeMode(): UseThemeModeResult {
  const ctx = useContext(ThemeModeContext);
  if (!ctx) throw new Error('useThemeMode must be used inside <ThemeModeProvider>');
  return ctx;
}