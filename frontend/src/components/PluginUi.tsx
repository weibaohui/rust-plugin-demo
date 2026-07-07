import { useEffect, useRef, useState } from 'react';
import type { PluginInfo } from '../api';
import { getPluginUi } from '../api';

interface PluginUiProps {
  plugins: PluginInfo[];
}

export default function PluginUi({ plugins }: PluginUiProps) {
  const [selectedPlugin, setSelectedPlugin] = useState<PluginInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // 筛选出有 UI 的插件
  const uiPlugins = plugins.filter(p => p.has_ui);

  // 当选中插件变化时加载对应的 Web Component
  useEffect(() => {
    if (!selectedPlugin) return;

    setLoading(true);
    setError(null);

    getPluginUi(selectedPlugin.id)
      .then(uiInfo => {
        // 检查该 Web Component 是否已定义
        if (customElements.get(uiInfo.tag_name)) {
          // 已定义，直接渲染
          renderComponent(uiInfo.tag_name, selectedPlugin.id);
          setLoading(false);
          return;
        }

        // 动态加载 JS 文件
        const script = document.createElement('script');
        script.src = uiInfo.js_url;
        script.onload = () => {
          // 等待 Web Component 注册
          customElements.whenDefined(uiInfo.tag_name).then(() => {
            renderComponent(uiInfo.tag_name, selectedPlugin.id);
            setLoading(false);
          });
        };
        script.onerror = () => {
          setError(`加载插件 UI 脚本失败: ${uiInfo.js_url}`);
          setLoading(false);
        };
        document.head.appendChild(script);

        return () => {
          // 清理：移除 script 标签
          if (script.parentNode) {
            script.parentNode.removeChild(script);
          }
        };
      })
      .catch(err => {
        setError(`获取 UI 元数据失败: ${err}`);
        setLoading(false);
      });
  }, [selectedPlugin]);

  function renderComponent(tagName: string, pluginId: string) {
    const container = containerRef.current;
    if (!container) return;

    // 清空容器
    container.innerHTML = '';

    // 创建自定义元素并设置属性
    const element = document.createElement(tagName);
    element.setAttribute('data-plugin-id', pluginId);
    if (selectedPlugin) {
      element.setAttribute('data-agency-name', selectedPlugin.agency);
    }
    container.appendChild(element);
  }

  if (uiPlugins.length === 0) {
    return (
      <div className="empty-state">
        <div className="empty-icon">🧩</div>
        <h3>没有可用的插件界面</h3>
        <p>请先在「插件库管理」中加载带有 UI 的插件（如路透社、法新社）。</p>
      </div>
    );
  }

  return (
    <div className="plugin-ui-container">
      {/* 插件选择器 */}
      <div className="plugin-selector">
        {uiPlugins.map(p => (
          <button
            key={p.id}
            className={`plugin-select-btn ${selectedPlugin?.id === p.id ? 'active' : ''}`}
            onClick={() => setSelectedPlugin(p)}
          >
            <span className="plugin-select-icon">
              {p.agency === 'Reuters' ? '📰' : p.agency === 'Agence France-Presse' ? '📡' : '🔌'}
            </span>
            <span className="plugin-select-name">{p.agency}</span>
          </button>
        ))}
      </div>

      {/* 插件 UI 渲染区 */}
      <div className="plugin-ui-render-area">
        {!selectedPlugin && (
          <div className="empty-state">
            <div className="empty-icon">👈</div>
            <h3>请选择一个插件</h3>
            <p>点击左侧的插件按钮，其自定义界面将在此处加载。</p>
          </div>
        )}

        {loading && (
          <div className="empty-state">
            <div className="empty-icon">⏳</div>
            <h3>正在加载插件 UI...</h3>
          </div>
        )}

        {error && (
          <div className="error-message" onClick={() => setError(null)}>
            {error}
          </div>
        )}

        <div ref={containerRef} className="web-component-container" />
      </div>
    </div>
  );
}