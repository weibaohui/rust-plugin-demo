/**
 * 主题切换按钮 — 显示 Sun/Moon 图标，置于 Sider 底部 Menu 下方。
 */
import { Button, Tooltip } from 'antd';
import { SunOutlined, MoonOutlined } from '@ant-design/icons';
import { useThemeMode } from '../theme/use-theme-mode';

export function ThemeToggle(): React.ReactElement {
  const { isDark, toggle } = useThemeMode();
  const label = isDark ? '切换到亮色' : '切换到暗色';
  return (
    <Tooltip title={label} placement="right">
      <Button
        type="text"
        block
        icon={isDark ? <SunOutlined /> : <MoonOutlined />}
        onClick={toggle}
        aria-label={label}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-start',
          gap: 8,
          color: 'var(--color-text)',
        }}
      >
        {isDark ? '亮色模式' : '暗色模式'}
      </Button>
    </Tooltip>
  );
}

export default ThemeToggle;