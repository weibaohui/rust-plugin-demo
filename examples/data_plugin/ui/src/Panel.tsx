/**
 * data_plugin 数据面板 — 完整 CRUD 操作。
 * 通过插件自定义路由 /plugin-api/<pluginId>/items 实现增删改查。
 */
import { useState, useEffect, useCallback, useMemo, type ReactNode } from 'react';
import { Card, Table, Tag, Button, Space, Modal, Form, Input, message, Popconfirm } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
import type { TableColumnsType } from 'antd';

interface DataRow {
  id: number;
  title: string;
  content: string;
  created_at: string;
  created_by: string;
  updated_by: string;
  remark: string;
}

interface PanelProps {
  pluginId?: string;
}

function PanelContent({ pluginId = 'data_plugin.DataPlugin' }: PanelProps): ReactNode {
  const apiBase = useMemo(() => `/plugin-api/${pluginId}/items`, [pluginId]);
  const whoamiBase = useMemo(() => `/plugin-api/${pluginId}/whoami`, [pluginId]);

  // 带 token 的 fetch 封装
  const authedFetch = useCallback((url: string, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    const token = localStorage.getItem('plugkit_token') || (window as any).__plugkit_token__;
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }
    return fetch(url, { ...init, headers });
  }, []);

  const [data, setData] = useState<DataRow[]>([]);
  const [username, setUsername] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<DataRow | null>(null);
  const [form] = Form.useForm();

  const fetchWhoami = useCallback(async () => {
    try {
      const res = await authedFetch(whoamiBase);
      if (res.ok) {
        const { username: u } = await res.json();
        setUsername(u);
      }
    } catch { /* ignore */ }
  }, [authedFetch, whoamiBase]);

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const res = await authedFetch(apiBase);
      if (res.ok) {
        const list: DataRow[] = await res.json();
        setData(list);
      }
    } catch {
      message.error('获取数据失败');
    } finally {
      setLoading(false);
    }
  }, [authedFetch, apiBase]);

  useEffect(() => { fetchWhoami(); fetchData(); }, [fetchWhoami, fetchData]);

  const handleAdd = () => {
    setEditing(null);
    form.resetFields();
    setModalOpen(true);
  };

  const handleEdit = (record: DataRow) => {
    setEditing(record);
    form.setFieldsValue({ title: record.title, content: record.content, remark: record.remark });
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const url = editing ? `${apiBase}/${editing.id}` : apiBase;
      const method = editing ? 'PUT' : 'POST';
      const res = await authedFetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(values),
      });
      if (res.ok) {
        message.success(editing ? '更新成功' : '创建成功');
        setModalOpen(false);
        fetchData();
      } else {
        const err = await res.json().catch(() => ({ message: '操作失败' }));
        message.error(err.message);
      }
    } catch {
      // form validation error
    }
  };

  const handleDelete = async (id: number) => {
    try {
      const res = await authedFetch(`${apiBase}/${id}`, { method: 'DELETE' });
      if (res.ok) {
        message.success('删除成功');
        fetchData();
      } else {
        message.error('删除失败');
      }
    } catch {
      message.error('删除失败');
    }
  };

  const columns: TableColumnsType<DataRow> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 60 },
    { title: '标题', dataIndex: 'title', key: 'title' },
    { title: '内容', dataIndex: 'content', key: 'content', ellipsis: true },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 160,
      render: (v: string) => <Tag color="blue">{v}</Tag>,
    },
    {
      title: '创建人',
      dataIndex: 'created_by',
      key: 'created_by',
      width: 90,
      render: (v: string) => v || '-',
    },
    {
      title: '编辑人',
      dataIndex: 'updated_by',
      key: 'updated_by',
      width: 90,
      render: (v: string) => v || '-',
    },
    {
      title: '备注',
      dataIndex: 'remark',
      key: 'remark',
      ellipsis: true,
      render: (v: string) => v || '-',
    },
    {
      title: '操作',
      key: 'action',
      width: 140,
      render: (_: unknown, record: DataRow) => (
        <Space>
          <Button size="small" icon={<EditOutlined />} onClick={() => handleEdit(record)}>编辑</Button>
          <Popconfirm title="确定删除?" onConfirm={() => handleDelete(record.id)} okText="是" cancelText="否">
            <Button size="small" danger icon={<DeleteOutlined />}>删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <Card title={<span>🗄️ Data Plugin 控制面板</span>} style={{ maxWidth: 960 }}>
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>{username ? `${username} 你好 — ` : ''}数据记录 (插件: {pluginId})</span>
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>新增</Button>
        </div>
        <Table
          columns={columns}
          dataSource={data}
          pagination={false}
          size="small"
          rowKey="id"
          loading={loading}
        />
      </Space>
      <Modal
        title={editing ? '编辑记录' : '新增记录'}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
        okText="保存"
        cancelText="取消"
      >
        <Form form={form} layout="vertical">
          <Form.Item name="title" label="标题" rules={[{ required: true, message: '请输入标题' }]}>
            <Input placeholder="输入标题" />
          </Form.Item>
          <Form.Item name="content" label="内容" rules={[{ required: true, message: '请输入内容' }]}>
            <Input.TextArea rows={4} placeholder="输入内容" />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={2} placeholder="备注信息" />
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  );
}

export function Panel(props: PanelProps): ReactNode {
  return (
    <PanelContent {...props} />
  );
}

export default Panel;