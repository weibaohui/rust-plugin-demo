/**
 * data_plugin 主题变量表。
 *
 * 与宿主 `frontend/src/theme/theme-palette.ts` 内容保持一致 — 独立副本避免跨 crate 共享源码。
 * 写入 :root 后 CSS 可通过 `var(--color-bg)` 消费，跨 qiankun experimentalStyleIsolation 自动级联。
 */
export const THEME_PALETTE = {
  dark: {
    '--color-bg': '#000000',
    '--color-text': '#e6e6e6',
    '--color-text-secondary': '#a6a6a6',
    '--color-border': '#303030',
    '--color-primary': '#1677ff',
  },
  light: {
    '--color-bg': '#f5f5f5',
    '--color-text': '#1f1f1f',
    '--color-text-secondary': '#595959',
    '--color-border': '#e5e5e5',
    '--color-primary': '#1677ff',
  },
} as const;

export type ThemeMode = keyof typeof THEME_PALETTE;

/** localStorage 键 — 与宿主共用。 */
export const THEME_STORAGE_KEY = 'plugkit:theme';

/** 将指定模式应用到 :root。 */
export function applyThemePalette(mode: ThemeMode): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.dataset.theme = mode;
  for (const [k, v] of Object.entries(THEME_PALETTE[mode])) {
    root.style.setProperty(k, v);
  }
}