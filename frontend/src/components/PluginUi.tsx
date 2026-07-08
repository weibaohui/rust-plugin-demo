import { useMemo } from 'react';
import type { PluginInfo } from '../api';
import { qiankunEntryFor } from '../micro';

interface PluginUiProps {
  plugin: PluginInfo;
}

/**
 * qiankun 微前端容器。
 *
 * `<div id="plugin-mount">` 是 qiankun 约定的挂载点；
 * 子应用的 bootstrap/mount/unmount 由 qiankun 自身驱动，宿主不调用任何插件代码。
 */
export function PluginUi({ plugin }: PluginUiProps) {
  const entry = useMemo(() => qiankunEntryFor(plugin, window.location.origin), [plugin]);

  return (
    <div className="plugin-ui-mount">
      <div className="plugin-ui-header">
        <span className="plugin-ui-icon">
          {plugin.agency === 'Reuters'
            ? '📰'
            : plugin.agency === 'Agence France-Presse'
            ? '📡'
            : '🔌'}
        </span>
        <span className="plugin-ui-name">{plugin.agency}</span>
        <span className="plugin-ui-id">
          <code>{plugin.id}</code>
        </span>
      </div>

      {!entry ? (
        <div className="empty-state">
          <div className="empty-icon">⚠️</div>
          <h3>该插件没有可用的 qiankun 入口</h3>
          <p>请确认已执行 <code>npm run build</code> 生成 <code>{plugin.id}/ui/dist/index.html</code>。</p>
        </div>
      ) : (
        <div id="plugin-mount" className="web-component-container" />
      )}
    </div>
  );
}

export default PluginUi;
