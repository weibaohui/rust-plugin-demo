import { useState } from 'react';
import type { ReactNode } from 'react';
import {
  Plug,
  FolderArchive,
  ChevronDown,
  ChevronRight,
  Puzzle,
  Blocks,
} from 'lucide-react';
import type { PluginMenu, PluginInfo } from '../api';

interface SidebarProps {
  plugins: PluginInfo[];
  currentPath: string;
  onNavigate: (route: string) => void;
}

/** 宿主固定菜单项(与插件菜单同构)。 */
const HOST_MENUS: PluginMenu[] = [
  { key: 'plugins', title: '已加载插件', icon: null, route: '/', order: 0, children: [] },
  { key: 'libraries', title: '插件库管理', icon: null, route: '/libraries', order: 10, children: [] },
];

/** 宿主菜单 key → lucide 图标。 */
const HOST_ICONS: Record<string, ReactNode> = {
  plugins: <Plug size={16} />,
  libraries: <FolderArchive size={16} />,
};

interface MenuItemProps {
  item: PluginMenu;
  currentPath: string;
  onNavigate: (route: string) => void;
  depth: number;
}

/** 递归渲染一个菜单项。 */
function MenuItem({ item, currentPath, onNavigate, depth }: MenuItemProps): ReactNode {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = item.children.length > 0;
  const isActive = item.route !== null && currentPath === item.route;
  const paddingLeft = 10 + depth * 16;
  const icon = HOST_ICONS[item.key] ?? (item.icon ? <span className="menu-icon-emoji">{item.icon}</span> : null);

  return (
    <li className="menu-item">
      {hasChildren ? (
        <button
          className="menu-btn group"
          style={{ paddingLeft }}
          onClick={() => setExpanded(v => !v)}
          aria-expanded={expanded}
        >
          {icon}
          <span className="menu-title">{item.title}</span>
          {expanded ? <ChevronDown size={14} className="menu-caret" /> : <ChevronRight size={14} className="menu-caret" />}
        </button>
      ) : (
        <button
          className={`menu-btn ${isActive ? 'active' : ''}`}
          style={{ paddingLeft }}
          onClick={() => {
            if (item.route) onNavigate(item.route);
          }}
        >
          {icon}
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
 * 左侧菜单:宿主固定菜单 + 已加载插件声明的菜单,合并后按 order 排序、树形渲染。
 */
export default function Sidebar({ plugins, currentPath, onNavigate }: SidebarProps): ReactNode {
  const pluginMenus = plugins.flatMap(p => p.menu);
  const menus = [...HOST_MENUS, ...pluginMenus].sort((a, b) => a.order - b.order);

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <Blocks size={20} className="brand-icon" />
        <span className="brand-text">plugkit</span>
        <span className="brand-badge">v0.2</span>
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