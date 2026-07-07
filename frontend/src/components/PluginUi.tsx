import { useEffect, useRef, useState } from 'react';
import * as React from 'react';
import { createRoot } from 'react-dom/client';
import type { PluginInfo, PluginUiInfo } from '../api';
import { getPluginUi } from '../api';

interface PluginUiProps {
  plugins: PluginInfo[];
}

/**
 * 插件 api 对象——注入给 ESM 插件，提供宿主能力和设置持久化。
 * 使用 localStorage 持久化，无需后端存储端点。
 */
function createPluginApi(pluginId: string) {
  return {
    async getSettings(id: string): Promise<Record<string, unknown> | null> {
      const raw = localStorage.getItem(`plugin-settings-${id}`);
      return raw ? JSON.parse(raw) : null;
    },
    async saveSettings(id: string, settings: Record<string, unknown>): Promise<void> {
      localStorage.setItem(`plugin-settings-${id}`, JSON.stringify(settings));
    },
  };
}

export default function PluginUi({ plugins }: PluginUiProps) {
  const [selectedPlugin, setSelectedPlugin] = useState<PluginInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // 保存当前渲染插件的清理函数
  const unmountRef = useRef<(() => void) | null>(null);
  // 保存上一次的 script 标签引用（仅 Web Component 模式）
  const scriptRef = useRef<HTMLScriptElement | null>(null);
  // 取消标记 ref——避免异步操作在卸载后执行 setState
  const cancelledRef = useRef(false);

  // 筛选出有 UI 的插件
  const uiPlugins = plugins.filter(p => p.has_ui);

  // 当选中插件变化时加载对应的 UI
  useEffect(() => {
    cancelledRef.current = false;

    if (!selectedPlugin) {
      setLoading(false);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    // ---- 清理上一次渲染 ----
    // 1. 调用之前的 unmount 函数
    if (unmountRef.current) {
      unmountRef.current();
      unmountRef.current = null;
    }
    // 2. 移除之前的 script 标签
    if (scriptRef.current && scriptRef.current.parentNode) {
      scriptRef.current.parentNode.removeChild(scriptRef.current);
      scriptRef.current = null;
    }
    // 3. 清空容器
    if (containerRef.current) {
      containerRef.current.innerHTML = '';
    }

    // ---- 加载新 UI ----
    getPluginUi(selectedPlugin.id)
      .then((uiInfo: PluginUiInfo) => {
        if (cancelledRef.current) return;

        if (uiInfo.module_type === 'react') {
          // ────────── React ESM 模式 ──────────
          loadReactPlugin(uiInfo, selectedPlugin.id);
        } else {
          // ────────── Web Component 模式（向后兼容）──────────
          loadWebComponentPlugin(uiInfo, selectedPlugin.id);
        }
      })
      .catch(err => {
        if (!cancelledRef.current) {
          setError(`获取 UI 元数据失败: ${err}`);
          setLoading(false);
        }
      });

    return () => {
      cancelledRef.current = true;
      // 清理 unmount
      if (unmountRef.current) {
        unmountRef.current();
        unmountRef.current = null;
      }
      // 清理 script 标签
      if (scriptRef.current && scriptRef.current.parentNode) {
        scriptRef.current.parentNode.removeChild(scriptRef.current);
        scriptRef.current = null;
      }
    };
  }, [selectedPlugin]);

  /** 加载 React ESM 插件模块 */
  async function loadReactPlugin(uiInfo: PluginUiInfo, pluginId: string) {
    try {
      // import() 走 Vite proxy → 后端 /plugin-files/...
      const mod = await import(/* @vite-ignore */ uiInfo.js_url);
      if (!mod.mount) {
        throw new Error('插件模块没有导出 mount() 函数');
      }

      const container = containerRef.current;
      if (!container) return;

      // 注入 React + createRoot + 宿主 api
      const api = createPluginApi(pluginId);
      const unmount = mod.mount(container, {
        React,
        createRoot,
        pluginId,
        api,
      });

      // 保存清理函数（插件 mount() 应返回 unmount）
      if (typeof unmount === 'function') {
        unmountRef.current = unmount;
      }

      setLoading(false);
    } catch (err) {
      if (cancelledRef.current) return;
      setError(`加载 React 插件失败: ${err}`);
      setLoading(false);
    }
  }

  /** 加载 Web Component 插件（传统方式） */
  function loadWebComponentPlugin(uiInfo: PluginUiInfo, pluginId: string) {
    // 检查该 Web Component 是否已定义
    if (customElements.get(uiInfo.tag_name)) {
      renderWebComponent(uiInfo.tag_name, pluginId);
      setLoading(false);
      return;
    }

    // 动态加载 JS 文件
    const script = document.createElement('script');
    script.src = uiInfo.js_url;
    script.onload = () => {
      if (cancelledRef.current) return;
      customElements.whenDefined(uiInfo.tag_name).then(() => {
        if (cancelledRef.current) return;
        renderWebComponent(uiInfo.tag_name, pluginId);
        setLoading(false);
      });
    };
    script.onerror = () => {
      if (!cancelledRef.current) {
        setError(`加载 Web Component 脚本失败: ${uiInfo.js_url}`);
        setLoading(false);
      }
    };
    document.head.appendChild(script);
    scriptRef.current = script;
  }

  function renderWebComponent(tagName: string, pluginId: string) {
    const container = containerRef.current;
    if (!container) return;

    container.innerHTML = '';
    const element = document.createElement(tagName);
    element.setAttribute('data-plugin-id', pluginId);
    if (selectedPlugin) {
      element.setAttribute('data-agency-name', selectedPlugin.agency);
    }
    container.appendChild(element);
  }

  // ────────── 渲染 ──────────

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