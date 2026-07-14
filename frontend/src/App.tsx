import { useState, useEffect, useCallback, useMemo } from 'react';
import type { ReactNode } from 'react';
import { ConfigProvider, Layout, theme, Menu, Card, Button, Tag, Popconfirm, Space, Typography, Alert, App as AntApp } from 'antd';
import {
  ApiOutlined, FolderOpenOutlined, DownloadOutlined, ReloadOutlined,
  DeleteOutlined, PlayCircleOutlined, PauseCircleOutlined,
  CaretUpOutlined, CaretDownOutlined
} from '@ant-design/icons';
import type { PluginInfo, LibraryInfo, PluginMenu, PluginStatus } from './api';
import { scanLibraries, listPlugins, loadLibrary, unloadPlugin, unloadAllPlugins, enablePlugin, disablePlugin, startPlugin, stopPlugin } from './api';
import { registerLoadedPlugins, qiankunEntryFor } from './micro';

const { Sider, Content, Footer } = Layout;
const { Text, Title } = Typography;

function useRoute(): string {
  const [path, setPath] = useState(() => window.location.pathname || '/');
  useEffect(() => {
    const onPop = () => setPath(window.location.pathname || '/');
    window.addEventListener('popstate', onPop);
    return () => window.removeEventListener('popstate', onPop);
  }, []);
  return path;
}

function navigate(to: string): void {
  if (window.location.pathname !== to) {
    window.history.pushState({}, '', to);
    window.dispatchEvent(new PopStateEvent('popstate'));
  }
}

function statusLabel(s: PluginStatus): string {
  switch (s) {
    case 'Loaded': return '已加载（未启用）';
    case 'Enabled': return '已启用（菜单可见）';
    case 'Running': return '运行中（cron 调度）';
    default: return s;
  }
}

const hostMenus: PluginMenu[] = [
  { key: 'plugins', title: '已加载插件', icon: null, route: '/', order: 0, children: [] },
  { key: 'libraries', title: '插件库管理', icon: null, route: '/libraries', order: 10, children: [] },
];

function buildMenuItems(plugins: PluginInfo[]): { key: string; icon: ReactNode; label: string; children?: any[] }[] {
  const pluginMenus = plugins.flatMap(p => p.menu || []);
  const all = [...hostMenus, ...pluginMenus].sort((a, b) => a.order - b.order);
  return all.map(m => {
    const icon = m.key === 'plugins' ? <ApiOutlined /> : m.key === 'libraries' ? <FolderOpenOutlined /> : <span>{m.icon}</span>;
    if (m.children && m.children.length > 0) {
      return {
        key: m.key,
        icon,
        label: m.title,
        children: m.children.map(c => ({
          key: c.route || c.key,
          icon: c.icon ? <span>{c.icon}</span> : undefined,
          label: c.title,
        })),
      };
    }
    return { key: m.route || m.key, icon, label: m.title };
  });
}

function PluginUiView({ plugin }: { plugin: PluginInfo }) {
  const entry = useMemo(() => qiankunEntryFor(plugin, window.location.origin), [plugin]);
  return (
    <div>
      <Card style={{ marginBottom: 16 }}>
        <Space>
          <span style={{ fontSize: 24 }}>🔌</span>
          <div>
            <Text strong style={{ fontSize: 16 }}>{plugin.name}</Text>
            <br />
            <Text type="secondary" code>{plugin.id}</Text>
          </div>
        </Space>
      </Card>
      {!entry ? (
        <Card>
          <div style={{ textAlign: 'center', padding: 40 }}>
            <div style={{ fontSize: 48, marginBottom: 12 }}>⚠️</div>
            <Title level={4}>该插件没有嵌入 UI</Title>
            <Text type="secondary">插件未声明 ui_base_dir，没有可用的 qiankun 子应用入口。</Text>
          </div>
        </Card>
      ) : (
        <div id="plugin-mount" style={{ minHeight: 400, background: '#141414', borderRadius: 8, padding: 20 }} />
      )}
    </div>
  );
}

export default function App(): ReactNode {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [libraries, setLibraries] = useState<LibraryInfo[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(false);

  const addLog = useCallback((msg: string) => {
    setLogs(prev => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev].slice(0, 100));
  }, []);

  const refreshPlugins = useCallback(async () => {
    try { const list = await listPlugins(); setPlugins(list); return list; }
    catch (e) { setError(`获取插件列表失败: ${e}`); return []; }
  }, []);

  const refreshLibraries = useCallback(async () => {
    try { const libs = await scanLibraries(); setLibraries(libs); return libs; }
    catch (e) { setError(`扫描插件库失败: ${e}`); return []; }
  }, []);

  useEffect(() => { refreshPlugins(); refreshLibraries(); }, [refreshPlugins, refreshLibraries]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const origin = window.location.origin;
    registerLoadedPlugins(plugins, origin).catch(err => console.error('qiankun registerLoadedPlugins failed', err));
  }, [plugins]);

  const handleLoad = async (name: string) => {
    try {
      setError(null); addLog(`正在加载插件库: ${name}...`);
      await loadLibrary(name); addLog(`✅ 加载成功`);
      await refreshPlugins(); await refreshLibraries(); navigate('/');
    } catch (e) { addLog(`❌ 加载失败: ${e}`); setError(`加载失败: ${e}`); }
  };

  const handleUnload = async (id: string, keepData = false) => {
    try {
      setError(null);
      addLog(keepData ? `正在卸载（保留数据）: ${id}...` : `正在完全卸载: ${id}...`);
      await unloadPlugin(id, keepData);
      addLog(keepData ? `✅ 已卸载: ${id}（数据已保留）` : `✅ 已完全卸载: ${id}（数据已删除）`);
      await refreshPlugins(); await refreshLibraries();
    } catch (e) { addLog(`❌ 卸载失败: ${e}`); setError(`卸载失败: ${e}`); }
  };

  const handleUnloadAll = async () => {
    try { setError(null); addLog('正在卸载所有插件...'); await unloadAllPlugins(); addLog('✅ 所有插件已卸载'); await refreshPlugins(); await refreshLibraries(); }
    catch (e) { addLog(`❌ 卸载失败: ${e}`); setError(`卸载失败: ${e}`); }
  };

  const handleEnable = async (id: string) => {
    try { setError(null); addLog(`正在启用: ${id}...`); await enablePlugin(id); addLog(`✅ 已启用: ${id}`); await refreshPlugins(); }
    catch (e) { addLog(`❌ 启用失败: ${e}`); setError(`启用失败: ${e}`); }
  };
  const handleDisable = async (id: string) => {
    try { setError(null); addLog(`正在禁用: ${id}...`); await disablePlugin(id); addLog(`✅ 已禁用: ${id}`); await refreshPlugins(); }
    catch (e) { addLog(`❌ 禁用失败: ${e}`); setError(`禁用失败: ${e}`); }
  };
  const handleStart = async (id: string) => {
    try { setError(null); addLog(`正在启动: ${id}...`); await startPlugin(id); addLog(`✅ 已启动: ${id}(cron 已注册)`); await refreshPlugins(); }
    catch (e) { addLog(`❌ 启动失败: ${e}`); setError(`启动失败: ${e}`); }
  };
  const handleStop = async (id: string) => {
    try { setError(null); addLog(`正在停止: ${id}...`); await stopPlugin(id); addLog(`✅ 已停止: ${id}`); await refreshPlugins(); }
    catch (e) { addLog(`❌ 停止失败: ${e}`); setError(`停止失败: ${e}`); }
  };

  const routePath = useRoute();
  const pluginRouteMatch = useMemo(() => {
    const m = /^\/plugin\/(.+)$/.exec(routePath); return m ? decodeURIComponent(m[1]) : null;
  }, [routePath]);
  const routedPlugin = useMemo(() => {
    if (!pluginRouteMatch) return null; return plugins.find(p => p.id === pluginRouteMatch) ?? null;
  }, [pluginRouteMatch, plugins]);

  const menuItems = useMemo(() => buildMenuItems(plugins), [plugins]);

  const handleMenuClick = (info: { key: string }) => {
    if (info.key.startsWith('/')) navigate(info.key);
    else if (info.key === 'plugins') navigate('/');
    else if (info.key === 'libraries') navigate('/libraries');
  };

  const selectedKeys = useMemo(() => {
    if (pluginRouteMatch) return [`/plugin/${encodeURIComponent(pluginRouteMatch)}`];
    return [routePath];
  }, [routePath, pluginRouteMatch]);

  let content: ReactNode;

  if (pluginRouteMatch) {
    content = routedPlugin ? <PluginUiView plugin={routedPlugin} /> : (
      <Card>
        <div style={{ textAlign: 'center', padding: 40 }}>
          <div style={{ fontSize: 48, marginBottom: 12 }}>🚫</div>
          <Title level={4}>插件未加载</Title>
          <Text type="secondary">找不到 id 为 <Text code>{pluginRouteMatch}</Text> 的插件</Text>
        </div>
      </Card>
    );
  } else if (routePath === '/libraries') {
    content = (
      <Card title={<><FolderOpenOutlined /> 可用的插件库</>} extra={<Button icon={<ReloadOutlined />} onClick={refreshLibraries}>重新扫描</Button>}>
        <Alert message="编译后的插件库位于 target/debug/ 目录下，扫描功能会自动查找这些文件。" type="info" showIcon style={{ marginBottom: 16 }} />
        {libraries.length === 0 ? (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <div style={{ fontSize: 48, marginBottom: 12 }}>🔍</div>
            <Text type="secondary">未扫描到插件库文件，请先编译插件库</Text>
          </div>
        ) : (
          <Space direction="vertical" style={{ width: '100%' }}>
            {libraries.map(lib => (
              <Card key={lib.path} size="small" hoverable
                style={{ borderColor: lib.loaded ? '#52c41a' : undefined }}
                extra={<Tag color={lib.loaded ? 'success' : 'default'}>{lib.loaded ? `已加载 (${lib.plugin_count} 插件)` : '未加载'}</Tag>}
              >
                <Space direction="vertical" style={{ width: '100%' }}>
                  <Space><Text strong>{lib.name}</Text><Text code>{lib.file_name}</Text></Space>
                  <Text type="secondary" code>路径: {lib.path}</Text>
                  <Button type="primary" size="small" disabled={lib.loaded}
                    icon={<DownloadOutlined />} onClick={() => handleLoad(lib.name)}
                  >{lib.loaded ? '✅ 已加载' : '📥 加载插件'}</Button>
                </Space>
              </Card>
            ))}
          </Space>
        )}
      </Card>
    );
  } else {
    content = (
      <Card title={<><ApiOutlined /> 已加载的插件 ({plugins.length})</>}
        extra={<Space>{plugins.length > 0 && <Button danger icon={<DeleteOutlined />} onClick={handleUnloadAll}>卸载全部</Button>}</Space>}>
        <Card size="small" style={{ marginBottom: 16, background: '#1a1a2e' }}>
          <Space wrap size={[2, 4]}>
            <Text type="secondary" style={{ fontSize: 12 }}>操作指引:</Text>
            <Tag color="default">已加载</Tag><Text type="secondary"> → </Text>
            <Tag color="blue">启用</Tag><Text type="secondary"> → </Text>
            <Tag color="processing">已启用</Tag><Text type="secondary"> → </Text>
            <Tag color="blue">启动</Tag><Text type="secondary"> → </Text>
            <Tag color="success">运行中</Tag><Text type="secondary"> → </Text>
            <Tag color="blue">停止</Tag><Text type="secondary"> → </Text>
            <Tag color="processing">已启用</Tag><Text type="secondary"> → </Text>
            <Tag color="blue">禁用</Tag><Text type="secondary"> → </Text>
            <Tag color="default">已加载</Tag><Text type="secondary"> → </Text>
            <Tag color="red">卸载</Tag>
          </Space>
          <div style={{ marginTop: 6 }}>
            <Space size={4}>
              <Tag color="blue" style={{ fontSize: 10 }}>按钮</Tag>
              <Text type="secondary" style={{ fontSize: 11 }}>= 用户操作</Text>
              <Tag color="processing" style={{ fontSize: 10 }}>状态</Tag>
              <Text type="secondary" style={{ fontSize: 11 }}>= 插件当前生命周期状态</Text>
            </Space>
          </div>
        </Card>
        {plugins.length === 0 ? (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <div style={{ fontSize: 48, marginBottom: 12 }}>📭</div>
            <Text type="secondary">尚未加载任何插件，请前往「插件库管理」页面扫描并加载插件库</Text>
          </div>
        ) : (
          <Space direction="vertical" style={{ width: '100%' }}>
            {plugins.map(p => {
              const statusColor = p.status === 'Running' ? 'success' : p.status === 'Enabled' ? 'processing' : 'default';
              return (
                <Card key={p.id} size="small" hoverable
                  actions={[
                    p.status === 'Loaded' ? <Popconfirm key="enable" title={`启用 ${p.name}?`} onConfirm={() => handleEnable(p.id)}><Button type="primary" size="small" icon={<CaretUpOutlined />}>启用</Button></Popconfirm> : null,
                    p.status === 'Enabled' ? <Popconfirm key="start" title={`启动 ${p.name}（后台任务 + cron）?`} onConfirm={() => handleStart(p.id)}><Button type="primary" size="small" icon={<PlayCircleOutlined />}>启动</Button></Popconfirm> : null,
                    p.status === 'Enabled' ? <Popconfirm key="disable" title={`禁用 ${p.name}?`} onConfirm={() => handleDisable(p.id)}><Button size="small" icon={<PauseCircleOutlined />}>禁用</Button></Popconfirm> : null,
                    p.status === 'Running' ? <Popconfirm key="stop" title={`停止 ${p.name}?`} onConfirm={() => handleStop(p.id)}><Button size="small" icon={<CaretDownOutlined />}>停止</Button></Popconfirm> : null,
                    p.status === 'Loaded' ? <Popconfirm key="unload-full" title="完全卸载？插件数据（表、配置）将被永久删除，不可恢复。" onConfirm={() => handleUnload(p.id, false)}><Button danger size="small" icon={<DeleteOutlined />}>完全卸载</Button></Popconfirm> : null,
                    p.status === 'Loaded' ? <Popconfirm key="unload-keep" title="仅卸载？插件将被卸载，但数据（表、配置）会保留，重新加载后即可恢复。" onConfirm={() => handleUnload(p.id, true)}><Button size="small">仅卸载</Button></Popconfirm> : null,
                  ].filter(Boolean)}
                >
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <Space>
                      <Text strong>{p.name}</Text>
                      <Tag color={statusColor}>{statusLabel(p.status)}</Tag>
                      <Text code>v{p.version}</Text>
                    </Space>
                    {p.author && <Text type="secondary" style={{ fontSize: 12 }}>作者: {p.author}</Text>}
                    {p.description && <Text type="secondary" style={{ fontSize: 12 }} ellipsis>{p.description}</Text>}
                    <Space>
                      <Text type="secondary" style={{ fontSize: 12 }}>ID:</Text>
                      <Text code style={{ fontSize: 12 }}>{p.id}</Text>
                    </Space>
                    <Space>
                      <Text type="secondary" style={{ fontSize: 12 }}>UI:</Text>
                      <Text style={{ fontSize: 12 }}>{p.has_ui ? '✅ 已嵌入' : '—'}</Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>Cron:</Text>
                      <Text style={{ fontSize: 12 }}>{p.has_cron ? '⏰ 已配置' : '—'}</Text>
                    </Space>
                  </Space>
                </Card>
              );
            })}
          </Space>
        )}
      </Card>
    );
  }

  return (
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
      <AntApp>
        <Layout style={{ minHeight: '100vh' }}>
          <Sider collapsible collapsed={collapsed} onCollapse={setCollapsed} theme="dark">
            <div style={{ height: 32, margin: 16, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <Text strong style={{ color: '#1677ff', fontSize: collapsed ? 14 : 16 }}>{collapsed ? 'P' : 'plugkit'}</Text>
              {!collapsed && <Tag style={{ marginLeft: 8 }}>v0.2</Tag>}
            </div>
            <Menu theme="dark" mode="inline" selectedKeys={selectedKeys} items={menuItems} onClick={handleMenuClick} />
          </Sider>
          <Layout>
            <Content style={{ margin: 16, overflow: 'auto' }}>
              {error && <Alert message={error} type="error" closable onClose={() => setError(null)} style={{ marginBottom: 16 }} />}
              {content}
            </Content>
            <Footer style={{ padding: '8px 16px', background: '#141414', borderTop: '1px solid #303030', maxHeight: 160, overflow: 'auto' }}>
              <Text type="secondary" style={{ fontSize: 12 }}>📋 操作日志</Text>
              <div style={{ marginTop: 4 }}>
                {logs.length === 0 && <Text type="secondary" italic style={{ fontSize: 12 }}>暂无操作</Text>}
                {logs.map((log, i) => <div key={i}><Text style={{ fontSize: 12, fontFamily: 'monospace' }} type="secondary">{log}</Text></div>)}
              </div>
            </Footer>
          </Layout>
        </Layout>
      </AntApp>
    </ConfigProvider>
  );
}