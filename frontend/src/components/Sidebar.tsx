import { useState } from 'react';
import type { PluginMenu, PluginInfo } from '../api';

interface SidebarProps {
  plugins: PluginInfo[];
  currentPath: string;
  onNavigate: (route: string) => void;
}

/** 宿主固定菜单项（与插件菜单同构，前端硬编码）。 */
const HOST_MENUS: PluginMenu[] = [
  { key: 'plugins', title: '已加载插件', icon: '🔌', route: '/', order: 0, children: [] },
  { key: 'libraries', title: '插件库管理', icon: '📦', route: '/libraries', order: 10, children: [] },
  { key: 'publish', title: '发布新闻', icon: '📰', route: '/publish', order: 20, children: [] },
];

interface MenuItemProps {
  item: PluginMenu;
  currentPath: string;
  onNavigate: (route: string) => void;
  depth: number;
}

/** 递归渲染一个菜单项：有 children 则为可展开分组，否则为可点击叶子。 */
function MenuItem({ item, currentPath, onNavigate, depth }: MenuItemProps): JSX.Element {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = item.children.length > 0;
  const isActive = item.route !== null && currentPath === item.route;
  const paddingLeft = 12 + depth * 16;

  return (
    <li className="menu-item">
      {hasChildren ? (
        <button
          className="menu-btn group"
          style={{ paddingLeft }}
          onClick={() => setExpanded(v => !v)}
          aria-expanded={expanded}
        >
          {item.icon && <span className="menu-icon">{item.icon}</span>}
          <span className="menu-title">{item.title}</span>
          <span className="menu-caret">{expanded ? '▾' : '▸'}</span>
        </button>
      ) : (
        <button
          className={`menu-btn ${isActive ? 'active' : ''}`}
          style={{ paddingLeft }}
          onClick={() => {
            if (item.route) onNavigate(item.route);
          }}
        >
          {item.icon && <span className="menu-icon">{item.icon}</span>}
          <span className="menu-title">{item.title}</span>
        </button>
      )}
      {hasChildren && expanded && (
        <ul className="menu-children">
          {item.children.map(child => (
            <MenuItem
              key={child.key}
              item={child}
              currentPath={currentPath}
              onNavigate={onNavigate}
              depth={depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

/**
 * 左侧菜单：宿主固定菜单 + 已加载插件声明的菜单，合并后按 order 排序、树形渲染。
 * 点击叶子菜单项调用 onNavigate(route) 触发 SPA 路由。
 */
export default function Sidebar({ plugins, currentPath, onNavigate }: SidebarProps): JSX.Element {
  const pluginMenus = plugins.flatMap(p => p.menu);
  const menus = [...HOST_MENUS, ...pluginMenus].sort((a, b) => a.order - b.order);

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <span className="brand-icon">📡</span>
        <span className="brand-text">新闻插件管理系统</span>
        <span className="brand-badge">v0.1</span>
      </div>
      <nav className="sidebar-nav" aria-label="主导航">
        <ul className="menu-list">
          {menus.map(m => (
            <MenuItem
              key={m.key}
              item={m}
              currentPath={currentPath}
              onNavigate={onNavigate}
              depth={0}
            />
          ))}
        </ul>
      </nav>
    </aside>
  );
}
