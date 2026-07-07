import type { PluginInfo } from '../api';

interface PluginListProps {
  plugins: PluginInfo[];
  onUnload: (id: string) => void;
  onUnloadAll: () => void;
  onRefresh: () => Promise<PluginInfo[]>;
}

export default function PluginList({ plugins, onUnload, onUnloadAll, onRefresh }: PluginListProps) {
  return (
    <div className="section-card">
      <div className="section-header">
        <h2>🔌 已加载的插件 ({plugins.length})</h2>
        <div className="header-actions">
          <button className="btn btn-secondary" onClick={() => onRefresh()}>
            ⟳ 刷新
          </button>
          {plugins.length > 0 && (
            <button className="btn btn-danger" onClick={onUnloadAll}>
              🗑️ 卸载全部
            </button>
          )}
        </div>
      </div>

      <div className="info-box">
        <strong>💡 插件全生命周期管理</strong>
        <ol>
          <li><strong>发现</strong> — 在「插件库管理」中扫描可用的 .dylib 文件</li>
          <li><strong>加载</strong> — 点击加载按钮，框架自动打开动态库、检查版本兼容性、调用注册函数</li>
          <li><strong>查看</strong> — 在此页面查看已加载的插件列表，每个插件有唯一 ID 和机构名称</li>
          <li><strong>调用</strong> — 在「发布新闻」页面选择插件，输入新闻内容，调用 publish() 方法</li>
          <li><strong>卸载</strong> — 卸载单个插件或全部卸载，框架关闭动态库释放资源</li>
        </ol>
      </div>

      {plugins.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">📭</div>
          <div className="empty-text">尚未加载任何插件</div>
          <div className="empty-hint">请前往「插件库管理」页面扫描并加载插件库</div>
        </div>
      ) : (
        <div className="plugin-grid">
          {plugins.map(plugin => (
            <div key={plugin.id} className="plugin-card">
              <div className="plugin-card-header">
                <span className="plugin-agency-icon">
                  {plugin.agency === 'Reuters' ? '📰' : plugin.agency === 'Agence France-Presse' ? '🇫🇷' : '📡'}
                </span>
                <span className="plugin-agency-name">{plugin.agency}</span>
              </div>
              <div className="plugin-card-body">
                <div className="field">
                  <span className="field-label">插件 ID</span>
                  <code className="field-value plugin-id">{plugin.id}</code>
                </div>
                <div className="field">
                  <span className="field-label">状态</span>
                  <span className="status-badge status-loaded">已加载 ✅</span>
                </div>
              </div>
              <div className="plugin-card-footer">
                <button
                  className="btn btn-danger btn-sm"
                  onClick={() => onUnload(plugin.id)}
                >
                  卸载
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}