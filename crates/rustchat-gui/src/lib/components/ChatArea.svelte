<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { currentRoom, messages, user, actions } from '../store';
  import { roomApi } from '../api';  import { 
    webSocketClient, 
    connectWebSocket, 
    sendMessage as sendWsMessage,
    sendRoomMessage as sendWsRoomMessage,
    joinRoom as joinWsRoom,
    leaveRoom as leaveWsRoom,
    connectionState,
    lastWebSocketError,
    ConnectionState
  } from '../websocket';

  let messageInput = $state('');
  let sending = $state(false);
  let loading = $state(false);
  let messagesContainer = $state<HTMLElement>();
  // WebSocket 事件处理器
  let unsubscribeMessage: (() => void) | null = null;
  let unsubscribeRoomMessage: (() => void) | null = null;
  let unsubscribeUserJoinedRoom: (() => void) | null = null;
  let unsubscribeUserLeftRoom: (() => void) | null = null;
  let unsubscribeConnection: (() => void) | null = null;
  let unsubscribeError: (() => void) | null = null;
  onMount(async () => {
    // 初始化WebSocket连接
    await initializeWebSocket();
    
    if ($currentRoom) {
      await loadMessages();
    }
  });
  onDestroy(() => {
    // 清理WebSocket事件监听器
    if (unsubscribeMessage) unsubscribeMessage();
    if (unsubscribeRoomMessage) unsubscribeRoomMessage();
    if (unsubscribeUserJoinedRoom) unsubscribeUserJoinedRoom();
    if (unsubscribeUserLeftRoom) unsubscribeUserLeftRoom();
    if (unsubscribeConnection) unsubscribeConnection();
    if (unsubscribeError) unsubscribeError();
  });
  async function initializeWebSocket() {
    try {
      // 连接WebSocket
      await connectWebSocket();
      
      // 注册全局消息处理器
      unsubscribeMessage = webSocketClient.onMessage((messageEvent) => {
        console.log('Received global WebSocket message:', messageEvent);
        // WebSocket消息会自动通过store添加到消息列表
      });

      // 注册房间消息处理器
      unsubscribeRoomMessage = webSocketClient.onRoomMessage((roomMessageEvent) => {
        console.log('Received room WebSocket message:', roomMessageEvent);
        // 房间消息会自动通过store添加到消息列表
      });

      // 注册用户加入房间处理器
      unsubscribeUserJoinedRoom = webSocketClient.onUserJoinedRoom((event) => {
        console.log('User joined room:', event);
        // 可以在这里显示用户加入通知
      });

      // 注册用户离开房间处理器
      unsubscribeUserLeftRoom = webSocketClient.onUserLeftRoom((event) => {
        console.log('User left room:', event);
        // 可以在这里显示用户离开通知
      });

      // 注册连接状态处理器
      unsubscribeConnection = webSocketClient.onConnection((connected) => {
        console.log('WebSocket connection state:', connected);
      });

      // 注册错误处理器
      unsubscribeError = webSocketClient.onError((error) => {
        console.error('WebSocket error:', error);
      });

    } catch (error) {
      console.error('Failed to initialize WebSocket:', error);
    }
  }

  $effect(() => {
    // 当切换房间时重新加载消息
    if ($currentRoom) {
      loadMessages();
    }
  });

  $effect(() => {
    // 当有新消息时自动滚动到底部
    if (messagesContainer && $messages.length > 0) {
      scrollToBottom();
    }
  });  async function loadMessages() {
    if (!$currentRoom) return;

    loading = true;
    try {
      const response = await roomApi.getRoomMessages($currentRoom.id);
      if (response.data) {
        actions.setMessages(response.data);
      }
    } catch (error) {
      console.error('Failed to load messages:', error);
    } finally {
      loading = false;
    }
  }  async function sendMessage() {
    if (!messageInput.trim() || sending) return;

    const content = messageInput.trim();
    messageInput = '';
    sending = true;

    try {
      if ($currentRoom) {
        // 发送房间消息 (优先使用 WebSocket)
        if ($connectionState === ConnectionState.Connected) {
          sendWsRoomMessage($currentRoom.id, content);
        } else {
          // WebSocket 未连接时回退到 HTTP API
          const response = await roomApi.sendMessage($currentRoom.id, content);
          if (response.data) {
            actions.addMessage(response.data);
          }
        }
      } else {
        // 发送全局消息 (通过 WebSocket)
        if ($connectionState === ConnectionState.Connected) {
          const nickname = $user?.username || $user?.email || 'Anonymous';
          sendWsMessage(content, nickname);
        } else {
          throw new Error('WebSocket not connected');
        }
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      // 恢复输入内容
      messageInput = content;
    } finally {
      sending = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      sendMessage();
    }
  }

  function scrollToBottom() {
    if (messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  }

  function formatTime(timestamp: string) {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function formatDate(timestamp: string) {
    const date = new Date(timestamp);
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(today.getDate() - 1);

    if (date.toDateString() === today.toDateString()) {
      return 'Today';
    } else if (date.toDateString() === yesterday.toDateString()) {
      return 'Yesterday';
    } else {
      return date.toLocaleDateString();
    }
  }
  // 按日期分组消息
  const currentMessages = $derived(() => {
    return $messages
      .filter(message => message.room_id === $currentRoom?.id)
      .reduce((groups: any[], message) => {
        const date = formatDate(message.created_at);
        let group = groups.find(g => g.date === date);
        
        if (!group) {
          group = { date, messages: [] };
          groups.push(group);
        }
        
        group.messages.push(message);
        return groups;
      }, []);
  });
</script>

{#if !$currentRoom}
  <div class="no-room">
    <div class="no-room-content">
      <p>Select a room to start chatting</p>
      <div class="websocket-status">
        <div class="status-indicator" class:connected={$connectionState === ConnectionState.Connected} class:connecting={$connectionState === ConnectionState.Connecting} class:disconnected={$connectionState === ConnectionState.Disconnected || $connectionState === ConnectionState.Failed}></div>
        <span class="status-text">
          {#if $connectionState === ConnectionState.Connected}
            Global Chat Connected
          {:else if $connectionState === ConnectionState.Connecting}
            Connecting to Global Chat...
          {:else}
            Global Chat Disconnected
          {/if}
        </span>
      </div>
      {#if $connectionState === ConnectionState.Connected}
        <div class="global-chat-info">
          <p>You can send messages to the global chat below</p>
        </div>
      {/if}
    </div>
  </div>
{:else}
  <div class="chat-area">    <!-- 聊天头部 -->
    <div class="chat-header">
      <div class="room-info">
        <h3>{$currentRoom.name}</h3>
        {#if $currentRoom.description}
          <p>{$currentRoom.description}</p>
        {/if}
      </div>
      <div class="room-actions">
        <span class="member-count">👥 {$currentRoom.member_count || 0}</span>
        <div class="websocket-status">
          <div class="status-indicator" class:connected={$connectionState === ConnectionState.Connected} class:connecting={$connectionState === ConnectionState.Connecting} class:disconnected={$connectionState === ConnectionState.Disconnected || $connectionState === ConnectionState.Failed}></div>
          <span class="status-text">
            {#if $connectionState === ConnectionState.Connected}
              Live
            {:else if $connectionState === ConnectionState.Connecting}
              Connecting...
            {:else}
              Offline
            {/if}
          </span>
        </div>
      </div>
    </div>

    <!-- 消息区域 -->
    <div class="messages-container" bind:this={messagesContainer}>
      {#if loading}
        <div class="loading">
          <div class="spinner"></div>
          <p>Loading messages...</p>
        </div>      {:else if currentMessages().length === 0}
        <div class="empty-messages">
          <p>No messages yet</p>
          <p>Be the first to start the conversation! 💬</p>
        </div>
      {:else}
        {#each currentMessages() as group (group.date)}
          <div class="date-group">
            <div class="date-divider">
              <span>{group.date}</span>
            </div>
            
            {#each group.messages as message (message.id)}
              <div class="message {message.user_id === $user?.id ? 'own' : 'other'}">
                <div class="message-content">
                  {#if message.user_id !== $user?.id}
                    <div class="message-author">
                      {message.user?.email || 'Unknown User'}
                    </div>
                  {/if}
                  <div class="message-text">
                    {message.content}
                  </div>
                  <div class="message-time">
                    {formatTime(message.created_at)}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    </div>    <!-- 输入区域 -->
    <div class="input-area">
      <div class="input-container">
        <textarea
          bind:value={messageInput}
          onkeydown={handleKeydown}
          placeholder={$currentRoom ? "Type a message..." : "Type a message to global chat..."}
          disabled={sending || (!$currentRoom && $connectionState !== ConnectionState.Connected)}
          rows="1"
        ></textarea>
        <button 
          onclick={sendMessage} 
          disabled={!messageInput.trim() || sending || (!$currentRoom && $connectionState !== ConnectionState.Connected)}
          class="send-button"
        >
          {sending ? '⏳' : '📤'}
        </button>
      </div>
      {#if !$currentRoom && $connectionState !== ConnectionState.Connected}
        <div class="connection-warning">
          <span>⚠️ Connect to global chat to send messages</span>
        </div>
      {/if}
      {#if $lastWebSocketError}
        <div class="websocket-error">
          <span>❌ {$lastWebSocketError}</span>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>  .no-room {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
    font-size: 16px;
  }

  .no-room-content {
    text-align: center;
    max-width: 400px;
  }

  .no-room-content p {
    margin-bottom: 20px;
  }

  .websocket-status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    background: #f8f9fa;
    border: 1px solid #e9ecef;
    margin-bottom: 16px;
  }

  .status-indicator {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    transition: background-color 0.3s;
  }

  .status-indicator.connected {
    background-color: #28a745;
  }

  .status-indicator.connecting {
    background-color: #ffc107;
    animation: pulse 1.5s infinite;
  }

  .status-indicator.disconnected {
    background-color: #dc3545;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .status-text {
    font-size: 12px;
    font-weight: 500;
    color: #495057;
  }

  .global-chat-info {
    color: #666;
    font-size: 14px;
  }

  .global-chat-info p {
    margin: 0;
  }

  .chat-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    background-color: white;
    margin: 20px 20px 20px 0;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .chat-header {
    padding: 16px 20px;
    border-bottom: 1px solid #e1e8ed;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: #f8f9fa;
  }

  .room-info h3 {
    margin: 0;
    color: #333;
    font-size: 18px;
    font-weight: 600;
  }

  .room-info p {
    margin: 4px 0 0;
    color: #666;
    font-size: 14px;
  }
  .room-actions {
    display: flex;
    align-items: center;
    color: #666;
    font-size: 14px;
    gap: 16px;
  }

  .room-actions .websocket-status {
    margin: 0;
    padding: 4px 8px;
    font-size: 11px;
  }

  .room-actions .status-indicator {
    width: 8px;
    height: 8px;
  }

  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    scroll-behavior: smooth;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: #666;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #f3f3f3;
    border-top: 3px solid #3498db;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 12px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .empty-messages {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: #666;
    text-align: center;
  }

  .empty-messages p {
    margin: 4px 0;
    font-size: 16px;
  }

  .date-group {
    margin-bottom: 24px;
  }

  .date-divider {
    text-align: center;
    margin: 16px 0;
  }

  .date-divider span {
    background-color: #f5f5f5;
    color: #666;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
  }

  .message {
    margin-bottom: 12px;
    display: flex;
  }

  .message.own {
    justify-content: flex-end;
  }

  .message.other {
    justify-content: flex-start;
  }

  .message-content {
    max-width: 70%;
    background-color: #f1f3f4;
    border-radius: 18px;
    padding: 10px 16px;
    position: relative;
  }

  .message.own .message-content {
    background-color: #3498db;
    color: white;
  }

  .message-author {
    font-size: 12px;
    font-weight: 600;
    color: #666;
    margin-bottom: 4px;
  }

  .message.own .message-author {
    color: rgba(255, 255, 255, 0.8);
  }

  .message-text {
    font-size: 14px;
    line-height: 1.4;
    word-wrap: break-word;
    white-space: pre-wrap;
  }

  .message-time {
    font-size: 11px;
    color: #888;
    margin-top: 4px;
    text-align: right;
  }

  .message.own .message-time {
    color: rgba(255, 255, 255, 0.7);
  }

  .input-area {
    padding: 16px 20px;
    border-top: 1px solid #e1e8ed;
    background-color: #f8f9fa;
  }

  .input-container {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    max-height: 120px;
  }

  .input-container textarea {
    flex: 1;
    min-height: 40px;
    max-height: 120px;
    padding: 12px 16px;
    border: 1px solid #ddd;
    border-radius: 20px;
    font-size: 14px;
    font-family: inherit;
    resize: none;
    outline: none;
    background-color: white;
  }

  .input-container textarea:focus {
    border-color: #3498db;
  }

  .input-container textarea:disabled {
    background-color: #f5f5f5;
    cursor: not-allowed;
  }

  .send-button {
    width: 40px;
    height: 40px;
    border: none;
    border-radius: 50%;
    background-color: #3498db;
    color: white;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    transition: all 0.2s ease;
    flex-shrink: 0;
  }

  .send-button:hover:not(:disabled) {
    background-color: #2980b9;
    transform: translateY(-2px);
  }
  .send-button:disabled {
    background-color: #bdc3c7;
    cursor: not-allowed;
    transform: none;
  }

  .connection-warning {
    text-align: center;
    padding: 8px;
    background-color: #fff3cd;
    color: #856404;
    border-radius: 4px;
    font-size: 12px;
    margin-top: 8px;
  }

  .websocket-error {
    text-align: center;
    padding: 8px;
    background-color: #f8d7da;
    color: #721c24;
    border-radius: 4px;
    font-size: 12px;
    margin-top: 8px;
  }
</style>
