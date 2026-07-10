import { useState, useEffect } from 'react';
import type { PluginInfo, PluginStatus, CronInfo } from '../api';
import { listCrons, runCron } from '../api';

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

function confirmAction(msg: string): boolean {
  return window.confirm(msg);
}

interface PluginCardProps {
  plugin: PluginInfo;
  onUnload: (id: string) => void;
  onEnable: (id: string) => Promise<void>;
  onDisable: (id: string) => Promise<void>;
  onStart: (id: string) => Promise<void>;
  onStop: (id: string) => Promise<void>;
  onRefresh: () => Promise<PluginInfo[]>;
}

function PluginCard({ plugin, onUnload, onEnable, onDisable, onStart, onStop, onRefresh }: PluginCardProps) {
  const [crons, setCrons] = useState<CronInfo[]>([]);
  const meta = STATUS_META[plugin.status];

  useEffect(() => {
    listCrons(plugin.id)
      .then(setCrons)
      .catch(() => setCrons([]));
  }, [plugin.id, plugin.status]);

  const handleRunCron = async (name: string) => {
    if (!confirmAction(`手动执行 cron "${name}"?`)) return;
    try {
      await runCron(plugin.id, name);
      await onRefresh();
    } catch (e) {
      window.alert(`执行失败: ${e}`);
    }
  };

  return (
    <div className="plugin-card">
      <div className="plugin-card-header">
        <span className="plugin-name">{plugin.name}</span>
        <span className="status-badge" style={{ background: meta.color, color: 'white' }}>
          {meta.label}
        </span>
      </div>
      <div className="plugin-card-body">
        <div className="field">
          <span className="field-label">插件 ID</span>
          <code className="field-value plugin-id">{plugin.id}</code>
        </div>
        <div className="field">
          <span className="field-label">版本</span>
          <code className="field-value">{plugin.version}</code>
        </div>
        <div className="field">
          <span className="field-label">UI</span>
          <code className="field-value">{plugin.has_ui ? '✅ 已嵌入' : '—'}</code>
        </div>
      </div>
      {crons.length > 0 && (
        <div className="plugin-card-body">
          <div className="field">
            <span className="field-label">定时任务</span>
          </div>
          {crons.map(c => (
            <div
              key={c.name}
              style={{ display: 'flex', alignItems: 'center', gap: 8, margin: '4px 0' }}
            >
              <span
                className="status-badge"
                style={{
                  background: c.running ? 'var(--green)' : 'var(--bg-hover)',
                  color: 'white',
                }}
              >
                {c.running ? '运行中' : '已停止'}
              </span>
              <code>{c.name}</code>
              <span style={{ color: 'var(--text-dim)' }}>每 {c.interval_secs}s</span>
              <button className="btn btn-secondary btn-sm" onClick={() => handleRunCron(c.name)}>
                ⚡ 执行一次
              </button>
            </div>
          ))}
        </div>
      )}
      <div className="plugin-card-footer">
        {plugin.status === 'Loaded' && (
          <button
            className="btn btn-primary btn-sm"
            onClick={() => {
              if (confirmAction(`启用 ${plugin.name}?`)) onEnable(plugin.id);
            }}
          >
            启用
          </button>
        )}
        {plugin.status === 'Enabled' && (
          <>
            <button
              className="btn btn-primary btn-sm"
              onClick={() => {
                if (confirmAction(`启动 ${plugin.name}(后台任务 + cron)?`)) onStart(plugin.id);
              }}
            >
              启动
            </button>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => {
                if (confirmAction(`禁用 ${plugin.name}?`)) onDisable(plugin.id);
              }}
            >
              禁用
            </button>
          </>
        )}
        {plugin.status === 'Running' && (
          <button
            className="btn btn-secondary btn-sm"
            onClick={() => {
              if (confirmAction(`停止 ${plugin.name}?`)) onStop(plugin.id);
            }}
          >
            停止
          </button>
        )}
        <button
          className="btn btn-danger btn-sm"
          onClick={() => {
            if (confirmAction(`卸载 ${plugin.name}(清理 + 关库)?`)) onUnload(plugin.id);
          }}
        >
          卸载
        </button>
      </div>
    </div>
  );
}

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
        <strong>💡 插件生命周期状态机</strong>
        <p>
          load → <strong>Loaded</strong> → enable → <strong>Enabled</strong>(菜单可见)→ start →{' '}
          <strong>Running</strong>(cron 运行)→ stop → Enabled → disable → Loaded → unload → 移除
        </p>
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
            <PluginCard
              key={plugin.id}
              plugin={plugin}
              onUnload={onUnload}
              onEnable={onEnable}
              onDisable={onDisable}
              onStart={onStart}
              onStop={onStop}
              onRefresh={onRefresh}
            />
          ))}
        </div>
      )}
    </div>
  );
}