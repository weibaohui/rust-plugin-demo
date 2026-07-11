import { useMemo, useEffect, useRef } from 'react';
import type { PluginInfo } from '../api';
import { qiankunEntryFor, registerLoadedPlugins } from '../micro';

interface PluginUiProps {
  plugin: PluginInfo;
}

/**
 * qiankun 微前端容器。
 *
 * `<div id="plugin-mount">` 是 qiankun 约定的挂载点；
 * 子应用的 bootstrap/mount/unmount 由 qiankun 自身驱动。
 * 容器渲染到 DOM 后再注册子应用，确保 qiankun 能找到挂载点。
 */
export function PluginUi({ plugin }: PluginUiProps) {
  const entry = useMemo(() => qiankunEntryFor(plugin, window.location.origin), [plugin]);
  const mountedRef = useRef(false);

  useEffect(() => {
    if (!entry || mountedRef.current) return;
    mountedRef.current = true;
    const origin = window.location.origin;
    const enriched = [{ ...plugin, qiankunEntry: entry }];
    registerLoadedPlugins(enriched, origin).catch(err => {
      console.error('qiankun registerLoadedPlugins failed', err);
    });
  }, [entry, plugin]);

  return (
    <div className="plugin-ui-mount">
      <div className="plugin-ui-header">
        <span className="plugin-ui-icon">🔌</span>
        <span className="plugin-ui-name">{plugin.name}</span>
        <span className="plugin-ui-id">
          <code>{plugin.id}</code>
        </span>
      </div>

      {!entry ? (
        <div className="empty-state">
          <div className="empty-icon">⚠️</div>
          <h3>该插件没有嵌入 UI</h3>
          <p>插件未声明 <code>ui_base_dir</code>，没有可用的 qiankun 子应用入口。</p>
        </div>
      ) : (
        <div id="plugin-mount" className="web-component-container" />
      )}
    </div>
  );
}

export default PluginUi;