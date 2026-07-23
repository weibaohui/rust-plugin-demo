import { useState } from 'react';
import { Button, Card, Form, Input, Typography, Alert, Space } from 'antd';
import { UserOutlined, LockOutlined } from '@ant-design/icons';
import { login } from './auth';

const { Title, Text } = Typography;

export default function LoginPage({ onSuccess }: { onSuccess: () => void }) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const onFinish = async (values: { username: string; password: string }) => {
    setLoading(true);
    setError(null);
    try {
      await login(values.username, values.password);
      onSuccess();
    } catch (e) {
      setError(e instanceof Error ? e.message : '登录失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{
      minHeight: '100vh',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'var(--color-bg-layout, #f5f5f5)',
    }}>
      <Card style={{ width: 400, boxShadow: '0 4px 12px rgba(0,0,0,0.1)' }}>
        <Space direction="vertical" size="large" style={{ width: '100%', textAlign: 'center' }}>
          <div>
            <Title level={2} style={{ marginBottom: 8 }}>plugkit</Title>
            <Text type="secondary">插件管理框架</Text>
          </div>

          {error && <Alert message={error} type="error" showIcon />}

          <Form
            name="login"
            onFinish={onFinish}
            autoComplete="off"
            size="large"
            initialValues={{ username: 'admin' }}
          >
            <Form.Item
              name="username"
              rules={[{ required: true, message: '请输入用户名' }]}
            >
              <Input
                prefix={<UserOutlined />}
                placeholder="用户名"
                defaultValue="admin"
              />
            </Form.Item>

            <Form.Item
              name="password"
              rules={[{ required: true, message: '请输入密码' }]}
            >
              <Input.Password
                prefix={<LockOutlined />}
                placeholder="密码"
              />
            </Form.Item>

            <Form.Item>
              <Button type="primary" htmlType="submit" loading={loading} block>
                登录
              </Button>
            </Form.Item>
          </Form>

          <Text type="secondary" style={{ fontSize: 12 }}>
            默认用户名: admin，密码见服务端控制台或环境变量 PLUGKIT_ADMIN_PASSWORD
          </Text>
        </Space>
      </Card>
    </div>
  );
}
