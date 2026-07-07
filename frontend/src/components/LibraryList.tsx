import type { LibraryInfo, PluginInfo } from '../api';

interface LibraryListProps {
  libraries: LibraryInfo[];
  onLoad: (name: string) => Promise<PluginInfo[]>;
  onRefresh: () => Promise<LibraryInfo[]>;
  plugins: PluginInfo[];
}

export default function LibraryList({ libraries, onLoad, onRefresh, plugins }: LibraryListProps) {
  const knownPlugins = ['reuters_plugin', 'afp_plugin'];
  const dylibExt = 'dylib'; // macOS 动态库扩展名

  return (
    <div className="section-card">
      <div className="section-header">
        <h2>📦 可用的插件库</h2>
        <div className="header-actions">
          <button className="btn btn-secondary" onClick={() => onRefresh()}>
            ⟳ 重新扫描
          </button>
        </div>
      </div>

      <div className="info-box">
        <strong>📖 工作原理</strong>
        <p>
          编译后的插件库（<code>libreuters_plugin.{dylibExt}</code> 等）位于 <code>target/debug/</code> 目录下。
          扫描功能会自动查找这些文件。每个插件库内部定义了一个 <code>register_plugins</code> 函数，
          通过 <code>PluginRegistrar</code> 向框架注册 <code>NewsAgencyPlugin</code> 实例。
        </p>
        <p>已预置的插件库：<strong>路透社 (Reuters)</strong>、<strong>法新社 (AFP)</strong></p>
      </div>

      {libraries.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">🔍</div>
          <div className="empty-text">未扫描到插件库文件</div>
          <div className="empty-hint">请先执行 <code>cargo build -p reuters_plugin -p afp_plugin</code> 编译插件库</div>
        </div>
      ) : (
        <div className="library-grid">
          {libraries
            .filter(lib => knownPlugins.includes(lib.name))
            .map(lib => {
              const shortName = lib.name;
              const agencyName =
                shortName === 'reuters_plugin' ? 'Reuters 路透社' :
                shortName === 'afp_plugin' ? 'AFP 法新社' :
                shortName;
              return (
                <div key={lib.path} className={`library-card ${lib.loaded ? 'loaded' : ''}`}>
                  <div className="library-card-header">
                    <span className="lib-icon">{shortName === 'reuters_plugin' ? '📰' : '🇫🇷'}</span>
                    <div className="lib-info">
                      <span className="lib-name">{agencyName}</span>
                      <code className="lib-filename">{lib.file_name}</code>
                    </div>
                    <span className={`status-badge ${lib.loaded ? 'status-loaded' : 'status-available'}`}>
                      {lib.loaded ? `已加载 (${lib.plugin_count} 插件)` : '未加载'}
                    </span>
                  </div>
                  <div className="library-card-body">
                    <div className="field">
                      <span className="field-label">路径</span>
                      <code className="field-value">{lib.path}</code>
                    </div>
                    <div className="field">
                      <span className="field-label">注册函数</span>
                      <code className="field-value">register_plugins</code>
                    </div>
                  </div>
                  <div className="library-card-footer">
                    <button
                      className={`btn ${lib.loaded ? 'btn-disabled' : 'btn-primary'} btn-sm`}
                      disabled={lib.loaded}
                      onClick={() => onLoad(shortName)}
                    >
                      {lib.loaded ? '✅ 已加载' : '📥 加载插件'}
                    </button>
                  </div>
                </div>
              );
            })}
        </div>
      )}
    </div>
  );
}