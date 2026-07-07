/**
 * 法新社插件 React 面板
 *
 * 宿主调用 mount(container, deps) 注入 React/ReactDOM/pluginId/api。
 * 使用 React.createElement（JSX 不可用——浏览器直接加载 ESM，无构建步骤）。
 */

export function mount(container, { React, createRoot, pluginId, api }) {
  const { useState, useCallback, useEffect, createElement: h } = React;

  /** 面板主组件 */
  function AfpPanel() {
    const [language, setLanguage] = useState('fr');
    const [status, setStatus] = useState(null);
    const [settings, setSettings] = useState(null);

    // 加载已保存的设置
    useEffect(() => {
      api
        .getSettings(pluginId)
        .then((s) => {
          if (s && s.language) setLanguage(s.language);
          setSettings(s);
        })
        .catch(() => {});
    }, []);

    const handleSave = useCallback(async () => {
      setStatus('saving');
      try {
        await api.saveSettings(pluginId, { language });
        setStatus('success');
        setTimeout(() => setStatus(null), 2000);
      } catch (_) {
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
          onChange: (e) => setLanguage(e.target.value),
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
      }, status === 'saving' ? '⏳ 保存中...' : '💾 保存设置'),

      status === 'success' && h('p', { className: 'msg success' }, '✅ 语言偏好已保存'),
      status === 'error' && h('p', { className: 'msg error' }, '❌ 保存失败，请重试'),
    );
  }

  const root = createRoot(container);
  root.render(h(AfpPanel));
  return () => root.unmount();
}