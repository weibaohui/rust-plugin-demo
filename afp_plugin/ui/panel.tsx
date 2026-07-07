/**
 * 法新社插件 React 面板
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

// ── mount 入口 ────────────────────────────────────────

export function mount(container: Element, deps: PluginDeps): () => void {
  const { React, createRoot, pluginId, api } = deps;
  const { useState, useCallback, useEffect, createElement: h } = React;

  // ── 面板组件 ──

  function AfpPanel() {
    const [language, setLanguage] = useState('fr');
    const [status, setStatus] = useState<string | null>(null);

    useEffect(() => {
      api.getSettings(pluginId).then((s) => {
        if (s?.language) setLanguage(s.language as string);
      }).catch(() => {});
    }, []);

    const handleSave = useCallback(async () => {
      setStatus('saving');
      try {
        await api.saveSettings(pluginId, { language });
        setStatus('success');
        setTimeout(() => setStatus(null), 2000);
      } catch {
        setStatus('error');
        setTimeout(() => setStatus(null), 2000);
      }
    }, [language]);

    return h('div', { className: 'plugin-panel' },
      h('h3', { className: 'panel-title' }, '📡 法新社控制面板'),
      h('p', { className: 'panel-desc' }, '配置 AFP 的默认语言偏好'),

      h('div', { className: 'field' },
        h('label', { className: 'field-label' }, '语言 (Language)'),
        h('select', {
          className: 'field-input',
          value: language,
          onChange: (e: any) => setLanguage(e.target.value),
        },
          h('option', { value: 'fr' }, '🇫🇷 Français'),
          h('option', { value: 'en' }, '🇬🇧 English'),
          h('option', { value: 'ar' }, '🇸🇦 العربية'),
          h('option', { value: 'es' }, '🇪🇸 Español'),
        ),
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

      status === 'success' && h('p', { className: 'msg success' }, '✅ 语言偏好已保存'),
      status === 'error' && h('p', { className: 'msg error' }, '❌ 保存失败，请重试'),
    );
  }

  const root = createRoot(container);
  root.render(h(AfpPanel));
  return () => root.unmount();
}