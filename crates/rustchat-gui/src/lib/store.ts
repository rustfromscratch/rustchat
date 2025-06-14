// 应用状态管理
import { writable, derived } from 'svelte/store';
import type { User, Room, Message, AppState } from './types';

// 创建可写存储
export const user = writable<User | null>(null);
export const currentRoom = writable<Room | null>(null);
export const rooms = writable<Room[]>([]);
export const messages = writable<Message[]>([]);
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
  [user, currentRoom, rooms, messages, isLoading, error],
  ([$user, $currentRoom, $rooms, $messages, $isLoading, $error]): AppState => ({
    user: $user,
    currentRoom: $currentRoom,
    rooms: $rooms,
    messages: $messages,
    isLoading: $isLoading,
    error: $error,
  })
);

// 状态管理操作
export const actions = {
  setUser: (userData: User | null) => {
    user.set(userData);
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

  setLoading: (loading: boolean) => {
    isLoading.set(loading);
  },

  setError: (errorMessage: string | null) => {
    error.set(errorMessage);
  },

  clearError: () => {
    error.set(null);
  },

  reset: () => {
    user.set(null);
    currentRoom.set(null);
    rooms.set([]);
    messages.set([]);
    isLoading.set(false);
    error.set(null);
  },
};
