/**
 * 双主题 CSS 变量映射表。
 *
 * 宿主与 data_plugin 各保留一份相同内容的副本（避免跨 crate 共享源码）。
 * 写入 `document.documentElement.style` 即可让 CSS 通过 `var(--color-bg)` 等消费，
 * 同时也跨 qiankun `experimentalStyleIsolation` 边界自动级联。
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

/** localStorage 键 — 宿主与插件共用。 */
export const THEME_STORAGE_KEY = 'plugkit:theme';

/** 将指定模式应用到 :root（同时设置 dataset 与 CSS 变量）。 */
export function applyThemePalette(mode: ThemeMode): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.dataset.theme = mode;
  for (const [k, v] of Object.entries(THEME_PALETTE[mode])) {
    root.style.setProperty(k, v);
  }
}