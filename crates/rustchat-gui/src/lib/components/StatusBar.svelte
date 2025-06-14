<script lang="ts">
  import { onMount } from 'svelte';
  import { tauriApi, networkManager, windowManager, logManager } from '../tauri-api';

  interface Props {
    serverStatus?: string;
    appInfo?: any;
  }

  let { serverStatus = 'unknown', appInfo = null }: Props = $props();

  let windowState = $state({ maximized: false, minimized: false, visible: true, focused: true });
  let windowSize = $state({ width: 0, height: 0 });
  let showDebugInfo = $state(false);

  onMount(async () => {
    // 获取窗口状态
    try {
      windowState = await windowManager.getState();
      windowSize = await windowManager.getSize();
    } catch (error) {
      console.error('Failed to get window state:', error);
    }

    // 定期更新状态
    setInterval(async () => {
      try {
        windowState = await windowManager.getState();
        windowSize = await windowManager.getSize();
      } catch (error) {
        // 静默处理错误
      }
    }, 5000);
  });

  async function refreshServerStatus() {
    try {
      const result = await networkManager.testServerConnection();
      serverStatus = result.success ? 'connected' : 'disconnected';
      
      await tauriApi.showNotification(
        'Server Status', 
        `Connection ${result.success ? 'successful' : 'failed'} (${result.response_time_ms}ms)`
      );
    } catch (error) {
      serverStatus = 'error';
      await logManager.error(`Server status check failed: ${error}`);
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'connected': return '#22c55e';
      case 'disconnected': return '#ef4444';
      case 'error': return '#f59e0b';
      default: return '#6b7280';
    }
  }

  function getStatusText(status: string): string {
    switch (status) {
      case 'connected': return 'Connected';
      case 'disconnected': return 'Disconnected';
      case 'error': return 'Error';
      default: return 'Unknown';
    }
  }
</script>

<div class="status-bar">
  <div class="status-left">    <!-- 服务器状态 -->
    <button 
      class="status-item status-button" 
      onclick={refreshServerStatus}
      role="button"
      tabindex="0"
      aria-label="Refresh server status"
    >
      <div 
        class="status-indicator" 
        style="background-color: {getStatusColor(serverStatus)}"
      ></div>
      <span>Server: {getStatusText(serverStatus)}</span>
    </button>

    <!-- 应用信息 -->
    {#if appInfo}
      <div class="status-item">
        <span>v{appInfo.version}</span>
      </div>
    {/if}
  </div>

  <div class="status-right">
    <!-- 窗口信息 -->
    <div class="status-item">
      <span>{windowSize.width}×{windowSize.height}</span>
    </div>    <!-- 调试信息切换 -->
    <button 
      class="debug-toggle"
      onclick={() => showDebugInfo = !showDebugInfo}
      title="Toggle debug info"
      aria-label="Toggle debug information panel"
    >
      🐛
    </button>
  </div>
</div>

{#if showDebugInfo}
  <div class="debug-panel">
    <h4>Debug Information</h4>
    <div class="debug-grid">
      <div class="debug-section">
        <h5>Window State</h5>
        <ul>
          <li>Maximized: {windowState.maximized}</li>
          <li>Minimized: {windowState.minimized}</li>
          <li>Visible: {windowState.visible}</li>
          <li>Focused: {windowState.focused}</li>
          <li>Size: {windowSize.width}×{windowSize.height}</li>
        </ul>
      </div>

      <div class="debug-section">
        <h5>Application</h5>
        {#if appInfo}
          <ul>
            <li>Name: {appInfo.name}</li>
            <li>Version: {appInfo.version}</li>
            <li>Author: {appInfo.author}</li>
          </ul>
        {/if}
      </div>

      <div class="debug-section">
        <h5>Server Status</h5>
        <ul>
          <li>Status: {getStatusText(serverStatus)}</li>
          <li>Last check: {new Date().toLocaleTimeString()}</li>
        </ul>
      </div>
    </div>    <div class="debug-actions">
      <button onclick={() => windowManager.center()}>Center Window</button>
      <button onclick={() => logManager.info('Debug info viewed')}>Log Debug View</button>
      <button onclick={refreshServerStatus}>Refresh Server Status</button>
    </div>
  </div>
{/if}

<style>
  .status-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 12px;
    background: var(--color-bg-secondary, #f8f9fa);
    border-top: 1px solid var(--color-border, #e9ecef);
    font-size: 12px;
    color: var(--color-text-secondary, #6c757d);
    min-height: 24px;
  }

  .status-left, .status-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    transition: color 0.2s;
  }

  .status-button {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    transition: background-color 0.2s, color 0.2s;
    font-size: 12px;
    color: var(--color-text-secondary, #6c757d);
  }

  .status-button:hover {
    background-color: var(--color-bg-hover, #e9ecef);
    color: var(--color-text, #212529);
  }

  .status-button:focus {
    outline: 2px solid var(--color-primary, #007bff);
    outline-offset: 2px;
  }

  .status-item:hover {
    color: var(--color-text, #212529);
  }

  .status-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    transition: background-color 0.3s;
  }

  .debug-toggle {
    background: none;
    border: none;
    font-size: 14px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background-color 0.2s;
  }

  .debug-toggle:hover {
    background-color: var(--color-bg-hover, #e9ecef);
  }

  .debug-panel {
    position: fixed;
    bottom: 32px;
    right: 16px;
    width: 600px;
    max-height: 400px;
    background: var(--color-bg, #ffffff);
    border: 1px solid var(--color-border, #e9ecef);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 1000;
    padding: 16px;
    overflow-y: auto;
  }

  .debug-panel h4 {
    margin: 0 0 12px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--color-text, #212529);
  }

  .debug-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 16px;
    margin-bottom: 16px;
  }

  .debug-section h5 {
    margin: 0 0 8px 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-secondary, #6c757d);
    text-transform: uppercase;
  }

  .debug-section ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .debug-section li {
    font-size: 11px;
    padding: 2px 0;
    color: var(--color-text, #212529);
  }

  .debug-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .debug-actions button {
    padding: 4px 8px;
    font-size: 11px;
    background: var(--color-primary, #007bff);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .debug-actions button:hover {
    background: var(--color-primary-dark, #0056b3);
  }

  /* 暗色主题支持 */
  @media (prefers-color-scheme: dark) {
    .status-bar {
      background: #2d3748;
      border-top-color: #4a5568;
      color: #a0aec0;
    }

    .debug-panel {
      background: #2d3748;
      border-color: #4a5568;
      color: #e2e8f0;
    }

    .debug-toggle:hover {
      background-color: #4a5568;
    }
  }
</style>
