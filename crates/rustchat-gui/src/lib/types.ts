// 类型定义文件
export interface User {
  id: string;
  email: string;
  username?: string;
  verified: boolean;
  created_at: string;
}

export interface Room {
  id: string;
  name: string;
  description?: string;
  created_by: string;
  created_at: string;
  member_count: number;
}

export interface Message {
  id: string;
  room_id: string;
  user_id: string;
  content: string;
  created_at: string;
  user?: User;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface ApiResponse<T> {
  data?: T;
  error?: string;
  message?: string;
}

// 应用状态
export interface AppState {
  user: User | null;
  currentRoom: Room | null;
  rooms: Room[];
  messages: Message[];
  isLoading: boolean;
  error: string | null;
}
