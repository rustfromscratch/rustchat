<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from './Sidebar.svelte';
  import ChatArea from './ChatArea.svelte';
  import RoomList from './RoomList.svelte';
  import StatusBar from './StatusBar.svelte';
  import { actions, currentRoom, rooms, user } from '../store';
  import { roomApi, clearTokens } from '../api';

  interface Props {
    serverStatus?: string;
    appInfo?: any;
  }

  let { serverStatus = 'unknown', appInfo = null }: Props = $props();

  let showRoomList = $state(true);
  let loading = $state(false);

  onMount(async () => {
    await loadRooms();
  });

  async function loadRooms() {
    loading = true;
    try {
      const response = await roomApi.getRooms();
      if (response.data) {
        actions.setRooms(response.data);
      }
    } catch (error) {
      console.error('Failed to load rooms:', error);
    } finally {
      loading = false;
    }
  }

  function handleLogout() {
    clearTokens();
    actions.reset();
  }

  function toggleRoomList() {
    showRoomList = !showRoomList;
  }
</script>

<div class="main-layout">
  <div class="main-content">
    <!-- 侧边栏 -->
    <Sidebar 
      {showRoomList}
      onToggleRoomList={toggleRoomList}
      onLogout={handleLogout}
    />

    <!-- 主要内容区域 -->
    <div class="content-area">
      {#if $currentRoom}
        <ChatArea />
      {:else}
        <div class="welcome-area">
          <div class="welcome-content">
            <h2>Welcome to RustChat! 🦀</h2>
            <p>Select a room from the sidebar to start chatting, or create a new room.</p>
            {#if $rooms.length === 0}
              <p>No rooms available yet. Be the first to create one!</p>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- 房间列表侧边栏 -->
    {#if showRoomList}
      <RoomList onRefresh={loadRooms} {loading} />
    {/if}
  </div>
  
  <!-- 状态栏 -->
  <StatusBar {serverStatus} {appInfo} />
</div>

<style>  .main-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-color: #f5f5f5;
  }

  .main-content {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0; /* 确保 flex 子元素可以收缩 */
  }

  .welcome-area {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: white;
    margin: 20px;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .welcome-content {
    text-align: center;
    padding: 40px;
    max-width: 500px;
  }

  .welcome-content h2 {
    color: #333;
    margin-bottom: 16px;
    font-size: 28px;
    font-weight: 600;
  }

  .welcome-content p {
    color: #666;
    margin-bottom: 12px;
    font-size: 16px;
    line-height: 1.5;
  }
</style>
