import { useState, useEffect, useCallback } from 'react';
import type { ReactNode } from 'react';
import './App.css';
import Navbar from './components/Navbar';
import LibraryList from './components/LibraryList';
import PluginList from './components/PluginList';
import PluginUi from './components/PluginUi';
import PublishForm from './components/PublishForm';
import type { PluginInfo, LibraryInfo, ArticleResponse } from './api';
import { scanLibraries, listPlugins, loadLibrary, unloadPlugin, unloadAllPlugins, publishArticle } from './api';

type Tab = 'plugins' | 'libraries' | 'publish' | 'ui';

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
    { key: 'ui', label: '插件界面', icon: '🎨' },
    { key: 'publish', label: '发布新闻', icon: '📰' },
  ];

  return (
    <div className="app-container">
      <Navbar
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={setActiveTab}
        pluginCount={plugins.length}
      />

      <main className="main-content">
        {error && (
          <div className="error-banner" onClick={() => setError(null)}>
            {error}
          </div>
        )}

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
            plugins={plugins}
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
          <PluginUi plugins={plugins} />
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