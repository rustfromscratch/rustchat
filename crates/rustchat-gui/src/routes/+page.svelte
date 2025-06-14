<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { isAuthenticated } from '../lib/store';
  import LoginPage from '../lib/components/LoginPage.svelte';
  import MainLayout from '../lib/components/MainLayout.svelte';
  import { isAuthenticated as checkAuth } from '../lib/api';
  import { 
    initializeTauriApi, 
    cleanupTauriApi, 
    tauriApi, 
    settingsManager, 
    logManager, 
    networkManager 
  } from '../lib/tauri-api';
  let authenticated = $state(false);
  let loading = $state(true);
  let initError = $state('');
  let appInfo = $state<any>(null);
  let serverStatus = $state<string>('unknown');

  onMount(async () => {
    try {
      // 初始化 Tauri API
      const { appInfo: info } = await initializeTauriApi();
      appInfo = info;
      
      // 检查服务器连接状态
      await checkServerConnection();
      
      // 显示欢迎通知
      if (await settingsManager.getNotificationsEnabled()) {
        await tauriApi.showNotification(
          'Welcome to RustChat!', 
          `Desktop application v${info.version} is ready to use.`
        );
      }
      
      // 检查本地存储中的认证状态
      authenticated = checkAuth();
      
      // 记录启动日志
      await logManager.info(`Application started successfully - v${info.version}`);
    } catch (error) {
      console.error('Initialization error:', error);
      initError = `Failed to initialize application: ${error}`;
      await logManager.error(`Initialization failed: ${error}`);
    } finally {
      loading = false;
    }
  });

  onDestroy(async () => {
    await cleanupTauriApi();
  });

  async function checkServerConnection() {
    try {
      const result = await networkManager.testServerConnection();
      serverStatus = result.success ? 'connected' : 'disconnected';
      
      if (!result.success) {
        await logManager.warn(`Server connection failed: ${result.error}`);
      } else {
        await logManager.info(`Server connection successful (${result.response_time_ms}ms)`);
      }
    } catch (error) {
      serverStatus = 'error';
      await logManager.error(`Server connection check failed: ${error}`);
    }
  }

  // 监听认证状态变化
  $effect(() => {
    const unsubscribe = isAuthenticated.subscribe(value => {
      authenticated = value;
    });
    return unsubscribe;
  });
</script>

{#if loading}
  <div class="loading">
    <div class="spinner"></div>
    <p>Initializing RustChat...</p>
    {#if appInfo}
      <p class="app-info">v{appInfo.version}</p>
    {/if}
  </div>
{:else if initError}
  <div class="error-page">
    <div class="error-content">
      <h2>❌ Initialization Error</h2>
      <p>{initError}</p>
      <button onclick={() => window.location.reload()}>
        🔄 Retry
      </button>
    </div>
  </div>
{:else if authenticated}
  <MainLayout {serverStatus} {appInfo} />
{:else}
  <LoginPage />
{/if}

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Helvetica', 'Arial', sans-serif;
    background-color: #f5f5f5;
    color: #333;
  }

  :global(*) {
    box-sizing: border-box;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid rgba(255, 255, 255, 0.3);
    border-top: 4px solid white;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 16px;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .loading p {
    font-size: 16px;
    margin: 8px 0;
  }

  .app-info {
    font-size: 12px;
    opacity: 0.8;
  }

  .error-page {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background-color: #f8f8f8;
  }

  .error-content {
    background: white;
    border-radius: 12px;
    padding: 40px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
    text-align: center;
    max-width: 400px;
  }

  .error-content h2 {
    color: #e74c3c;
    margin-bottom: 16px;
  }

  .error-content p {
    color: #666;
    margin-bottom: 24px;
    line-height: 1.5;
  }

  .error-content button {
    background: #3498db;
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    transition: all 0.2s ease;
  }

  .error-content button:hover {
    background: #2980b9;
    transform: translateY(-2px);
  }
</style>
