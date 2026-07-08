import type { PluginInfo, PluginStatus } from '../api';

interface PluginListProps {
  plugins: PluginInfo[];
  onUnload: (id: string) => void;
  onUnloadAll: () => void;
  onRefresh: () => Promise<PluginInfo[]>;
  onEnable: (id: string) => Promise<void>;
  onDisable: (id: string) => Promise<void>;
  onStart: (id: string) => Promise<void>;
  onStop: (id: string) => Promise<void>;
}

const STATUS_META: Record<PluginStatus, { label: string; color: string }> = {
  Loaded: { label: '已加载', color: 'var(--bg-hover)' },
  Enabled: { label: '已启用', color: 'var(--accent)' },
  Running: { label: '运行中', color: 'var(--green)' },
};

export default function PluginList({
  plugins,
  onUnload,
  onUnloadAll,
  onRefresh,
  onEnable,
  onDisable,
  onStart,
  onStop,
}: PluginListProps) {
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
        <strong>💡 插件全生命周期状态机</strong>
        <ol>
          <li><strong>发现</strong> — 在「插件库管理」中扫描可用的 .dylib 文件</li>
          <li><strong>加载</strong> — on_load + on_install,状态 = Loaded</li>
          <li><strong>启用</strong> — on_enable,状态 = Enabled,菜单对前端可见</li>
          <li><strong>启动</strong> — on_start + cron 注册,状态 = Running,后台任务运行</li>
          <li><strong>停止/禁用/卸载</strong> — on_stop / on_disable / on_unload,资源收敛</li>
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
          {plugins.map(plugin => {
            const meta = STATUS_META[plugin.status];
            return (
              <div key={plugin.id} className="plugin-card">
                <div className="plugin-card-header">
                  <span className="plugin-agency-icon">
                    {plugin.agency === 'Reuters'
                      ? '📰'
                      : plugin.agency === 'Agence France-Presse'
                        ? '🇫🇷'
                        : '📡'}
                  </span>
                  <span className="plugin-agency-name">{plugin.agency}</span>
                  <span
                    className="status-badge"
                    style={{ background: meta.color, color: 'white' }}
                  >
                    {meta.label}
                  </span>
                </div>
                <div className="plugin-card-body">
                  <div className="field">
                    <span className="field-label">插件 ID</span>
                    <code className="field-value plugin-id">{plugin.id}</code>
                  </div>
                </div>
                <div className="plugin-card-footer">
                  {plugin.status === 'Loaded' && (
                    <button className="btn btn-primary btn-sm" onClick={() => onEnable(plugin.id)}>
                      启用
                    </button>
                  )}
                  {plugin.status === 'Enabled' && (
                    <>
                      <button className="btn btn-primary btn-sm" onClick={() => onStart(plugin.id)}>
                        启动
                      </button>
                      <button className="btn btn-secondary btn-sm" onClick={() => onDisable(plugin.id)}>
                        禁用
                      </button>
                    </>
                  )}
                  {plugin.status === 'Running' && (
                    <button className="btn btn-secondary btn-sm" onClick={() => onStop(plugin.id)}>
                      停止
                    </button>
                  )}
                  <button className="btn btn-danger btn-sm" onClick={() => onUnload(plugin.id)}>
                    卸载
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
