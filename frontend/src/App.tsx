import { useState, useEffect, useCallback, useMemo } from 'react';
import type { ReactNode } from 'react';
import './App.css';
import Sidebar from './components/Sidebar';
import LibraryList from './components/LibraryList';
import PluginList from './components/PluginList';
import PluginUi from './components/PluginUi';
import type { PluginInfo, LibraryInfo } from './api';
import { scanLibraries, listPlugins, loadLibrary, unloadPlugin, unloadAllPlugins, enablePlugin, disablePlugin, startPlugin, stopPlugin } from './api';
import { registerLoadedPlugins } from './micro';

/**
 * 极简客户端路由，基于 window.location.pathname。
 *   /                → 已加载插件列表
 *   /libraries       → 插件库管理
 *   /plugin/<id>     → 插件 qiankun 子应用
 */
function useRoute(): string {
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

function navigate(to: string): void {
  if (typeof window === 'undefined') return;
  if (window.location.pathname !== to) {
    window.history.pushState({}, '', to);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }
}

export default function App(): ReactNode {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [libraries, setLibraries] = useState<LibraryInfo[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const addLog = useCallback((msg: string) => {
    setLogs(prev => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev].slice(0, 100));
  }, []);

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

  // 插件列表变化时注册 qiankun 子应用
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const origin = window.location.origin;
    registerLoadedPlugins(plugins, origin).catch(err => {
      console.error('qiankun registerLoadedPlugins failed', err);
    });
  }, [plugins]);

  const handleLoad = async (name: string) => {
    try {
      setError(null);
      addLog(`正在加载插件库: ${name}...`);
      const newPlugins = await loadLibrary(name);
      addLog(`✅ 加载成功，新增 ${newPlugins.length} 个插件`);
      const list = await refreshPlugins();
      await refreshLibraries();
      navigate('/');
      return list;
    } catch (e) {
      const msg = `❌ 加载失败: ${e}`;
      addLog(msg);
      setError(msg);
      return [];
    }
  };

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

  const handleEnable = async (id: string) => {
    try {
      setError(null);
      addLog(`正在启用: ${id}...`);
      await enablePlugin(id);
      addLog(`✅ 已启用: ${id}`);
      await refreshPlugins();
    } catch (e) {
      const msg = `❌ 启用失败: ${e}`;
      addLog(msg);
      setError(msg);
    }
  };

  const handleDisable = async (id: string) => {
    try {
      setError(null);
      addLog(`正在禁用: ${id}...`);
      await disablePlugin(id);
      addLog(`✅ 已禁用: ${id}`);
      await refreshPlugins();
    } catch (e) {
      const msg = `❌ 禁用失败: ${e}`;
      addLog(msg);
      setError(msg);
    }
  };

  const handleStart = async (id: string) => {
    try {
      setError(null);
      addLog(`正在启动: ${id}...`);
      await startPlugin(id);
      addLog(`✅ 已启动: ${id}(cron 已注册)`);
      await refreshPlugins();
    } catch (e) {
      const msg = `❌ 启动失败: ${e}`;
      addLog(msg);
      setError(msg);
    }
  };

  const handleStop = async (id: string) => {
    try {
      setError(null);
      addLog(`正在停止: ${id}...`);
      await stopPlugin(id);
      addLog(`✅ 已停止: ${id}`);
      await refreshPlugins();
    } catch (e) {
      const msg = `❌ 停止失败: ${e}`;
      addLog(msg);
      setError(msg);
    }
  };

  const routePath = useRoute();
  const pluginRouteMatch = useMemo(() => {
    const m = /^\/plugin\/(.+)$/.exec(routePath);
    return m ? decodeURIComponent(m[1]) : null;
  }, [routePath]);
  const routedPlugin = useMemo(() => {
    if (!pluginRouteMatch) return null;
    return plugins.find(p => p.id === pluginRouteMatch) ?? null;
  }, [pluginRouteMatch, plugins]);

  let content: ReactNode;
  if (pluginRouteMatch) {
    content = routedPlugin ? (
      <PluginUi plugin={routedPlugin} />
    ) : (
      <div className="empty-state">
        <div className="empty-icon">🚫</div>
        <h3>插件未加载</h3>
        <p>找不到 id 为 <code>{pluginRouteMatch}</code> 的插件，可能尚未加载或已被卸载。</p>
      </div>
    );
  } else if (routePath === '/libraries') {
    content = <LibraryList libraries={libraries} onLoad={handleLoad} onRefresh={refreshLibraries} />;
  } else {
    content = (
      <PluginList
        plugins={plugins}
        onUnload={handleUnload}
        onUnloadAll={handleUnloadAll}
        onRefresh={refreshPlugins}
        onEnable={handleEnable}
        onDisable={handleDisable}
        onStart={handleStart}
        onStop={handleStop}
      />
    );
  }

  return (
    <div className="app-container">
      <Sidebar plugins={plugins} currentPath={routePath} onNavigate={navigate} />
      <div className="app-body">
        <main className="main-content">
          {error && (
            <div className="error-banner" onClick={() => setError(null)}>
              {error}
            </div>
          )}
          {content}
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
    </div>
  );
}