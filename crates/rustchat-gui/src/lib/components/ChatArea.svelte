<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { currentRoom, messages, user, actions } from '../store';
  import { roomApi } from '../api';
  let messageInput = $state('');
  let sending = $state(false);
  let loading = $state(false);
  let messagesContainer = $state<HTMLElement>();

  onMount(async () => {
    if ($currentRoom) {
      await loadMessages();
    }
  });

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
  });

  async function loadMessages() {
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
  }

  async function sendMessage() {
    if (!messageInput.trim() || !$currentRoom || sending) return;

    const content = messageInput.trim();
    messageInput = '';
    sending = true;

    try {
      const response = await roomApi.sendMessage($currentRoom.id, content);
      if (response.data) {
        actions.addMessage(response.data);
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
    <p>Select a room to start chatting</p>
  </div>
{:else}
  <div class="chat-area">
    <!-- 聊天头部 -->
    <div class="chat-header">
      <div class="room-info">
        <h3>{$currentRoom.name}</h3>
        {#if $currentRoom.description}
          <p>{$currentRoom.description}</p>
        {/if}
      </div>
      <div class="room-actions">
        <span class="member-count">👥 {$currentRoom.member_count || 0}</span>
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
    </div>

    <!-- 输入区域 -->
    <div class="input-area">
      <div class="input-container">
        <textarea
          bind:value={messageInput}
          onkeydown={handleKeydown}
          placeholder="Type a message..."
          disabled={sending}
          rows="1"
        ></textarea>
        <button 
          onclick={sendMessage} 
          disabled={!messageInput.trim() || sending}
          class="send-button"
        >
          {sending ? '⏳' : '📤'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .no-room {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
    font-size: 16px;
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
</style>
