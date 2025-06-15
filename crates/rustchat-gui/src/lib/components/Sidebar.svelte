<script lang="ts">
  import { user } from '../store';
  import SettingsPage from './SettingsPage.svelte';

  interface Props {
    showRoomList: boolean;
    showFriends: boolean;
    onToggleRoomList: () => void;
    onToggleFriends: () => void;
    onLogout: () => void;
  }

  let { showRoomList, showFriends, onToggleRoomList, onToggleFriends, onLogout }: Props = $props();
  let showSettings = $state(false);
</script>

<div class="sidebar">
  <div class="sidebar-header">
    <div class="app-title">
      <span class="app-icon">🦀</span>
      <span class="app-name">RustChat</span>
    </div>
  </div>
  <nav class="sidebar-nav">
    <button 
      class="nav-item {showRoomList ? 'active' : ''}"
      onclick={onToggleRoomList}
    >
      <span class="nav-icon">💬</span>
      <span class="nav-text">Rooms</span>
    </button>

    <button 
      class="nav-item {showFriends ? 'active' : ''}"
      onclick={onToggleFriends}
    >
      <span class="nav-icon">👥</span>
      <span class="nav-text">Friends</span>
    </button>

    <button 
      class="nav-item {showSettings ? 'active' : ''}"
      onclick={() => showSettings = true}
    >
      <span class="nav-icon">⚙️</span>
      <span class="nav-text">Settings</span>
    </button>
  </nav>

  <div class="sidebar-footer">
    <div class="user-info">
      <div class="user-avatar">
        {$user?.email?.[0]?.toUpperCase() || '?'}
      </div>
      <div class="user-details">
        <div class="user-email">{$user?.email || 'Unknown'}</div>
        <div class="user-status">Online</div>
      </div>
    </div>
    
    <button class="logout-btn" onclick={onLogout} title="Logout">
      <span>🚪</span>
    </button>
  </div>
</div>

{#if showSettings}
  <SettingsPage onClose={() => showSettings = false} />
{/if}

<style>
  .sidebar {
    width: 240px;
    background-color: #2c3e50;
    color: white;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 20px 16px;
    border-bottom: 1px solid #34495e;
  }

  .app-title {
    display: flex;
    align-items: center;
    font-weight: 600;
    font-size: 18px;
  }

  .app-icon {
    margin-right: 8px;
    font-size: 20px;
  }

  .sidebar-nav {
    flex: 1;
    padding: 16px 0;
  }

  .nav-item {
    width: 100%;
    display: flex;
    align-items: center;
    padding: 12px 16px;
    background: none;
    border: none;
    color: #bdc3c7;
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 14px;
  }

  .nav-item:hover {
    background-color: #34495e;
    color: white;
  }

  .nav-item.active {
    background-color: #3498db;
    color: white;
  }

  .nav-icon {
    margin-right: 12px;
    font-size: 16px;
    width: 20px;
    text-align: center;
  }

  .sidebar-footer {
    padding: 16px;
    border-top: 1px solid #34495e;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .user-info {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
  }

  .user-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background-color: #3498db;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    font-size: 14px;
    margin-right: 10px;
    flex-shrink: 0;
  }

  .user-details {
    min-width: 0;
    flex: 1;
  }

  .user-email {
    font-size: 12px;
    font-weight: 500;
    color: white;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .user-status {
    font-size: 11px;
    color: #95a5a6;
    margin-top: 2px;
  }

  .logout-btn {
    background: none;
    border: none;
    color: #bdc3c7;
    cursor: pointer;
    padding: 6px;
    border-radius: 4px;
    transition: all 0.2s ease;
    font-size: 16px;
  }

  .logout-btn:hover {
    background-color: #e74c3c;
    color: white;
  }
</style>
