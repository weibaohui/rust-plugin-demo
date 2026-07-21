/**
 * 认证模块 — token 存储、登录/登出 API、fetch 封装（带 Authorization 头 + 401 拦截）。
 */

const TOKEN_KEY = 'plugkit_token';
const USER_KEY = 'plugkit_user';

export interface UserInfo {
  id: string;
  username: string;
  roles: string[];
  permissions: string[];
}

export interface LoginResponse {
  token: string;
  user: UserInfo;
  expires_at: string;
}

/** 获取存储的 token。 */
export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

/** 获取存储的用户信息。 */
export function getUser(): UserInfo | null {
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try { return JSON.parse(raw); } catch { return null; }
}

/** 是否已登录。 */
export function isAuthenticated(): boolean {
  return !!getToken();
}

/** 登录。 */
export async function login(username: string, password: string): Promise<LoginResponse> {
  const res = await fetch('/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  });
  if (!res.ok) {
    const err = await res.json();
    throw new Error(err.message || '登录失败');
  }
  const data: LoginResponse = await res.json();
  localStorage.setItem(TOKEN_KEY, data.token);
  localStorage.setItem(USER_KEY, JSON.stringify(data.user));
  return data;
}

/** 登出。 */
export async function logout(): Promise<void> {
  const token = getToken();
  if (token) {
    await fetch('/auth/logout', {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}` },
    });
  }
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
}

/** 撤销所有会话。 */
export async function revokeAll(): Promise<void> {
  const token = getToken();
  if (!token) return;
  await fetch('/auth/revoke-all', {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}` },
  });
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
}

/** 获取当前用户信息（从服务器刷新）。 */
export async function fetchMe(): Promise<UserInfo | null> {
  const token = getToken();
  if (!token) return null;
  const res = await fetch('/auth/me', {
    headers: { 'Authorization': `Bearer ${token}` },
  });
  if (!res.ok) {
    if (res.status === 401) {
      localStorage.removeItem(TOKEN_KEY);
      localStorage.removeItem(USER_KEY);
      return null;
    }
    throw new Error('获取用户信息失败');
  }
  return res.json();
}

/** 封装 fetch，自动带 Authorization 头，401 时跳转登录页。 */
export async function authFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const token = getToken();
  const headers = new Headers(init?.headers);
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  const res = await fetch(input, { ...init, headers });
  if (res.status === 401) {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
    window.location.href = '/login';
    throw new Error('未授权');
  }
  return res;
}
