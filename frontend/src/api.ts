const API_BASE = '/api';

export interface LibraryInfo {
  name: string;
  file_name: string;
  path: string;
  loaded: boolean;
  plugin_count: number;
}

export type PluginStatus = 'Loaded' | 'Enabled' | 'Running';

export interface PluginMenu {
  key: string;
  title: string;
  icon: string | null;
  route: string | null;
  order: number;
  children: PluginMenu[];
}

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  has_ui: boolean;
  has_cron: boolean;
  /** 是否使用了数据库表（metadata.tables 非空） */
  has_database: boolean;
  /** qiankun 子应用入口相对路径 */
  ui_entry: string | null;
  /** qiankun 子应用入口绝对 URL（由 micro.ts 填充） */
  qiankunEntry?: string;
  /** 插件声明的菜单树 */
  menu: PluginMenu[];
  /** 插件当前生命周期状态 */
  status: PluginStatus;
}

export async function scanLibraries(): Promise<LibraryInfo[]> {
  const res = await fetch(`${API_BASE}/libraries`);
  const data = await res.json();
  return data.libraries;
}

export async function loadLibrary(name: string): Promise<PluginInfo[]> {
  const res = await fetch(`${API_BASE}/libraries/${encodeURIComponent(name)}/load`, {
    method: 'POST',
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || '加载失败');
  }
  const data = await res.json();
  return data.plugins;
}

export async function listPlugins(): Promise<PluginInfo[]> {
  const res = await fetch(`${API_BASE}/plugins`);
  return res.json();
}

export async function getPlugin(id: string): Promise<PluginInfo> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}`);
  if (!res.ok) throw new Error('插件不存在');
  return res.json();
}

export async function unloadPlugin(id: string, keepData = false): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}?keep_data=${keepData}`, {
    method: 'DELETE',
  });
  if (!res.ok) throw new Error('卸载失败');
}

export async function unloadAllPlugins(): Promise<void> {
  await fetch(`${API_BASE}/plugins`, { method: 'DELETE' });
}

export async function enablePlugin(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/enable`, { method: 'POST' });
  if (!res.ok) throw new Error('启用失败');
}

export async function disablePlugin(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/disable`, { method: 'POST' });
  if (!res.ok) throw new Error('禁用失败');
}

export async function startPlugin(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/start`, { method: 'POST' });
  if (!res.ok) throw new Error('启动失败');
}

export async function stopPlugin(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/stop`, { method: 'POST' });
  if (!res.ok) throw new Error('停止失败');
}

export interface CronInfo {
  name: string;
  interval_secs: number;
  running: boolean;
}

export async function listCrons(id: string): Promise<CronInfo[]> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/cron`);
  if (!res.ok) throw new Error('获取 cron 失败');
  return res.json();
}

export async function runCron(id: string, name: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/cron/run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
  });
  if (!res.ok) throw new Error('执行 cron 失败');
}