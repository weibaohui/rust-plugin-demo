/**
 * 路透社插件 React 面板
 *
 * 宿主调用 mount(container, deps) 注入 React/ReactDOM/pluginId/api。
 * 使用 React.createElement（JSX 不可用——浏览器直接加载 ESM，无构建步骤）。
 */

export function mount(container, { React, createRoot, pluginId, api }) {
  const { useState, useCallback, useEffect, createElement: h } = React;

  /** 面板主组件 */
  function ReutersPanel() {
    const [dateline, setDateline] = useState('LONDON');
    const [status, setStatus] = useState(null); // 'saving' | 'success' | 'error'
    const [settings, setSettings] = useState(null);

    // 加载已保存的设置
    useEffect(() => {
      api
        .getSettings(pluginId)
        .then((s) => {
          if (s && s.dateline) setDateline(s.dateline);
          setSettings(s);
        })
        .catch(() => {});
    }, []);

    const handleSave = useCallback(async () => {
      setStatus('saving');
      try {
        await api.saveSettings(pluginId, { dateline });
        setStatus('success');
        setTimeout(() => setStatus(null), 2000);
      } catch (_) {
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
          onChange: (e) => setDateline(e.target.value),
          placeholder: '如 LONDON, NEW YORK...',
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
      }, status === 'saving' ? '⏳ 保存中...' : '💾 保存设置'),

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