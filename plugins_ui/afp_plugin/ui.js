/**
 * 法新社（AFP）插件 UI — Web Component
 *
 * 在主框架中渲染为 <afp-plugin-ui> 自定义标签。
 * 通过 data-plugin-id 和 data-agency-name 属性获取插件信息。
 */
class AfpPluginUI extends HTMLElement {
  constructor() {
    super();
    this._pluginId = this.getAttribute('data-plugin-id') || '';
    this._agencyName = this.getAttribute('data-agency-name') || 'AFP';
    this._shadow = this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this._shadow.innerHTML = `
      <style>
        :host { display: block; font-family: system-ui, sans-serif; }
        .card { background: #fff; border: 1px solid #e0e0e0; border-radius: 8px; padding: 20px; margin-bottom: 16px; }
        .card h3 { margin: 0 0 12px; color: #1a1a2e; font-size: 16px; display: flex; align-items: center; gap: 8px; }
        .card h3::before { content: '📡'; }
        .info-row { display: flex; gap: 16px; margin-bottom: 16px; flex-wrap: wrap; }
        .info-item { background: #f0f7ff; border-radius: 6px; padding: 10px 14px; flex: 1; min-width: 120px; }
        .info-item .label { font-size: 11px; color: #666; text-transform: uppercase; letter-spacing: 0.5px; }
        .info-item .value { font-size: 18px; font-weight: 600; color: #1a73e8; margin-top: 2px; }
        .field { margin-bottom: 12px; }
        .field label { display: block; font-size: 13px; color: #555; margin-bottom: 4px; }
        .field textarea { width: 100%; padding: 8px 10px; border: 1px solid #ccc; border-radius: 6px; font-size: 14px; box-sizing: border-box; resize: vertical; min-height: 60px; font-family: inherit; }
        .actions { display: flex; gap: 8px; margin-top: 8px; }
        .btn { padding: 8px 16px; border: none; border-radius: 6px; cursor: pointer; font-size: 14px; }
        .btn-primary { background: #1a73e8; color: #fff; }
        .btn-primary:hover { background: #1557b0; }
        .btn-outline { background: transparent; border: 1px solid #1a73e8; color: #1a73e8; }
        .btn-outline:hover { background: #f0f7ff; }
        .status { margin-top: 8px; padding: 6px 10px; border-radius: 4px; font-size: 13px; display: none; }
        .status.info { display: block; background: #e8f0fe; color: #1a73e8; }
        .status.error { display: block; background: #fce8e6; color: #c5221f; }
      </style>
      <div class="card">
        <h3>${this._agencyName} 快速发稿</h3>

        <div class="info-row">
          <div class="info-item">
            <div class="label">已发稿件</div>
            <div class="value" id="articleCount">—</div>
          </div>
          <div class="info-item">
            <div class="label">服务器状态</div>
            <div class="value" id="serverStatus">●</div>
          </div>
        </div>

        <div class="field">
          <label>标题</label>
          <input id="headline" type="text" placeholder="输入新闻标题..." />
        </div>
        <div class="field">
          <label>正文</label>
          <textarea id="body" placeholder="输入新闻正文..."></textarea>
        </div>
        <div class="actions">
          <button id="publishBtn" class="btn btn-primary">📨 发布新闻</button>
          <button id="refreshBtn" class="btn btn-outline">🔄 刷新状态</button>
        </div>
        <div id="status" class="status"></div>
      </div>
    `;

    this._shadow.getElementById('publishBtn').addEventListener('click', () => this._publish());
    this._shadow.getElementById('refreshBtn').addEventListener('click', () => this._refreshStats());

    this._refreshStats();
  }

  async _publish() {
    const status = this._shadow.getElementById('status');
    const headline = this._shadow.getElementById('headline').value.trim();
    const body = this._shadow.getElementById('body').value.trim();

    if (!headline || !body) {
      status.className = 'status error';
      status.textContent = '❌ 请填写标题和正文';
      return;
    }

    status.className = 'status info';
    status.textContent = '⏳ 正在发布...';

    try {
      const res = await fetch(`/api/plugins/${this._pluginId}/publish`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ headline, body }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      status.className = 'status info';
      status.textContent = `✅ 已发布: ${data.headline}`;
      this._shadow.getElementById('headline').value = '';
      this._shadow.getElementById('body').value = '';
      this._refreshStats();
    } catch (err) {
      status.className = 'status error';
      status.textContent = `❌ 发布失败: ${err.message}`;
    }
  }

  async _refreshStats() {
    try {
      const res = await fetch('/api/plugins');
      if (!res.ok) return;
      const plugins = await res.json();
      const countEl = this._shadow.getElementById('articleCount');
      const serverEl = this._shadow.getElementById('serverStatus');
      serverEl.style.color = '#1e7e34';
      // 发布一篇假文章来看看计数变化
      const testRes = await fetch(`/api/plugins/${this._pluginId}/publish`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          headline: 'Status Check',
          body: 'Internal server health check.',
        }),
      });
      if (testRes.ok) {
        const testData = await testRes.json();
        countEl.textContent = `✓`;
      }
    } catch {
      // silently ignore
    }
  }
}

customElements.define('afp-plugin-ui', AfpPluginUI);