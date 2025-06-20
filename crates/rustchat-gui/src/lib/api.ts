// API 服务
import axios, { type AxiosResponse } from 'axios';
import type { User, Room, Message, AuthTokens, ApiResponse, FriendRequest } from './types';

const API_BASE_URL = 'http://127.0.0.1:8080/api';

// 创建 axios 实例
const api = axios.create({
  baseURL: API_BASE_URL,
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
});

// 请求拦截器 - 添加认证令牌
api.interceptors.request.use(
  (config) => {
    const token = localStorage.getItem('access_token');
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error) => {
    return Promise.reject(error);
  }
);

// 响应拦截器 - 处理令牌刷新
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;
    
    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true;
      
      try {
        const refreshToken = localStorage.getItem('refresh_token');
        if (refreshToken) {
          const response = await refreshAccessToken(refreshToken);
          localStorage.setItem('access_token', response.access_token);
          
          // 重试原始请求
          originalRequest.headers.Authorization = `Bearer ${response.access_token}`;
          return api(originalRequest);
        }
      } catch (refreshError) {
        // 刷新失败，清除令牌并跳转到登录页
        clearTokens();
        window.location.href = '/login';
      }
    }
    
    return Promise.reject(error);
  }
);

// 认证相关 API
export const authApi = {
  async register(email: string, password: string): Promise<ApiResponse<{ message: string }>> {
    const response: AxiosResponse<any> = await api.post('/auth/register', {
      email,
      password,
    });
    
    // 适配后端响应格式
    if (response.data.success) {
      return { data: { message: response.data.message }, message: response.data.message };
    } else {
      return { error: response.data.message || 'Registration failed' };
    }
  },  async login(email: string, password: string): Promise<ApiResponse<AuthTokens & { user: User }>> {
    const response: AxiosResponse<any> = await api.post('/auth/login', {
      email,
      password,
    });
    
    // 适配后端响应格式
    if (response.data.success && response.data.account && response.data.account.tokens) {
      const account = response.data.account;
      const tokens = account.tokens;
      const authTokens: AuthTokens = {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        token_type: tokens.token_type,
        expires_in: tokens.expires_in,
      };
      
      const user: User = {
        id: account.account_id,
        email: account.email,
        username: account.display_name || undefined,
        verified: account.email_verified,
        created_at: account.created_at,
      };
      
      return { 
        data: { ...authTokens, user }, 
        message: response.data.message 
      };
    } else {
      return { error: response.data.message || 'Login failed' };
    }
  },

  async verifyEmail(email: string, code: string): Promise<ApiResponse<{ message: string }>> {
    const response: AxiosResponse<any> = await api.post('/auth/verify-email', {
      email,
      code,
    });
    
    // 适配后端响应格式
    if (response.data.success) {
      return { data: { message: response.data.message }, message: response.data.message };
    } else {
      return { error: response.data.message || 'Email verification failed' };
    }
  },

  async resendCode(email: string): Promise<ApiResponse<{ message: string }>> {
    const response: AxiosResponse<any> = await api.post('/auth/resend-code', {
      email,
    });
    
    // 适配后端响应格式
    if (response.data.success) {
      return { data: { message: response.data.message }, message: response.data.message };
    } else {
      return { error: response.data.message || 'Failed to resend code' };
    }
  },
  async refreshToken(refreshToken: string): Promise<AuthTokens> {
    const response: AxiosResponse<any> = await api.post('/auth/refresh', {
      refresh_token: refreshToken,
    });
    
    // 适配后端响应格式
    if (response.data.success && response.data.tokens) {
      return response.data.tokens;
    } else {
      throw new Error(response.data.message || 'Failed to refresh token');
    }
  },

  async logout(): Promise<void> {
    await api.post('/auth/logout');
  },

  async getCurrentUser(): Promise<ApiResponse<User>> {
    try {
      const response: AxiosResponse<any> = await api.get('/auth/me');
      
      if (response.data.success && response.data.account) {
        const account = response.data.account;
        const user: User = {
          id: account.account_id,
          email: account.email,
          username: account.display_name || undefined,
          verified: account.email_verified,
          created_at: account.created_at,
        };
        return { data: user, message: response.data.message };
      } else {
        return { error: response.data.message || 'Failed to get user info' };
      }
    } catch (error: any) {
      return { error: error.response?.data?.message || 'Failed to get user info' };
    }
  },

  // 从后端账户响应中提取用户信息
  extractUserFromAccount(accountData: any): User {
    return {
      id: accountData.account_id,
      email: accountData.email,
      username: accountData.display_name || undefined,
      verified: accountData.email_verified,
      created_at: accountData.created_at,
    };
  },
};

// 房间相关 API
export const roomApi = {
  async createRoom(name: string, description?: string): Promise<ApiResponse<Room>> {
    const response: AxiosResponse<ApiResponse<Room>> = await api.post('/rooms', {
      name,
      description,
    });
    return response.data;
  },

  async getRooms(): Promise<ApiResponse<Room[]>> {
    const response: AxiosResponse<ApiResponse<Room[]>> = await api.get('/rooms');
    return response.data;
  },

  async getUserRooms(): Promise<ApiResponse<Room[]>> {
    const response: AxiosResponse<ApiResponse<Room[]>> = await api.get('/user/rooms');
    return response.data;
  },

  async joinRoom(roomId: string): Promise<ApiResponse<{ message: string }>> {
    const response: AxiosResponse<ApiResponse<{ message: string }>> = await api.post(`/rooms/${roomId}/join`);
    return response.data;
  },

  async leaveRoom(roomId: string): Promise<ApiResponse<{ message: string }>> {
    const response: AxiosResponse<ApiResponse<{ message: string }>> = await api.post(`/rooms/${roomId}/leave`);
    return response.data;
  },

  async getRoomMessages(roomId: string, limit = 50, offset = 0): Promise<ApiResponse<Message[]>> {
    const response: AxiosResponse<ApiResponse<Message[]>> = await api.get(`/rooms/${roomId}/messages`, {
      params: { limit, offset },
    });
    return response.data;
  },

  async sendMessage(roomId: string, content: string): Promise<ApiResponse<Message>> {
    const response: AxiosResponse<ApiResponse<Message>> = await api.post(`/rooms/${roomId}/messages`, {
      content,
    });
    return response.data;
  },
};

// 好友相关 API
export const friendApi = {
  async sendFriendRequest(userId: string, toUserId: string, message?: string): Promise<ApiResponse<FriendRequest>> {
    const response: AxiosResponse<FriendRequest> = await api.post('/friends/request', {
      to_user_id: toUserId,
      message,
    }, {
      params: { user_id: userId }
    });
    return { data: response.data };
  },

  async respondToFriendRequest(userId: string, requestId: string, accept: boolean): Promise<ApiResponse<FriendRequest>> {
    const response: AxiosResponse<FriendRequest> = await api.post('/friends/request/respond', {
      request_id: requestId,
      accept,
    }, {
      params: { user_id: userId }
    });
    return { data: response.data };
  },

  async getFriendRequests(userId: string): Promise<ApiResponse<FriendRequest[]>> {
    const response: AxiosResponse<FriendRequest[]> = await api.get('/friends/requests', {
      params: { user_id: userId }
    });
    return { data: response.data };
  },

  async getFriends(userId: string): Promise<ApiResponse<string[]>> {
    const response: AxiosResponse<string[]> = await api.get('/friends/list', {
      params: { user_id: userId }
    });
    return { data: response.data };
  },

  async removeFriend(userId: string, friendUserId: string): Promise<ApiResponse<{ message: string }>> {
    await api.delete('/friends/remove', {
      params: { user_id: userId, friend_user_id: friendUserId }
    });
    return { data: { message: 'Friend removed successfully' } };
  },
};

// 工具函数
export async function refreshAccessToken(refreshToken: string): Promise<AuthTokens> {
  return await authApi.refreshToken(refreshToken);
}

export function setTokens(tokens: AuthTokens): void {
  localStorage.setItem('access_token', tokens.access_token);
  localStorage.setItem('refresh_token', tokens.refresh_token);
}

export function clearTokens(): void {
  localStorage.removeItem('access_token');
  localStorage.removeItem('refresh_token');
}

export function isAuthenticated(): boolean {
  return !!localStorage.getItem('access_token');
}

// 登出函数
export function logout() {
  clearTokens();
  localStorage.removeItem('user_info');
  // 可以添加调用后端logout API的逻辑
}

// 清除用户会话
export function clearUserSession() {
  logout();
  // 如果需要，可以在这里添加其他清理逻辑
}
