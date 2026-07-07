/**
 * 路透社插件 React 面板
 *
 * TypeScript 源码，由 esbuild 编译为 ESM `.js`，浏览器通过 import() 加载。
 * 宿主注入 React / createRoot / pluginId / api。
 */
import type { createRoot as CreateRoot } from 'react-dom/client';

// ── 类型 ──────────────────────────────────────────────

type ReactType = typeof import('react');

interface PluginDeps {
  React: ReactType;
  createRoot: typeof CreateRoot;
  pluginId: string;
  api: {
    getSettings(id: string): Promise<Record<string, unknown> | null>;
    saveSettings(id: string, settings: Record<string, unknown>): Promise<void>;
  };
}

// ── mount 入口（被宿主调用） ──────────────────────────

export function mount(container: Element, deps: PluginDeps): () => void {
  const { React, createRoot, pluginId, api } = deps;
  const { useState, useCallback, useEffect, createElement: h } = React;

  // ── 面板组件 ──

  function ReutersPanel() {
    const [dateline, setDateline] = useState('LONDON');
    const [status, setStatus] = useState<string | null>(null); // 'saving' | 'success' | 'error'

    useEffect(() => {
      api.getSettings(pluginId).then((s) => {
        if (s?.dateline) setDateline(s.dateline as string);
      }).catch(() => {});
    }, []);

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
    }, [dateline]);

    return h('div', { className: 'plugin-panel' },
      h('h3', { className: 'panel-title' }, '📰 路透社控制面板'),
      h('p', { className: 'panel-desc' }, '配置路透社的电头（dateline）偏好设置'),

      h('div', { className: 'field' },
        h('label', { className: 'field-label' }, '电头 (Dateline)'),
        h('input', {
          className: 'field-input',
          value: dateline,
          onChange: (e: any) => setDateline(e.target.value),
          placeholder: '如 LONDON, NEW YORK…',
        }),
      ),

      h('div', { className: 'field' },
        h('label', { className: 'field-label' }, '当前插件 ID'),
        h('code', { className: 'field-code' }, pluginId),
      ),

      h('button', {
        className: 'btn btn-primary',
        onClick: handleSave,
        disabled: status === 'saving',
      }, status === 'saving' ? '⏳ 保存中…' : '💾 保存设置'),

      status === 'success' && h('p', { className: 'msg success' }, '✅ 设置已保存'),
      status === 'error' && h('p', { className: 'msg error' }, '❌ 保存失败，请重试'),
    );
  }

  // 渲染
  const root = createRoot(container);
  root.render(h(ReutersPanel));

  // 返回清理函数
  return () => root.unmount();
}