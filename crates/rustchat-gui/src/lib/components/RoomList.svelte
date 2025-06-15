<script lang="ts">  import { rooms, currentRoom, actions, user } from '../store';
  import { roomApi } from '../api';
  import { 
    joinRoom as joinWsRoom, 
    leaveRoom as leaveWsRoom, 
    connectionState, 
    ConnectionState 
  } from '../websocket';

  interface Props {
    onRefresh: () => Promise<void>;
    loading: boolean;
  }

  let { onRefresh, loading }: Props = $props();
  let showCreateForm = $state(false);
  let newRoomName = $state('');
  let newRoomDescription = $state('');
  let creating = $state(false);
  let error = $state('');  async function handleCreateRoom() {
    if (!newRoomName.trim()) {
      error = 'Room name is required';
      return;
    }

    creating = true;
    error = '';

    try {
      const response = await roomApi.createRoom(newRoomName.trim(), newRoomDescription.trim());
      
      if (response.data) {
        actions.addRoom(response.data);
        newRoomName = '';
        newRoomDescription = '';
        showCreateForm = false;
      } else if (response.error) {
        error = response.error;
      }
    } catch (err: any) {
      error = err.response?.data?.error || 'Failed to create room';
    } finally {
      creating = false;
    }
  }  async function handleJoinRoom(room: any) {
    try {
      console.log('Joining room:', room);
      
      // 如果有当前房间，先通过WebSocket离开
      if ($currentRoom && $connectionState === ConnectionState.Connected) {
        console.log('Leaving current room via WebSocket:', $currentRoom.id);
        leaveWsRoom($currentRoom.id);
      }
      
      // 通过HTTP API加入房间（确保认证）
      await roomApi.joinRoom(room.id);
      console.log('Successfully joined room via HTTP API, setting current room:', room);
      
      // 通过WebSocket加入房间（用于实时消息）
      if ($connectionState === ConnectionState.Connected) {
        console.log('Joining room via WebSocket:', room.id);
        joinWsRoom(room.id);
      }
      
      actions.setCurrentRoom(room);
      console.log('Current room set in store');
    } catch (err: any) {
      console.error('Failed to join room:', err);
      
      // 如果是409冲突（用户已经在房间中），仍然设置为当前房间
      if (err.response?.status === 409) {
        console.log('User already in room, setting as current room anyway:', room);
        
        // 通过WebSocket加入房间（用于实时消息）
        if ($connectionState === ConnectionState.Connected) {
          console.log('Joining room via WebSocket:', room.id);
          joinWsRoom(room.id);
        }
        
        actions.setCurrentRoom(room);
      }
    }
  }

  function cancelCreate() {
    showCreateForm = false;
    newRoomName = '';
    newRoomDescription = '';
    error = '';
  }
</script>

<div class="room-list">
  <div class="room-list-header">
    <h3>Rooms</h3>
    <div class="header-actions">
      <button 
        class="refresh-btn" 
        onclick={onRefresh} 
        disabled={loading}
        title="Refresh rooms"
      >
        🔄
      </button>
      <button 
        class="create-btn" 
        onclick={() => showCreateForm = true}
        title="Create new room"
      >
        ➕
      </button>
    </div>
  </div>

  {#if showCreateForm}
    <div class="create-form">
      <h4>Create New Room</h4>
      
      <div class="form-group">
        <input
          type="text"
          bind:value={newRoomName}
          placeholder="Room name"
          disabled={creating}
          maxlength="50"
        />
      </div>

      <div class="form-group">
        <textarea
          bind:value={newRoomDescription}
          placeholder="Room description (optional)"
          disabled={creating}
          maxlength="200"
          rows="3"
        ></textarea>
      </div>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      <div class="form-actions">
        <button onclick={handleCreateRoom} disabled={creating || !newRoomName.trim()}>
          {creating ? 'Creating...' : 'Create'}
        </button>
        <button type="button" onclick={cancelCreate} disabled={creating} class="secondary">
          Cancel
        </button>
      </div>
    </div>
  {/if}

  <div class="room-list-content">
    {#if loading}
      <div class="loading">Loading rooms...</div>
    {:else if $rooms.length === 0}
      <div class="empty-state">
        <p>No rooms available</p>
        <p>Create a new room to get started!</p>
      </div>
    {:else}
      {#each $rooms as room (room.id)}
        <div 
          class="room-item {$currentRoom?.id === room.id ? 'active' : ''}"
          onclick={() => handleJoinRoom(room)}
        >
          <div class="room-info">
            <div class="room-name">{room.name}</div>
            {#if room.description}
              <div class="room-description">{room.description}</div>
            {/if}
            <div class="room-meta">
              <span class="member-count">👥 {room.member_count || 0}</span>
              <span class="created-date">
                {new Date(room.created_at).toLocaleDateString()}
              </span>
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .room-list {
    width: 280px;
    background-color: white;
    border-left: 1px solid #e1e8ed;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .room-list-header {
    padding: 16px 20px;
    border-bottom: 1px solid #e1e8ed;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .room-list-header h3 {
    margin: 0;
    color: #333;
    font-size: 18px;
    font-weight: 600;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .refresh-btn,
  .create-btn {
    background: none;
    border: 1px solid #ddd;
    border-radius: 6px;
    padding: 6px 8px;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.2s ease;
  }

  .refresh-btn:hover,
  .create-btn:hover {
    background-color: #f5f5f5;
    border-color: #ccc;
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .create-form {
    padding: 20px;
    border-bottom: 1px solid #e1e8ed;
    background-color: #f9f9f9;
  }

  .create-form h4 {
    margin: 0 0 16px;
    color: #333;
    font-size: 16px;
    font-weight: 600;
  }

  .form-group {
    margin-bottom: 12px;
  }

  .form-group input,
  .form-group textarea {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 14px;
    resize: vertical;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #3498db;
  }

  .form-actions {
    display: flex;
    gap: 8px;
  }

  .form-actions button {
    flex: 1;
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .form-actions button:not(.secondary) {
    background-color: #3498db;
    color: white;
  }

  .form-actions button:not(.secondary):hover:not(:disabled) {
    background-color: #2980b9;
  }

  .form-actions button.secondary {
    background-color: #f5f5f5;
    color: #666;
    border: 1px solid #ddd;
  }

  .form-actions button.secondary:hover:not(:disabled) {
    background-color: #e5e5e5;
  }

  .form-actions button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error {
    background: #fee;
    color: #c33;
    padding: 8px 12px;
    border-radius: 4px;
    margin-bottom: 12px;
    font-size: 12px;
    border-left: 3px solid #c33;
  }

  .room-list-content {
    flex: 1;
    overflow-y: auto;
  }

  .loading {
    padding: 40px 20px;
    text-align: center;
    color: #666;
  }

  .empty-state {
    padding: 40px 20px;
    text-align: center;
    color: #666;
  }

  .empty-state p {
    margin: 8px 0;
    font-size: 14px;
    line-height: 1.4;
  }

  .room-item {
    padding: 16px 20px;
    border-bottom: 1px solid #f0f0f0;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .room-item:hover {
    background-color: #f8f9fa;
  }

  .room-item.active {
    background-color: #e3f2fd;
    border-left: 3px solid #3498db;
  }

  .room-info {
    width: 100%;
  }

  .room-name {
    font-weight: 600;
    color: #333;
    margin-bottom: 4px;
    font-size: 15px;
  }

  .room-description {
    color: #666;
    font-size: 13px;
    margin-bottom: 8px;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .room-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: #888;
  }

  .member-count {
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
