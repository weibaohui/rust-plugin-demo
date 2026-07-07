import type { ReactNode } from 'react';

interface Tab { key: string; label: string; icon: ReactNode }

interface NavbarProps {
  tabs: Tab[];
  activeTab: string;
  onTabChange: (key: string) => void;
  pluginCount: number;
}

export default function Navbar({ tabs, activeTab, onTabChange, pluginCount }: NavbarProps) {
  return (
    <header className="navbar">
      <div className="navbar-brand">
        <span className="brand-icon">📡</span>
        <span className="brand-text">新闻插件管理系统</span>
        <span className="brand-badge">v0.1</span>
      </div>
      <nav className="navbar-tabs">
        {tabs.map(tab => (
          <button
            key={tab.key}
            className={`tab-btn ${activeTab === tab.key ? 'active' : ''}`}
            onClick={() => onTabChange(tab.key)}
          >
            {tab.icon} {tab.label}
            {tab.key === 'plugins' && pluginCount > 0 && (
              <span className="badge">{pluginCount}</span>
            )}
          </button>
        ))}
      </nav>
    </header>
  );
}