import { useState, useEffect, useCallback, useMemo } from 'react';
import type { ReactNode, ReactElement } from 'react';
import { ConfigProvider, Layout, theme, Menu, Card, Button, Tag, Popconfirm, Space, Typography, Alert, App as AntApp, Dropdown, Avatar } from 'antd';
import {
  ApiOutlined, FolderOpenOutlined, DownloadOutlined, ReloadOutlined,
  DeleteOutlined, PlayCircleOutlined, PauseCircleOutlined,
  CaretUpOutlined, CaretDownOutlined, UserOutlined, LogoutOutlined
} from '@ant-design/icons';
import type { PluginInfo, LibraryInfo, PluginMenu } from './api';
import { scanLibraries, listPlugins, loadLibrary, unloadPlugin, unloadAllPlugins, enablePlugin, disablePlugin, startPlugin, stopPlugin, upgradePlugin } from './api';
import { registerLoadedPlugins, qiankunEntryFor } from './micro';
import { useThemeMode } from './theme/use-theme-mode.tsx';
import { ThemeToggle } from './components/theme-toggle';
import { isAuthenticated, getUser, logout } from './auth';
import LoginPage from './LoginPage';

const { Sider, Content } = Layout;
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

/** 固定显示的是/否徽标：始终可见，根据布尔值切换文案与颜色。 */
function YesNo({ value }: { value: boolean }): ReactElement {
  return value
    ? <Tag color="success" style={{ fontSize: 12 }}>✅ 是</Tag>
    : <Tag style={{ fontSize: 12 }}>⭕ 否</Tag>;
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
      {!entry ? (
        <Card>
          <div style={{ textAlign: 'center', padding: 40 }}>
            <div style={{ fontSize: 48, marginBottom: 12 }}>⚠️</div>
            <Title level={4}>该插件没有嵌入 UI</Title>
            <Text type="secondary">插件未声明 ui_base_dir，没有可用的 qiankun 子应用入口。</Text>
          </div>
        </Card>
      ) : (
        <div id="plugin-mount" />
      )}
    </div>
  );
}

export default function App(): ReactNode {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [libraries, setLibraries] = useState<LibraryInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [authed, setAuthed] = useState(isAuthenticated());
  const { isDark } = useThemeMode();

  const refreshPlugins = useCallback(async () => {
    try { const list = await listPlugins(); setPlugins(list); return list; }
    catch (e) { setError(`获取插件列表失败: ${e}`); return []; }
  }, []);

  const refreshLibraries = useCallback(async () => {
    try { const libs = await scanLibraries(); setLibraries(libs); return libs; }
    catch (e) { setError(`扫描插件库失败: ${e}`); return []; }
  }, []);

  useEffect(() => {
    if (authed) {
      refreshPlugins();
      refreshLibraries();
    }
  }, [refreshPlugins, refreshLibraries, authed]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const origin = window.location.origin;
    registerLoadedPlugins(plugins, origin).catch(err => console.error('qiankun registerLoadedPlugins failed', err));
  }, [plugins]);

  const handleLoginSuccess = () => {
    setAuthed(true);
    navigate('/');
  };

  const handleLogout = async () => {
    await logout();
    setAuthed(false);
    navigate('/login');
  };

  const handleLoad = async (name: string) => {
    try {
      setError(null);
      await loadLibrary(name);
      await refreshPlugins(); await refreshLibraries(); navigate('/');
    } catch (e) { setError(`加载失败: ${e}`); }
  };

  const handleUnload = async (id: string, keepData = false) => {
    try {
      setError(null);
      await unloadPlugin(id, keepData);
      await refreshPlugins(); await refreshLibraries();
    } catch (e) { setError(`卸载失败: ${e}`); }
  };

  const handleUnloadAll = async () => {
    try { setError(null); await unloadAllPlugins(); await refreshPlugins(); await refreshLibraries(); }
    catch (e) { setError(`卸载失败: ${e}`); }
  };

  const handleEnable = async (id: string) => {
    try { setError(null); await enablePlugin(id); await refreshPlugins(); }
    catch (e) { setError(`启用失败: ${e}`); }
  };
  const handleDisable = async (id: string) => {
    try { setError(null); await disablePlugin(id); await refreshPlugins(); }
    catch (e) { setError(`禁用失败: ${e}`); }
  };
  const handleStart = async (id: string) => {
    try { setError(null); await startPlugin(id); await refreshPlugins(); }
    catch (e) { setError(`启动失败: ${e}`); }
  };
  const handleStop = async (id: string) => {
    try { setError(null); await stopPlugin(id); await refreshPlugins(); }
    catch (e) { setError(`停止失败: ${e}`); }
  };
  const handleUpgrade = async (id: string) => {
    try { setError(null); await upgradePlugin(id); await refreshPlugins(); }
    catch (e) { setError(`升级失败: ${e}`); }
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
        {plugins.length === 0 ? (
          <div style={{ textAlign: 'center', padding: 40 }}>
            <div style={{ fontSize: 48, marginBottom: 12 }}>📭</div>
            <Text type="secondary">尚未加载任何插件，请前往「插件库管理」页面扫描并加载插件库</Text>
          </div>
        ) : (
          <Space direction="vertical" style={{ width: '100%' }}>
            {plugins.map(p => {
              return (
                <Card key={p.id} size="small" hoverable
                  actions={[
                    p.status === 'Loaded' ? <Popconfirm key="enable" title={`启用 ${p.name}?`} onConfirm={() => handleEnable(p.id)}><Button type="primary" size="small" icon={<CaretUpOutlined />}>启用</Button></Popconfirm> : null,
                    p.status === 'Enabled' ? <Popconfirm key="start" title={`启动 ${p.name}（后台任务 + cron）?`} onConfirm={() => handleStart(p.id)}><Button type="primary" size="small" icon={<PlayCircleOutlined />}>启动</Button></Popconfirm> : null,
                    p.status === 'Enabled' ? <Popconfirm key="disable" title={`禁用 ${p.name}?`} onConfirm={() => handleDisable(p.id)}><Button size="small" icon={<PauseCircleOutlined />}>禁用</Button></Popconfirm> : null,
                    p.status === 'Running' ? <Popconfirm key="stop" title={`停止 ${p.name}?`} onConfirm={() => handleStop(p.id)}><Button size="small" icon={<CaretDownOutlined />}>停止</Button></Popconfirm> : null,
                    p.status === 'Loaded' ? <Popconfirm key="unload-full" title="完全卸载？插件数据（表、配置）将被永久删除，不可恢复。" onConfirm={() => handleUnload(p.id, false)}><Button danger size="small" icon={<DeleteOutlined />}>完全卸载</Button></Popconfirm> : null,
                    p.status === 'Loaded' ? <Popconfirm key="unload-keep" title="仅卸载？插件将被卸载，但数据（表、配置）会保留，重新加载后即可恢复。" onConfirm={() => handleUnload(p.id, true)}><Button size="small">仅卸载</Button></Popconfirm> : null,
                    p.needs_upgrade ? <Popconfirm key="upgrade" title={`升级 ${p.name}?`} onConfirm={() => handleUpgrade(p.id)}><Button size="small" type="primary" icon={<ReloadOutlined />}>升级</Button></Popconfirm> : null,
                  ].filter(Boolean)}
                >
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <Space>
                      <Text strong>{p.name}</Text>
                      <Text code>v{p.version}</Text>
                      {p.installed_version && p.installed_version !== p.version ? (
                        <Text type="warning" style={{ fontSize: 11 }}>（已安装: v{p.installed_version}）</Text>
                      ) : p.installed_version ? (
                        <Text type="success" style={{ fontSize: 11 }}>✓ 已安装</Text>
                      ) : (
                        <Text type="secondary" style={{ fontSize: 11 }}>未安装</Text>
                      )}
                    </Space>
                    {p.author && <Text type="secondary" style={{ fontSize: 12 }}>作者: {p.author}</Text>}
                    {p.description && <Text type="secondary" style={{ fontSize: 12 }} ellipsis>{p.description}</Text>}
                    <Space>
                      <Text type="secondary" style={{ fontSize: 12 }}>ID:</Text>
                      <Text code style={{ fontSize: 12 }}>{p.id}</Text>
                    </Space>
                    <Space wrap>
                      <Text type="secondary" style={{ fontSize: 12 }}>能力:</Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>UI</Text>
                      <YesNo value={p.has_ui} />
                      <Text type="secondary" style={{ fontSize: 12 }}>Cron</Text>
                      <YesNo value={p.has_cron} />
                      <Text type="secondary" style={{ fontSize: 12 }}>数据库</Text>
                      <YesNo value={p.has_database} />
                    </Space>
                    <Space wrap>
                      <Text type="secondary" style={{ fontSize: 12 }}>状态:</Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>启用</Text>
                      <YesNo value={p.status !== 'Loaded'} />
                      <Text type="secondary" style={{ fontSize: 12 }}>UI嵌入</Text>
                      <YesNo value={p.has_ui && (p.status === 'Enabled' || p.status === 'Running')} />
                      <Text type="secondary" style={{ fontSize: 12 }}>Cron运行</Text>
                      <YesNo value={p.status === 'Running'} />
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

  // 未登录且不在登录页 → 跳转登录
  if (!authed && routePath !== '/login') {
    navigate('/login');
    return null;
  }

  // 登录页
  if (routePath === '/login') {
    if (authed) {
      navigate('/');
      return null;
    }
    return (
      <ConfigProvider
        theme={{
          algorithm: isDark ? theme.darkAlgorithm : theme.defaultAlgorithm,
          cssVar: { key: isDark ? 'dark' : 'light' },
          hashed: false,
        }}
      >
        <AntApp>
          <LoginPage onSuccess={handleLoginSuccess} />
        </AntApp>
      </ConfigProvider>
    );
  }

  return (
    <ConfigProvider
      theme={{
        algorithm: isDark ? theme.darkAlgorithm : theme.defaultAlgorithm,
        cssVar: { key: isDark ? 'dark' : 'light' },
        hashed: false,
      }}
    >
      <AntApp>
        <Layout style={{ minHeight: '100vh' }}>
          <Sider collapsible collapsed={collapsed} onCollapse={setCollapsed} theme={isDark ? 'dark' : 'light'}>
            <div style={{ height: 32, margin: 16, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <Text strong style={{ color: 'var(--color-primary)', fontSize: collapsed ? 14 : 16 }}>{collapsed ? 'P' : 'plugkit'}</Text>
              {!collapsed && <Tag style={{ marginLeft: 8 }}>v0.2</Tag>}
            </div>
            <Menu theme={isDark ? 'dark' : 'light'} mode="inline" selectedKeys={selectedKeys} items={menuItems} onClick={handleMenuClick} />
            <div style={{ marginTop: 'auto', padding: 12, borderTop: '1px solid var(--color-border)' }}>
              <ThemeToggle />
            </div>
          </Sider>
          <Layout>
            <div style={{
              padding: '12px 16px',
              display: 'flex',
              justifyContent: 'flex-end',
              alignItems: 'center',
              borderBottom: '1px solid var(--color-border)',
            }}>
              <Dropdown
                menu={{
                  items: [
                    { key: 'logout', icon: <LogoutOutlined />, label: '登出', onClick: handleLogout },
                  ],
                }}
              >
                <Space style={{ cursor: 'pointer' }}>
                  <Avatar size="small" icon={<UserOutlined />} />
                  <Text>{getUser()?.username || '用户'}</Text>
                </Space>
              </Dropdown>
            </div>
            <Content style={{ margin: 16, overflow: 'auto' }}>
              {error && <Alert message={error} type="error" closable onClose={() => setError(null)} style={{ marginBottom: 16 }} />}
              {content}
            </Content>
          </Layout>
        </Layout>
      </AntApp>
    </ConfigProvider>
  );
}