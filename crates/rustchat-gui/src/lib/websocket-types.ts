// WebSocket 事件类型定义
export interface WsEvent {
  event: 'Connected' | 'Message' | 'UserJoined' | 'UserLeft' | 'RoomMessage' | 'UserJoinedRoom' | 'UserLeftRoom' | 'Ping' | 'Pong' | 'Error';
  data?: any;
}

export interface ConnectedEvent {
  user_id: string;
}

export interface WsMessageEvent {
  id: string;
  user_id: string;
  content: string;
  nickname?: string;
  message_type: 'Text' | 'NickChange';
  created_at: string;
  additional_data?: any;
}

export interface UserJoinedEvent {
  user_id: string;
  nickname?: string;
}

export interface UserLeftEvent {
  user_id: string;
}

export interface RoomMessageEvent {
  room_id: string;
  message: {
    id: string;
    room_id: string | null;
    user_id: string;
    content: string;
    created_at: string;
    additional_data?: any;
  };
}

export interface UserJoinedRoomEvent {
  room_id: string;
  user_id: string;
}

export interface UserLeftRoomEvent {
  room_id: string;
  user_id: string;
}

export interface ErrorEvent {
  message: string;
}

// 客户端消息类型
export interface ClientMessage {
  type: 'SendMessage' | 'SendRoomMessage' | 'JoinRoom' | 'LeaveRoom' | 'SetNickname' | 'Pong';
  data?: any;
}

export interface SendMessageData {
  content: string;
  nickname?: string;
}

export interface SendRoomMessageData {
  room_id: string;
  content: string;
}

export interface JoinRoomData {
  room_id: string;
}

export interface LeaveRoomData {
  room_id: string;
}

export interface SetNicknameData {
  nickname: string;
}

// WebSocket 连接状态
export const enum ConnectionState {
  Disconnected = 'disconnected',
  Connecting = 'connecting',
  Connected = 'connected',
  Reconnecting = 'reconnecting',
  Failed = 'failed'
}

// WebSocket 连接选项
export interface WebSocketOptions {
  url: string;
  reconnectInterval: number;
  maxReconnectAttempts: number;
  heartbeatInterval: number;
  debug: boolean;
}
