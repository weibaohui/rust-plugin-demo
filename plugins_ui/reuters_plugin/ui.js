/**
 * 路透社插件 UI — Web Component
 *
 * 在主框架中渲染为 <reuters-plugin-ui> 自定义标签。
 * 通过 data-plugin-id 属性获取插件 ID，可调用后端 API。
 */
class ReutersPluginUI extends HTMLElement {
  constructor() {
    super();
    this._pluginId = this.getAttribute('data-plugin-id') || '';
    this._shadow = this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this._shadow.innerHTML = `
      <style>
        :host { display: block; font-family: system-ui, sans-serif; }
        .card { background: #fff; border: 1px solid #e0e0e0; border-radius: 8px; padding: 20px; margin-bottom: 16px; }
        .card h3 { margin: 0 0 12px; color: #1a1a2e; font-size: 16px; display: flex; align-items: center; gap: 8px; }
        .card h3::before { content: '📰'; }
        .field { margin-bottom: 12px; }
        .field label { display: block; font-size: 13px; color: #555; margin-bottom: 4px; }
        .field input, .field select { width: 100%; padding: 8px 10px; border: 1px solid #ccc; border-radius: 6px; font-size: 14px; box-sizing: border-box; }
        .field select { background: #fff; }
        .actions { display: flex; gap: 8px; }
        .btn { padding: 8px 16px; border: none; border-radius: 6px; cursor: pointer; font-size: 14px; }
        .btn-primary { background: #1a73e8; color: #fff; }
        .btn-primary:hover { background: #1557b0; }
        .btn-secondary { background: #f1f3f4; color: #333; }
        .btn-secondary:hover { background: #e0e0e0; }
        .status { margin-top: 8px; padding: 6px 10px; border-radius: 4px; font-size: 13px; display: none; }
        .status.success { display: block; background: #e6f4ea; color: #1e7e34; }
        .status.error { display: block; background: #fce8e6; color: #c5221f; }
        .preview { margin-top: 12px; padding: 12px; background: #f8f9fa; border-radius: 6px; font-size: 13px; }
        .preview h4 { margin: 0 0 8px; color: #333; }
        .preview .preview-item { color: #666; margin-bottom: 4px; }
      </style>
      <div class="card">
        <h3>路透社插件控制面板</h3>
        <div class="field">
          <label>电头（Dateline）</label>
          <input id="dateline" type="text" value="LONDON" />
        </div>
        <div class="field">
          <label>署名风格</label>
          <select id="style">
            <option value="standard">标准 (Standard)</option>
            <option value="brief">简洁 (Brief)</option>
            <option value="detailed">详细 (Detailed)</option>
          </select>
        </div>
        <div class="actions">
          <button id="saveBtn" class="btn btn-primary">💾 保存设置</button>
          <button id="previewBtn" class="btn btn-secondary">👁 预览效果</button>
        </div>
        <div id="status" class="status"></div>
        <div id="preview" class="preview" style="display:none;">
          <h4>文章预览</h4>
          <div class="preview-item" id="previewHeadline"></div>
          <div class="preview-item" id="previewBody"></div>
        </div>
      </div>
    `;

    this._shadow.getElementById('saveBtn').addEventListener('click', () => this._save());
    this._shadow.getElementById('previewBtn').addEventListener('click', () => this._preview());
  }

  async _save() {
    const status = this._shadow.getElementById('status');
    const dateline = this._shadow.getElementById('dateline').value;
    const style = this._shadow.getElementById('style').value;

    status.className = 'status';

    try {
      const res = await fetch(`/api/plugins/${this._pluginId}/ui/settings`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ dateline, style }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      status.className = 'status success';
      status.textContent = '✅ 设置已保存到服务器';
    } catch (err) {
      status.className = 'status error';
      status.textContent = `❌ 保存失败: ${err.message}`;
    }
  }

  async _preview() {
    const preview = this._shadow.getElementById('preview');
    const headlineEl = this._shadow.getElementById('previewHeadline');
    const bodyEl = this._shadow.getElementById('previewBody');

    try {
      const res = await fetch(`/api/plugins/${this._pluginId}/publish`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          headline: 'Plugin UI 测试标题',
          body: '这是一条通过 Web Component 面板发布的测试新闻。',
        }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      headlineEl.textContent = `标题: ${data.headline}`;
      bodyEl.textContent = `正文: ${data.body.substring(0, 100)}...`;
      preview.style.display = 'block';
    } catch (err) {
      const status = this._shadow.getElementById('status');
      status.className = 'status error';
      status.textContent = `❌ 预览失败: ${err.message}`;
    }
  }
}

customElements.define('reuters-plugin-ui', ReutersPluginUI);