<script lang="ts">
  import { onMount } from 'svelte';
  import { friendApi } from '../api';
  import { user, friends, friendRequests, actions } from '../store';
  import type { FriendRequest } from '../types';

  let showFriendRequests = false;
  let newFriendUserId = '';
  let friendRequestMessage = '';
  let isLoading = false;
  let error = '';

  // 获取好友列表
  async function loadFriends() {
    if (!$user) return;
    
    try {
      isLoading = true;
      const response = await friendApi.getFriends($user.id);
      if (response.data) {
        actions.setFriends(response.data);
      } else if (response.error) {
        error = response.error;
      }
    } catch (err) {
      error = 'Failed to load friends';
      console.error('Error loading friends:', err);
    } finally {
      isLoading = false;
    }
  }

  // 获取好友请求
  async function loadFriendRequests() {
    if (!$user) return;
    
    try {
      const response = await friendApi.getFriendRequests($user.id);
      if (response.data) {
        actions.setFriendRequests(response.data);
      } else if (response.error) {
        error = response.error;
      }
    } catch (err) {
      error = 'Failed to load friend requests';
      console.error('Error loading friend requests:', err);
    }
  }

  // 发送好友请求
  async function sendFriendRequest() {
    if (!$user || !newFriendUserId.trim()) return;
    
    try {
      isLoading = true;
      const response = await friendApi.sendFriendRequest(
        $user.id,
        newFriendUserId.trim(),
        friendRequestMessage.trim() || undefined
      );
      
      if (response.data) {
        actions.addFriendRequest(response.data);
        newFriendUserId = '';
        friendRequestMessage = '';
        error = '';
        alert('Friend request sent successfully!');
      } else if (response.error) {
        error = response.error;
      }
    } catch (err) {
      error = 'Failed to send friend request';
      console.error('Error sending friend request:', err);
    } finally {
      isLoading = false;
    }
  }

  // 接受好友请求
  async function acceptFriendRequest(request: FriendRequest) {
    if (!$user) return;
    
    try {
      const response = await friendApi.respondToFriendRequest($user.id, request.id, true);
      if (response.data) {
        actions.removeFriendRequest(request.id);
        actions.addFriend(request.from_user_id);
        error = '';
      } else if (response.error) {
        error = response.error;
      }
    } catch (err) {
      error = 'Failed to accept friend request';
      console.error('Error accepting friend request:', err);
    }
  }

  // 拒绝好友请求
  async function rejectFriendRequest(request: FriendRequest) {
    if (!$user) return;
    
    try {
      const response = await friendApi.respondToFriendRequest($user.id, request.id, false);
      if (response.data) {
        actions.removeFriendRequest(request.id);
        error = '';
      } else if (response.error) {
        error = response.error;
      }
    } catch (err) {
      error = 'Failed to reject friend request';
      console.error('Error rejecting friend request:', err);
    }
  }

  // 删除好友
  async function removeFriend(friendUserId: string) {
    if (!$user) return;
    
    if (!confirm('Are you sure you want to remove this friend?')) return;
    
    try {
      const response = await friendApi.removeFriend($user.id, friendUserId);
      if (response.data) {
        actions.removeFriend(friendUserId);
        error = '';
      } else if (response.error) {
        error = response.error;
      }
    } catch (err) {
      error = 'Failed to remove friend';
      console.error('Error removing friend:', err);
    }
  }

  onMount(() => {
    loadFriends();
    loadFriendRequests();
  });
</script>

<div class="friends-panel">
  <h2>Friends</h2>
  
  {#if error}
    <div class="error">{error}</div>
  {/if}

  <!-- Add Friend Section -->
  <div class="add-friend">
    <h3>Add Friend</h3>
    <div class="form-group">
      <input 
        type="text" 
        bind:value={newFriendUserId} 
        placeholder="Enter user ID"
        disabled={isLoading}
      />
    </div>
    <div class="form-group">
      <textarea 
        bind:value={friendRequestMessage} 
        placeholder="Optional message..."
        disabled={isLoading}
      ></textarea>
    </div>
    <button 
      on:click={sendFriendRequest} 
      disabled={isLoading || !newFriendUserId.trim()}
      class="btn-primary"
    >
      Send Friend Request
    </button>
  </div>

  <!-- Friend Requests Toggle -->
  <div class="section-header">
    <button 
      on:click={() => showFriendRequests = !showFriendRequests}
      class="btn-secondary"
    >
      Friend Requests ({$friendRequests.length})
    </button>
  </div>

  <!-- Friend Requests Section -->
  {#if showFriendRequests}
    <div class="friend-requests">
      {#if $friendRequests.length === 0}
        <p class="no-items">No pending friend requests</p>
      {:else}
        {#each $friendRequests as request}
          <div class="request-item">
            <div class="request-info">
              <strong>From: {request.from_user_id}</strong>
              {#if request.message}
                <p class="request-message">"{request.message}"</p>
              {/if}
              <span class="request-date">
                {new Date(request.created_at * 1000).toLocaleDateString()}
              </span>
            </div>
            <div class="request-actions">
              <button 
                on:click={() => acceptFriendRequest(request)}
                class="btn-success"
              >
                Accept
              </button>
              <button 
                on:click={() => rejectFriendRequest(request)}
                class="btn-danger"
              >
                Reject
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Friends List -->
  <div class="friends-list">
    <h3>Friends ({$friends.length})</h3>
    {#if isLoading}
      <p class="loading">Loading friends...</p>
    {:else if $friends.length === 0}
      <p class="no-items">No friends yet</p>
    {:else}
      {#each $friends as friendId}
        <div class="friend-item">
          <div class="friend-info">
            <strong>{friendId}</strong>
          </div>
          <div class="friend-actions">
            <button 
              on:click={() => removeFriend(friendId)}
              class="btn-danger btn-sm"
            >
              Remove
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .friends-panel {
    padding: 1rem;
    border: 1px solid #ddd;
    border-radius: 8px;
    background: white;
    max-width: 500px;
  }

  .error {
    background: #fee;
    color: #c33;
    padding: 0.5rem;
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .add-friend {
    margin-bottom: 1.5rem;
    padding: 1rem;
    border: 1px solid #eee;
    border-radius: 6px;
    background: #f9f9f9;
  }

  .form-group {
    margin-bottom: 0.5rem;
  }

  .form-group input,
  .form-group textarea {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 14px;
  }

  .form-group textarea {
    height: 60px;
    resize: vertical;
  }

  .section-header {
    margin: 1rem 0;
  }

  .friend-requests {
    margin-bottom: 1.5rem;
  }

  .request-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border: 1px solid #ddd;
    border-radius: 6px;
    margin-bottom: 0.5rem;
    background: #f9f9f9;
  }

  .request-info {
    flex: 1;
  }

  .request-message {
    font-style: italic;
    color: #666;
    margin: 0.25rem 0;
  }

  .request-date {
    font-size: 12px;
    color: #999;
  }

  .request-actions {
    display: flex;
    gap: 0.5rem;
  }

  .friend-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    border: 1px solid #ddd;
    border-radius: 6px;
    margin-bottom: 0.5rem;
  }

  .friend-info {
    flex: 1;
  }

  .no-items,
  .loading {
    color: #666;
    font-style: italic;
    text-align: center;
    padding: 1rem;
  }

  /* Button styles */
  .btn-primary {
    background: #007bff;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .btn-primary:hover:not(:disabled) {
    background: #0056b3;
  }

  .btn-primary:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: #6c757d;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .btn-secondary:hover {
    background: #545b62;
  }

  .btn-success {
    background: #28a745;
    color: white;
    border: none;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .btn-success:hover {
    background: #218838;
  }

  .btn-danger {
    background: #dc3545;
    color: white;
    border: none;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .btn-danger:hover {
    background: #c82333;
  }

  .btn-sm {
    padding: 0.25rem 0.5rem;
    font-size: 12px;
  }
</style>
