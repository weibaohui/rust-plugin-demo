import type { LibraryInfo, PluginInfo } from '../api';

interface LibraryListProps {
  libraries: LibraryInfo[];
  onLoad: (name: string) => Promise<PluginInfo[]>;
  onRefresh: () => Promise<LibraryInfo[]>;
}

export default function LibraryList({ libraries, onLoad, onRefresh }: LibraryListProps) {
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
          编译后的插件库（<code>.so</code> / <code>.dylib</code> / <code>.dll</code>）位于 <code>target/debug/</code>{' '}
          或 <code>target/release/</code> 目录下。扫描功能会自动查找这些文件。
          每个插件库内部定义了一个 <code>register_plugins</code> 函数，
          通过 <code>PluginRegistrar</code> 向框架注册插件实例。
        </p>
      </div>

      {libraries.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">🔍</div>
          <div className="empty-text">未扫描到插件库文件</div>
          <div className="empty-hint">
            请先编译插件库（<code>cargo build -p &lt;plugin_name&gt;</code>），然后点击「重新扫描」
          </div>
        </div>
      ) : (
        <div className="library-grid">
          {libraries.map(lib => {
            const shortName = lib.name;
            return (
              <div key={lib.path} className={`library-card ${lib.loaded ? 'loaded' : ''}`}>
                <div className="library-card-header">
                  <span className="lib-icon">📦</span>
                  <div className="lib-info">
                    <span className="lib-name">{shortName}</span>
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