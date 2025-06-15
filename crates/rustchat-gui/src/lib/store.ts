// 应用状态管理
import { writable, derived } from 'svelte/store';
import type { User, Room, Message, AppState, FriendRequest } from './types';
import { webSocketClient } from './websocket';

// 创建可写存储
export const user = writable<User | null>(null);
export const authToken = writable<string | null>(null);
export const currentRoom = writable<Room | null>(null);
export const rooms = writable<Room[]>([]);
export const messages = writable<Message[]>([]);
export const friends = writable<string[]>([]);
export const friendRequests = writable<FriendRequest[]>([]);
export const isLoading = writable<boolean>(false);
export const error = writable<string | null>(null);

// 派生存储
export const isAuthenticated = derived(user, ($user) => $user !== null);

export const currentRoomMessages = derived(
  [messages, currentRoom],
  ([$messages, $currentRoom]) => {
    if (!$currentRoom) return [];
    return $messages.filter(message => message.room_id === $currentRoom.id);
  }
);

// 应用状态
export const appState = derived(
  [user, authToken, currentRoom, rooms, messages, friends, friendRequests, isLoading, error],
  ([$user, $authToken, $currentRoom, $rooms, $messages, $friends, $friendRequests, $isLoading, $error]): AppState => ({
    user: $user,
    currentRoom: $currentRoom,
    rooms: $rooms,
    messages: $messages,
    friends: $friends,
    friendRequests: $friendRequests,
    isLoading: $isLoading,
    error: $error,
  })
);

// 状态管理操作
export const actions = {
  setUser: (userData: User | null) => {
    user.set(userData);
  },

  setAuthToken: (token: string | null) => {
    authToken.set(token);
    // 同步设置WebSocket客户端的认证令牌
    webSocketClient.setAuthToken(token);
    
    // 如果有token，尝试重连WebSocket以使用新的认证
    if (token) {
      webSocketClient.connect().catch(error => {
        console.error('Failed to reconnect WebSocket with auth token:', error);
      });
    }
  },

  setCurrentRoom: (room: Room | null) => {
    currentRoom.set(room);
  },

  setRooms: (roomList: Room[]) => {
    rooms.set(roomList);
  },

  addRoom: (room: Room) => {
    rooms.update(currentRooms => [...currentRooms, room]);
  },

  setMessages: (messageList: Message[]) => {
    messages.set(messageList);
  },
  addMessage: (message: Message) => {
    messages.update(currentMessages => [...currentMessages, message]);
  },

  setFriends: (friendList: string[]) => {
    friends.set(friendList);
  },

  addFriend: (friendUserId: string) => {
    friends.update(currentFriends => [...currentFriends, friendUserId]);
  },

  removeFriend: (friendUserId: string) => {
    friends.update(currentFriends => currentFriends.filter(id => id !== friendUserId));
  },

  setFriendRequests: (requestList: FriendRequest[]) => {
    friendRequests.set(requestList);
  },

  addFriendRequest: (request: FriendRequest) => {
    friendRequests.update(currentRequests => [...currentRequests, request]);
  },

  updateFriendRequest: (requestId: string, updates: Partial<FriendRequest>) => {
    friendRequests.update(currentRequests => 
      currentRequests.map(request => 
        request.id === requestId ? { ...request, ...updates } : request
      )
    );
  },

  removeFriendRequest: (requestId: string) => {
    friendRequests.update(currentRequests => 
      currentRequests.filter(request => request.id !== requestId)
    );
  },

  setLoading: (loading: boolean) => {
    isLoading.set(loading);
  },

  setError: (errorMessage: string | null) => {
    error.set(errorMessage);
  },

  clearError: () => {
    error.set(null);
  },  reset: () => {
    user.set(null);
    authToken.set(null);
    currentRoom.set(null);
    rooms.set([]);
    messages.set([]);
    friends.set([]);
    friendRequests.set([]);
    isLoading.set(false);
    error.set(null);
    
    // 清理WebSocket连接
    webSocketClient.setAuthToken(null);
    webSocketClient.disconnect();
    
    // 清理localStorage
    localStorage.removeItem('user_info');
  },
};
