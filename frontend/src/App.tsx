import { useState, useEffect, useCallback, useMemo } from 'react';
import type { ReactNode } from 'react';
import './App.css';
import Navbar from './components/Navbar';
import LibraryList from './components/LibraryList';
import PluginList from './components/PluginList';
import PluginUi from './components/PluginUi';
import PublishForm from './components/PublishForm';
import type { PluginInfo, LibraryInfo, ArticleResponse } from './api';
import { scanLibraries, listPlugins, loadLibrary, unloadPlugin, unloadAllPlugins, publishArticle } from './api';
import { registerLoadedPlugins, qiankunEntryFor } from './micro';

type Tab = 'plugins' | 'libraries' | 'publish' | 'ui';

/**
 * Very small client-side router keyed off `window.location.pathname`.
 * Supports:
 *   /                → default tab
 *   /plugin/<id>     → render <PluginUi> for the matching loaded plugin
 */
function useRoute() {
  const [path, setPath] = useState<string>(() =>
    typeof window === 'undefined' ? '/' : window.location.pathname || '/',
  );
  useEffect(() => {
    const onPop = () => setPath(window.location.pathname || '/');
    window.addEventListener('popstate', onPop);
    return () => window.removeEventListener('popstate', onPop);
  }, []);
  return path;
}

function navigate(to: string) {
  if (typeof window === 'undefined') return;
  if (window.location.pathname !== to) {
    window.history.pushState({}, '', to);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }
}

export default function App() {
  const [activeTab, setActiveTab] = useState<Tab>('plugins');
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [libraries, setLibraries] = useState<LibraryInfo[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const addLog = useCallback((msg: string) => {
    setLogs(prev => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev].slice(0, 100));
  }, []);

  // 加载插件列表
  const refreshPlugins = useCallback(async () => {
    try {
      const list = await listPlugins();
      setPlugins(list);
      return list;
    } catch (e) {
      setError(`获取插件列表失败: ${e}`);
      return [];
    }
  }, []);

  // 扫描插件库
  const refreshLibraries = useCallback(async () => {
    try {
      const libs = await scanLibraries();
      setLibraries(libs);
      return libs;
    } catch (e) {
      setError(`扫描插件库失败: ${e}`);
      return [];
    }
  }, []);

  useEffect(() => {
    refreshPlugins();
    refreshLibraries();
  }, [refreshPlugins, refreshLibraries]);

  // 一旦插件列表发生变化，为每个有 UI 的插件填充 qiankunEntry，
  // 然后调用 registerLoadedPlugins()（仅执行一次，后续重复调用会被 qiankun 去重）。
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const origin = window.location.origin;
    const enriched = plugins.map(p => ({
      ...p,
      qiankunEntry: qiankunEntryFor(p, origin),
    }));
    registerLoadedPlugins(enriched, origin).catch(err => {
      // eslint-disable-next-line no-console
      console.error('qiankun registerLoadedPlugins failed', err);
    });
  }, [plugins]);

  // 加载插件库
  const handleLoad = async (name: string) => {
    try {
      setError(null);
      addLog(`正在加载插件库: ${name}...`);
      const newPlugins = await loadLibrary(name);
      addLog(`✅ 加载成功，新增 ${newPlugins.length} 个插件`);
      const list = await refreshPlugins();
      await refreshLibraries();
      setActiveTab('plugins');
      return list;
    } catch (e) {
      const msg = `❌ 加载失败: ${e}`;
      addLog(msg);
      setError(msg);
      return [];
    }
  };

  // 卸载插件
  const handleUnload = async (id: string) => {
    try {
      setError(null);
      addLog(`正在卸载插件: ${id}...`);
      await unloadPlugin(id);
      addLog(`✅ 已卸载: ${id}`);
      await refreshPlugins();
      await refreshLibraries();
    } catch (e) {
      const msg = `❌ 卸载失败: ${e}`;
      addLog(msg);
      setError(msg);
    }
  };

  // 卸载全部
  const handleUnloadAll = async () => {
    try {
      setError(null);
      addLog('正在卸载所有插件...');
      await unloadAllPlugins();
      addLog('✅ 所有插件已卸载');
      await refreshPlugins();
      await refreshLibraries();
    } catch (e) {
      const msg = `❌ 卸载失败: ${e}`;
      addLog(msg);
      setError(msg);
    }
  };

  // 发布新闻
  const handlePublish = async (pluginId: string, headline: string, body: string): Promise<ArticleResponse> => {
    setError(null);
    addLog(`📝 正在调用 "${pluginId}" 发布新闻...`);
    try {
      const article = await publishArticle(pluginId, { headline, body });
      addLog(`✅ 发布成功！(${article.agency})`);
      return article;
    } catch (e) {
      const msg = `❌ 发布失败: ${e}`;
      addLog(msg);
      setError(msg);
      throw e;
    }
  };

  const tabs: { key: Tab; label: string; icon: ReactNode }[] = [
    { key: 'plugins', label: '已加载插件', icon: '🔌' },
    { key: 'libraries', label: '插件库管理', icon: '📦' },
    { key: 'publish', label: '发布新闻', icon: '📰' },
    { key: 'ui', label: '插件界面', icon: '🎨' },
  ];

  // ────────── 路由：/plugin/<id> → qiankun 子应用容器 ──────────
  const routePath = useRoute();
  const pluginRouteMatch = useMemo(() => {
    const m = /^\/plugin\/(.+)$/.exec(routePath);
    return m ? decodeURIComponent(m[1]) : null;
  }, [routePath]);

  const routedPlugin = useMemo(() => {
    if (!pluginRouteMatch) return null;
    return plugins.find(p => p.id === pluginRouteMatch) ?? null;
  }, [pluginRouteMatch, plugins]);

  const handleTabChange = useCallback((key: string) => {
    setActiveTab(key as Tab);
    navigate('/');
  }, []);

  return (
    <div className="app-container">
      <Navbar
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={handleTabChange}
        pluginCount={plugins.length}
      />

      <main className="main-content">
        {error && (
          <div className="error-banner" onClick={() => setError(null)}>
            {error}
          </div>
        )}

        {pluginRouteMatch ? (
          <div className="plugin-route">
            <div className="plugin-route-header">
              <button className="back-btn" onClick={() => navigate('/')}>
                ← 返回
              </button>
              <span className="route-path">/plugin/{pluginRouteMatch}</span>
            </div>
            {routedPlugin ? (
              <PluginUi plugin={routedPlugin} />
            ) : (
              <div className="empty-state">
                <div className="empty-icon">🚫</div>
                <h3>插件未加载</h3>
                <p>找不到 id 为 <code>{pluginRouteMatch}</code> 的插件，可能尚未加载或已被卸载。</p>
              </div>
            )}
          </div>
        ) : (
          <>
            {activeTab === 'plugins' && (
              <PluginList
                plugins={plugins}
                onUnload={handleUnload}
                onUnloadAll={handleUnloadAll}
                onRefresh={refreshPlugins}
              />
            )}

            {activeTab === 'libraries' && (
              <LibraryList
                libraries={libraries}
                onLoad={handleLoad}
                onRefresh={refreshLibraries}
              />
            )}

            {activeTab === 'publish' && (
              <PublishForm
                plugins={plugins}
                onPublish={handlePublish}
                onRefreshPlugins={refreshPlugins}
              />
            )}

            {activeTab === 'ui' && (
              <div className="ui-tab-placeholder">
                <div className="empty-state">
                  <div className="empty-icon">🎨</div>
                  <h3>请直接访问 <code>/plugin/&lt;插件ID&gt;</code> 查看子应用</h3>
                  <p>已加载的插件 ID 列表：</p>
                  <ul className="ui-plugin-list">
                    {plugins.filter(p => p.has_ui).map(p => (
                      <li key={p.id}>
                        <code>{p.id}</code> — {p.agency}
                      </li>
                    ))}
                    {plugins.filter(p => p.has_ui).length === 0 && <li>（暂无 UI 插件）</li>}
                  </ul>
                </div>
              </div>
            )}
          </>
        )}
      </main>

      <footer className="log-footer">
        <div className="log-header">📋 操作日志</div>
        <div className="log-entries">
          {logs.length === 0 && <div className="log-empty">暂无操作，加载插件后日志会显示在这里</div>}
          {logs.map((log, i) => (
            <div key={i} className="log-entry">{log}</div>
          ))}
        </div>
      </footer>
    </div>
  );
}
