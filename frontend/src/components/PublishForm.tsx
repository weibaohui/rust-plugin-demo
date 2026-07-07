import { useState, type FormEvent } from 'react';
import type { PluginInfo, ArticleResponse } from '../api';

interface PublishFormProps {
  plugins: PluginInfo[];
  onPublish: (pluginId: string, headline: string, body: string) => Promise<ArticleResponse>;
  onRefreshPlugins: () => Promise<PluginInfo[]>;
}

export default function PublishForm({ plugins, onPublish, onRefreshPlugins }: PublishFormProps) {
  const [selectedId, setSelectedId] = useState('');
  const [headline, setHeadline] = useState('');
  const [body, setBody] = useState('');
  const [result, setResult] = useState<ArticleResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!selectedId || !headline.trim() || !body.trim()) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const article = await onPublish(selectedId, headline, body);
      setResult(article);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const presetStories = [
    { headline: 'Global Markets Rally on Tech Earnings', body: 'Stock markets worldwide surged following strong quarterly earnings from major technology companies.' },
    { headline: 'Climate Summit Reaches Historic Agreement', body: 'World leaders have committed to binding emissions targets for the first time.' },
    { headline: 'Central Bank Holds Interest Rates Steady', body: 'The central bank maintained its benchmark interest rate at 4.5% amid mixed economic signals.' },
    { headline: '科学家发现新型量子材料', body: '中国科学院团队在室温超导研究领域取得突破性进展，新型材料在常压下展现了零电阻特性。' },
  ];

  const usePreset = (h: string, b: string) => {
    setHeadline(h);
    setBody(b);
  };

  if (plugins.length === 0) {
    return (
      <div className="section-card">
        <div className="section-header"><h2>📰 发布新闻</h2></div>
        <div className="empty-state">
          <div className="empty-icon">📭</div>
          <div className="empty-text">暂无已加载的插件</div>
          <div className="empty-hint">请先加载插件库，然后返回此页面发布新闻</div>
          <button className="btn btn-primary" onClick={() => onRefreshPlugins()}>
            ⟳ 刷新插件列表
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="section-card">
      <div className="section-header">
        <h2>📰 发布新闻</h2>
        <span className="hint">选择插件机构 → 输入新闻内容 → 查看不同风格</span>
      </div>

      <form className="publish-form" onSubmit={handleSubmit}>
        <div className="form-group">
          <label>选择新闻机构插件</label>
          <select value={selectedId} onChange={e => { setSelectedId(e.target.value); setResult(null); }}>
            <option value="">-- 请选择 --</option>
            {plugins.map(p => (
              <option key={p.id} value={p.id}>{p.agency} ({p.id})</option>
            ))}
          </select>
        </div>

        <div className="form-group">
          <label>预置新闻（点击填充）</label>
          <div className="preset-grid">
            {presetStories.map((s, i) => (
              <button
                key={i}
                type="button"
                className="preset-btn"
                onClick={() => usePreset(s.headline, s.body)}
                title={s.headline}
              >
                {s.headline.length > 30 ? s.headline.slice(0, 30) + '…' : s.headline}
              </button>
            ))}
          </div>
        </div>

        <div className="form-group">
          <label htmlFor="headline">新闻标题</label>
          <input
            id="headline"
            type="text"
            placeholder="输入新闻标题…"
            value={headline}
            onChange={e => setHeadline(e.target.value)}
            required
          />
        </div>

        <div className="form-group">
          <label htmlFor="body">新闻正文</label>
          <textarea
            id="body"
            rows={4}
            placeholder="输入新闻正文…"
            value={body}
            onChange={e => setBody(e.target.value)}
            required
          />
        </div>

        <button
          type="submit"
          className="btn btn-primary"
          disabled={loading || !selectedId}
        >
          {loading ? '⏳ 发布中…' : '📰 发布新闻'}
        </button>
      </form>

      {error && (
        <div className="result-card error">
          <div className="result-header">❌ 发布失败</div>
          <div className="result-body">{error}</div>
        </div>
      )}

      {result && (
        <div className="result-card success">
          <div className="result-header">
            ✅ 发布成功 — {result.agency}
            <span className="result-dateline">📍 {result.dateline}</span>
          </div>
          <div className="result-body">
            <div className="article-preview">
              <div className="article-headline">{result.headline}</div>
              <hr />
              <div className="article-dateline">{result.dateline} —</div>
              <div className="article-body">{result.body}</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}