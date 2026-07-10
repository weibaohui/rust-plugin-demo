/**
 * 法新社插件 React 面板（Ant Design 版）。
 *
 * 演示控件:Card / Form / Select / Input / Button / Table / Tag,
 * 体现主框架承载 antd 子应用。localStorage 持久化设置。
 */
import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { Card, Form, Select, Input, Button, Table, Tag, Space, App as AntApp } from 'antd';
import type { TableColumnsType } from 'antd';

interface AfpPanelProps {
  pluginId?: string;
}

interface DemoRow {
  key: string;
  lang: string;
  label: string;
  region: string;
}

const LANGUAGES = [
  { value: 'fr', label: '🇫🇷 Français' },
  { value: 'en', label: '🇬🇧 English' },
  { value: 'ar', label: '🇸🇦 العربية' },
  { value: 'es', label: '🇪🇸 Español' },
];

const DEMO_DATA: DemoRow[] = [
  { key: '1', lang: 'fr', label: 'Français', region: 'Paris' },
  { key: '2', lang: 'en', label: 'English', region: 'London' },
  { key: '3', lang: 'ar', label: 'العربية', region: 'Cairo' },
  { key: '4', lang: 'es', label: 'Español', region: 'Madrid' },
];

function PanelContent({ pluginId = 'afp_plugin' }: AfpPanelProps): ReactNode {
  const [language, setLanguage] = useState('fr');
  const [note, setNote] = useState('');
  const { message } = AntApp.useApp();

  useEffect(() => {
    const raw = localStorage.getItem(`plugin-settings-${pluginId}`);
    if (raw) {
      try {
        const s = JSON.parse(raw) as { language?: string; note?: string };
        if (typeof s.language === 'string') setLanguage(s.language);
        if (typeof s.note === 'string') setNote(s.note);
      } catch {
        /* ignore malformed settings */
      }
    }
  }, [pluginId]);

  const handleSave = useCallback(() => {
    try {
      localStorage.setItem(`plugin-settings-${pluginId}`, JSON.stringify({ language, note }));
      message.success('语言偏好已保存');
    } catch {
      message.error('保存失败,请重试');
    }
  }, [language, note, pluginId, message]);

  const columns: TableColumnsType<DemoRow> = [
    { title: '语言', dataIndex: 'label', key: 'label' },
    {
      title: '代码',
      dataIndex: 'lang',
      key: 'lang',
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
    { title: '地区', dataIndex: 'region', key: 'region' },
  ];

  return (
    <Card title="📡 法新社控制面板" style={{ maxWidth: 720 }}>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Form layout="vertical">
          <Form.Item label="默认语言">
            <Select value={language} onChange={setLanguage} options={LANGUAGES} />
          </Form.Item>
          <Form.Item label="备注">
            <Input
              value={note}
              onChange={e => setNote(e.target.value)}
              placeholder="输入备注信息"
              allowClear
            />
          </Form.Item>
          <Form.Item label="当前插件 ID">
            <Input value={pluginId} disabled />
          </Form.Item>
          <Button type="primary" onClick={handleSave}>
            💾 保存设置
          </Button>
        </Form>
        <Card type="inner" title="语言演示数据" size="small">
          <Table
            columns={columns}
            dataSource={DEMO_DATA}
            pagination={false}
            size="small"
            rowKey="key"
          />
        </Card>
      </Space>
    </Card>
  );
}

export function AfpPanel(props: AfpPanelProps): ReactNode {
  return (
    <AntApp>
      <PanelContent {...props} />
    </AntApp>
  );
}

export default AfpPanel;
