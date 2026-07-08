/**
 * 路透社插件 React 面板（qiankun 子应用版本）
 *
 * 之前由宿主注入 React / createRoot；现在此项目独立打包 React，
 * 由 vite-plugin-qiankun 在 qiankun 容器中挂载。
 */

import { useState, useCallback, useEffect } from 'react';

interface ReutersPanelProps {
  pluginId?: string;
}

export function ReutersPanel({ pluginId = 'reuters_plugin' }: ReutersPanelProps) {
  const [dateline, setDateline] = useState('LONDON');
  const [status, setStatus] = useState<'saving' | 'success' | 'error' | null>(null);

  const api = {
    async getSettings(id: string): Promise<Record<string, unknown> | null> {
      try {
        const raw = localStorage.getItem(`plugin-settings-${id}`);
        return raw ? JSON.parse(raw) : null;
      } catch {
        return null;
      }
    },
    async saveSettings(id: string, settings: Record<string, unknown>): Promise<void> {
      localStorage.setItem(`plugin-settings-${id}`, JSON.stringify(settings));
    },
  };

  useEffect(() => {
    api.getSettings(pluginId)
      .then((s) => {
        if (s && typeof s.dateline === 'string') setDateline(s.dateline);
      })
      .catch(() => undefined);
  }, [pluginId]);

  const handleSave = useCallback(async () => {
    setStatus('saving');
    try {
      await api.saveSettings(pluginId, { dateline });
      setStatus('success');
      setTimeout(() => setStatus(null), 2000);
    } catch {
      setStatus('error');
      setTimeout(() => setStatus(null), 2000);
    }
  }, [dateline, pluginId]);

  return (
    <div className="plugin-panel">
      <h3 className="panel-title">📰 路透社控制面板</h3>
      <p className="panel-desc">配置路透社的电头（dateline）偏好设置</p>

      <div className="field">
        <label className="field-label">电头 (Dateline)</label>
        <input
          className="field-input"
          value={dateline}
          onChange={(e) => setDateline(e.target.value)}
          placeholder="如 LONDON, NEW YORK…"
        />
      </div>

      <div className="field">
        <label className="field-label">当前插件 ID</label>
        <code className="field-code">{pluginId}</code>
      </div>

      <button
        className="btn btn-primary"
        onClick={handleSave}
        disabled={status === 'saving'}
      >
        {status === 'saving' ? '⏳ 保存中…' : '💾 保存设置'}
      </button>

      {status === 'success' && <p className="msg success">✅ 设置已保存</p>}
      {status === 'error' && <p className="msg error">❌ 保存失败，请重试</p>}
    </div>
  );
}

export default ReutersPanel;
