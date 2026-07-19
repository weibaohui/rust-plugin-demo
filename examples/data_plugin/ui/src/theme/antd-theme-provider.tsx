/**
 * data_plugin 主题包装。
 *
 * 包裹 `<StyleProvider hashPriority="high">` 解决 qiankun experimentalStyleIsolation
 * 下 antd v5 `:where()` 选择器特异性被 out-rank 的问题。
 *
 * 内部 `ConfigProvider`：
 * - algorithm 来自 useThemeMode
 * - cssVar: { key } 暴露 antd 变量到 :root
 * - hashed: false 便于调试
 * - token.colorPrimary 来自主题表，保持双主题一致
 *
 * 同步项目级 CSS 变量到 :root，使组件内普通 div 也能 `var(--color-bg)` 消费。
 */
import type { ReactNode } from 'react';
import { ConfigProvider, theme as antdTheme, App as AntApp } from 'antd';
import { StyleProvider } from '@ant-design/cssinjs';
import { THEME_PALETTE, type ThemeMode } from './theme-palette';

export interface AntdThemeProviderProps {
  mode: ThemeMode;
  children: ReactNode;
}

export function AntdThemeProvider({ mode, children }: AntdThemeProviderProps): ReactNode {
  const palette = THEME_PALETTE[mode];
  return (
    <StyleProvider hashPriority="high">
      <ConfigProvider
        theme={{
          cssVar: { key: mode },
          hashed: false,
          algorithm: mode === 'dark' ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
          token: {
            colorPrimary: palette['--color-primary'],
            borderRadius: 6,
          },
        }}
      >
        <AntApp>{children}</AntApp>
      </ConfigProvider>
    </StyleProvider>
  );
}

export default AntdThemeProvider;