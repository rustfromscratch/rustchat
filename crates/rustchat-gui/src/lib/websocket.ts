// WebSocket 客户端实现
import { writable, type Writable } from 'svelte/store';
import type { 
  WsEvent, 
  ClientMessage, 
  WebSocketOptions,
  ConnectedEvent,
  WsMessageEvent,
  UserJoinedEvent,
  UserLeftEvent,
  RoomMessageEvent,
  UserJoinedRoomEvent,
  UserLeftRoomEvent,
  ErrorEvent,
  SendMessageData,
  SendRoomMessageData,
  JoinRoomData,
  LeaveRoomData,
  SetNicknameData
} from './websocket-types';
import { ConnectionState } from './websocket-types';
import type { Message } from './types';
import { actions } from './store';

class WebSocketClient {
  private ws: WebSocket | null = null;
  private options: WebSocketOptions;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private heartbeatTimer: NodeJS.Timeout | null = null;  private reconnectAttempts = 0;
  private userId: string | null = null;
  private isManualClose = false;
  private authToken: string | null = null;

  // 可观察状态
  public connectionState: Writable<ConnectionState>;
  public lastError: Writable<string | null>;
  public connectedUsers: Writable<Set<string>>;
  public isConnecting: Writable<boolean>;
  // 事件回调
  private messageHandlers = new Set<(message: WsMessageEvent) => void>();
  private roomMessageHandlers = new Set<(event: RoomMessageEvent) => void>();
  private userJoinedHandlers = new Set<(event: UserJoinedEvent) => void>();
  private userLeftHandlers = new Set<(event: UserLeftEvent) => void>();
  private userJoinedRoomHandlers = new Set<(event: UserJoinedRoomEvent) => void>();
  private userLeftRoomHandlers = new Set<(event: UserLeftRoomEvent) => void>();
  private connectionHandlers = new Set<(connected: boolean) => void>();
  private errorHandlers = new Set<(error: string) => void>();

  constructor(options: Partial<WebSocketOptions> = {}) {
    this.options = {
      url: 'ws://127.0.0.1:8080/ws',
      reconnectInterval: 3000,
      maxReconnectAttempts: 10,
      heartbeatInterval: 30000,
      debug: false,
      ...options
    };

    // 初始化状态
    this.connectionState = writable(ConnectionState.Disconnected);
    this.lastError = writable(null);
    this.connectedUsers = writable(new Set());
    this.isConnecting = writable(false);
  }
  /**
   * 设置认证令牌
   */
  public setAuthToken(token: string | null): void {
    this.authToken = token;
    this.log('Auth token set:', token ? 'present' : 'null');
  }
  /**
   * 连接到WebSocket服务器
   */
  public async connect(): Promise<void> {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.log('Already connected');
      return;
    }

    this.isManualClose = false;
    this.isConnecting.set(true);
    this.connectionState.set(ConnectionState.Connecting);
    this.lastError.set(null);

    try {
      // 构建WebSocket URL，如果有认证令牌则添加到query参数
      let wsUrl = this.options.url;
      if (this.authToken) {
        const separator = this.options.url.includes('?') ? '&' : '?';
        wsUrl = `${this.options.url}${separator}token=${encodeURIComponent(this.authToken)}`;
      }
      
      this.log(`Connecting to ${wsUrl}`);
      this.ws = new WebSocket(wsUrl);
      
      this.ws.onopen = this.handleOpen.bind(this);
      this.ws.onmessage = this.handleMessage.bind(this);
      this.ws.onclose = this.handleClose.bind(this);
      this.ws.onerror = this.handleError.bind(this);

    } catch (error) {
      this.log('Connection failed:', error);
      this.connectionState.set(ConnectionState.Failed);
      this.isConnecting.set(false);
      this.notifyError(`Connection failed: ${error}`);
      throw error;
    }
  }

  /**
   * 断开连接
   */
  public disconnect(): void {
    this.log('Manually disconnecting');
    this.isManualClose = true;
    this.cleanup();
    this.connectionState.set(ConnectionState.Disconnected);
  }

  /**
   * 发送文本消息
   */
  public sendMessage(content: string, nickname?: string): void {
    const message: ClientMessage = {
      type: 'SendMessage',
      data: { content, nickname } as SendMessageData
    };
    this.send(message);
  }
  /**
   * 发送房间消息
   */
  public sendRoomMessage(roomId: string, content: string): void {
    const message: ClientMessage = {
      type: 'SendRoomMessage',
      data: { room_id: roomId, content } as SendRoomMessageData
    };
    this.send(message);
  }

  /**
   * 加入房间
   */
  public joinRoom(roomId: string): void {
    const message: ClientMessage = {
      type: 'JoinRoom',
      data: { room_id: roomId } as JoinRoomData
    };
    this.send(message);
  }

  /**
   * 离开房间
   */
  public leaveRoom(roomId: string): void {
    const message: ClientMessage = {
      type: 'LeaveRoom',
      data: { room_id: roomId } as LeaveRoomData
    };
    this.send(message);
  }
  /**
   * 设置昵称
   */
  public setNickname(nickname: string): void {
    const message: ClientMessage = {
      type: 'SetNickname',
      data: { nickname } as SetNicknameData
    };
    this.send(message);
  }
  /**
   * 发送心跳响应
   */
  public sendPong(): void {
    const message: ClientMessage = {
      type: 'Pong'
    };
    this.send(message);
  }
  private send(message: ClientMessage): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.log('Cannot send message: WebSocket is not open');
      this.notifyError('Cannot send message: Not connected');
      return;
    }

    try {
      const jsonMessage = JSON.stringify(message);
      this.ws.send(jsonMessage);
      this.log('Sent message:', message);
    } catch (error) {
      this.log('Failed to send message:', error);
      this.notifyError(`Failed to send message: ${error}`);
    }
  }
  /**
   * 通用发送方法
   */  /**
   * 事件处理器注册
   */
  public onMessage(handler: (message: WsMessageEvent) => void): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  public onRoomMessage(handler: (event: RoomMessageEvent) => void): () => void {
    this.roomMessageHandlers.add(handler);
    return () => this.roomMessageHandlers.delete(handler);
  }

  public onUserJoined(handler: (event: UserJoinedEvent) => void): () => void {
    this.userJoinedHandlers.add(handler);
    return () => this.userJoinedHandlers.delete(handler);
  }

  public onUserLeft(handler: (event: UserLeftEvent) => void): () => void {
    this.userLeftHandlers.add(handler);
    return () => this.userLeftHandlers.delete(handler);
  }

  public onUserJoinedRoom(handler: (event: UserJoinedRoomEvent) => void): () => void {
    this.userJoinedRoomHandlers.add(handler);
    return () => this.userJoinedRoomHandlers.delete(handler);
  }

  public onUserLeftRoom(handler: (event: UserLeftRoomEvent) => void): () => void {
    this.userLeftRoomHandlers.add(handler);
    return () => this.userLeftRoomHandlers.delete(handler);
  }

  public onConnection(handler: (connected: boolean) => void): () => void {
    this.connectionHandlers.add(handler);
    return () => this.connectionHandlers.delete(handler);
  }

  public onError(handler: (error: string) => void): () => void {
    this.errorHandlers.add(handler);
    return () => this.errorHandlers.delete(handler);
  }

  /**
   * WebSocket 事件处理
   */
  private handleOpen(): void {
    this.log('WebSocket connection opened');
    this.connectionState.set(ConnectionState.Connected);
    this.isConnecting.set(false);
    this.reconnectAttempts = 0;
    this.lastError.set(null);

    // 启动心跳
    this.startHeartbeat();

    // 通知连接状态
    this.connectionHandlers.forEach(handler => handler(true));
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const wsEvent: WsEvent = JSON.parse(event.data);
      this.log('Received event:', wsEvent);

      switch (wsEvent.event) {
        case 'Connected':
          this.handleConnectedEvent(wsEvent.data as ConnectedEvent);
          break;        case 'Message':
          this.handleMessageEvent(wsEvent.data as WsMessageEvent);
          break;
        case 'RoomMessage':
          this.handleRoomMessageEvent(wsEvent.data as RoomMessageEvent);
          break;
        case 'UserJoined':
          this.handleUserJoinedEvent(wsEvent.data as UserJoinedEvent);
          break;
        case 'UserLeft':
          this.handleUserLeftEvent(wsEvent.data as UserLeftEvent);
          break;
        case 'UserJoinedRoom':
          this.handleUserJoinedRoomEvent(wsEvent.data as UserJoinedRoomEvent);
          break;
        case 'UserLeftRoom':
          this.handleUserLeftRoomEvent(wsEvent.data as UserLeftRoomEvent);
          break;
        case 'Ping':
          this.sendPong();
          break;
        case 'Pong':
          this.log('Received pong');
          break;
        case 'Error':
          this.handleErrorEvent(wsEvent.data as ErrorEvent);
          break;
        default:
          this.log('Unknown event type:', wsEvent.event);
      }
    } catch (error) {
      this.log('Failed to parse message:', error);
      this.notifyError(`Failed to parse message: ${error}`);
    }
  }

  private handleClose(event: CloseEvent): void {
    this.log(`WebSocket connection closed: ${event.code} - ${event.reason}`);
    this.connectionState.set(ConnectionState.Disconnected);
    this.isConnecting.set(false);
    this.stopHeartbeat();

    // 通知连接状态
    this.connectionHandlers.forEach(handler => handler(false));

    if (!this.isManualClose) {
      this.scheduleReconnect();
    }
  }

  private handleError(event: Event): void {
    this.log('WebSocket error:', event);
    this.connectionState.set(ConnectionState.Failed);
    this.isConnecting.set(false);
    this.notifyError('WebSocket connection error');
  }

  /**
   * 具体事件处理
   */
  private handleConnectedEvent(data: ConnectedEvent): void {
    this.userId = data.user_id;
    this.log(`Connected with user ID: ${this.userId}`);
  }
  private handleMessageEvent(data: WsMessageEvent): void {
    this.log('Received message:', data);
    
    // 转换为应用消息格式并添加到store
    const message: Message = {
      id: data.id,
      room_id: null, // 全局消息
      user_id: data.user_id,
      content: data.content,
      created_at: data.created_at,
      additional_data: data.additional_data
    };
    
    actions.addMessage(message);
    
    // 通知处理器
    this.messageHandlers.forEach(handler => handler(data));
  }

  private handleRoomMessageEvent(data: RoomMessageEvent): void {
    this.log('Received room message:', data);
    
    // 转换为应用消息格式并添加到store
    const message: Message = {
      id: data.message.id,
      room_id: data.message.room_id,
      user_id: data.message.user_id,
      content: data.message.content,
      created_at: data.message.created_at,
      additional_data: data.message.additional_data
    };
    
    actions.addMessage(message);
    
    // 通知处理器
    this.roomMessageHandlers.forEach(handler => handler(data));
  }

  private handleUserJoinedEvent(data: UserJoinedEvent): void {
    this.log('User joined:', data);
    this.connectedUsers.update(users => {
      const newUsers = new Set(users);
      newUsers.add(data.user_id);
      return newUsers;
    });
    
    this.userJoinedHandlers.forEach(handler => handler(data));
  }
  private handleUserLeftEvent(data: UserLeftEvent): void {
    this.log('User left:', data);
    this.connectedUsers.update(users => {
      const newUsers = new Set(users);
      newUsers.delete(data.user_id);
      return newUsers;
    });
    
    this.userLeftHandlers.forEach(handler => handler(data));
  }

  private handleUserJoinedRoomEvent(data: UserJoinedRoomEvent): void {
    this.log('User joined room:', data);
    this.userJoinedRoomHandlers.forEach(handler => handler(data));
  }

  private handleUserLeftRoomEvent(data: UserLeftRoomEvent): void {
    this.log('User left room:', data);
    this.userLeftRoomHandlers.forEach(handler => handler(data));
  }

  private handleErrorEvent(data: ErrorEvent): void {
    this.log('Server error:', data.message);
    this.notifyError(data.message);
  }

  /**
   * 重连逻辑
   */
  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.log('Max reconnect attempts reached');
      this.connectionState.set(ConnectionState.Failed);
      this.notifyError('Connection failed after maximum retry attempts');
      return;
    }

    this.reconnectAttempts++;
    this.connectionState.set(ConnectionState.Reconnecting);
    
    this.log(`Scheduling reconnect attempt ${this.reconnectAttempts}/${this.options.maxReconnectAttempts} in ${this.options.reconnectInterval}ms`);
    
    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(error => {
        this.log('Reconnect failed:', error);
      });
    }, this.options.reconnectInterval);
  }

  /**
   * 心跳机制
   */
  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        // 服务器会发送Ping，我们只需要响应Pong
        this.log('Heartbeat check');
      }
    }, this.options.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * 清理资源
   */
  private cleanup(): void {
    this.stopHeartbeat();
    
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.onopen = null;
      this.ws.onmessage = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.close();
      this.ws = null;
    }

    this.reconnectAttempts = 0;
  }

  /**
   * 通知错误
   */
  private notifyError(message: string): void {
    this.lastError.set(message);
    this.errorHandlers.forEach(handler => handler(message));
  }

  /**
   * 日志输出
   */
  private log(...args: any[]): void {
    if (this.options.debug) {
      console.log('[WebSocket]', ...args);
    }
  }

  /**
   * 获取连接状态
   */
  public getConnectionState(): ConnectionState {
    if (!this.ws) return ConnectionState.Disconnected;
    
    switch (this.ws.readyState) {
      case WebSocket.CONNECTING:
        return ConnectionState.Connecting;
      case WebSocket.OPEN:
        return ConnectionState.Connected;
      default:
        return ConnectionState.Disconnected;
    }
  }

  /**
   * 获取用户ID
   */
  public getUserId(): string | null {
    return this.userId;
  }
  /**
   * 销毁客户端
   */
  public destroy(): void {
    this.disconnect();
    this.messageHandlers.clear();
    this.roomMessageHandlers.clear();
    this.userJoinedHandlers.clear();
    this.userLeftHandlers.clear();
    this.userJoinedRoomHandlers.clear();
    this.userLeftRoomHandlers.clear();
    this.connectionHandlers.clear();
    this.errorHandlers.clear();
  }
}

// 创建全局WebSocket客户端实例
export const webSocketClient = new WebSocketClient({
  debug: true
});

// 导出便捷方法
export const connectWebSocket = () => webSocketClient.connect();
export const disconnectWebSocket = () => webSocketClient.disconnect();
export const sendMessage = (content: string, nickname?: string) => webSocketClient.sendMessage(content, nickname);
export const sendRoomMessage = (roomId: string, content: string) => webSocketClient.sendRoomMessage(roomId, content);
export const joinRoom = (roomId: string) => webSocketClient.joinRoom(roomId);
export const leaveRoom = (roomId: string) => webSocketClient.leaveRoom(roomId);
export const setNickname = (nickname: string) => webSocketClient.setNickname(nickname);

// 导出状态 stores
export const connectionState = webSocketClient.connectionState;
export const lastWebSocketError = webSocketClient.lastError;
export const connectedUsers = webSocketClient.connectedUsers;
export const isWebSocketConnecting = webSocketClient.isConnecting;

// 导出类型和枚举
export { ConnectionState } from './websocket-types';
