const API_BASE = '/api';

export interface LibraryInfo {
  name: string;
  file_name: string;
  path: string;
  loaded: boolean;
  plugin_count: number;
}

export interface PluginInfo {
  id: string;
  agency: string;
  has_ui: boolean;
  module_type: string | null;
  ui_tag_name: string | null;
  ui_js_path: string | null;
  /**
   * qiankun 子应用入口 URL（由 micro.ts 计算并填充）。
   * 仅当插件编译为 qiankun 微前端时存在；前端使用此字段作为
   * `registerMicroApps` 的 entry。
   */
  qiankunEntry?: string;
}

export interface PluginUiInfo {
  tag_name: string;
  js_url: string;
  module_type: string;
}

export async function getPluginUi(id: string): Promise<PluginUiInfo> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/ui`);
  if (!res.ok) throw new Error('该插件没有关联的 UI');
  return res.json();
}

export interface ArticleResponse {
  headline: string;
  body: string;
  dateline: string;
  agency: string;
}

export interface PublishRequest {
  headline: string;
  body: string;
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

export async function publishArticle(id: string, req: PublishRequest): Promise<ArticleResponse> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}/publish`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || '发布失败');
  }
  return res.json();
}

export async function unloadPlugin(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
  if (!res.ok) throw new Error('卸载失败');
}

export async function unloadAllPlugins(): Promise<void> {
  await fetch(`${API_BASE}/plugins`, { method: 'DELETE' });
}