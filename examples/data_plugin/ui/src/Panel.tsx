/**
 * data_plugin 数据面板 — 展示数据 CRUD 表格。
 * 通过宿主 API 获取数据列表。
 */
import { useState, type ReactNode } from 'react';
import { Card, Table, Tag, Space, App as AntApp } from 'antd';
import type { TableColumnsType } from 'antd';

interface DataRow {
  id: number;
  title: string;
  content: string;
  created_at: string;
}

interface PanelProps {
  pluginId?: string;
}

function PanelContent({ pluginId = 'data_plugin' }: PanelProps): ReactNode {
  const [data] = useState<DataRow[]>([
    { id: 1, title: '示例记录 #1', content: '这是 data_plugin 的示例数据', created_at: '2026-07-12 10:00:00' },
    { id: 2, title: '示例记录 #2', content: 'cron 定时生成的数据', created_at: '2026-07-12 10:01:00' },
  ]);
  const { message } = AntApp.useApp();

  const columns: TableColumnsType<DataRow> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 60 },
    { title: '标题', dataIndex: 'title', key: 'title' },
    { title: '内容', dataIndex: 'content', key: 'content' },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
  ];

  return (
    <Card title="🗄️ Data Plugin 控制面板" style={{ maxWidth: 860 }}>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Card type="inner" title={`数据记录 (插件: ${pluginId})`} size="small">
          <Table
            columns={columns}
            dataSource={data}
            pagination={false}
            size="small"
            rowKey="id"
          />
        </Card>
      </Space>
    </Card>
  );
}

export function Panel(props: PanelProps): ReactNode {
  return (
    <AntApp>
      <PanelContent {...props} />
    </AntApp>
  );
}

export default Panel;