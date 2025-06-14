<script lang="ts">
  import { onMount } from 'svelte';
  import { tauriApi, settingsManager } from '../tauri-api';

  interface Props {
    onClose?: () => void;
  }

  let { onClose }: Props = $props();
  let theme = $state<string>('light');
  let notificationsEnabled = $state(true);
  let serverUrl = $state('http://localhost:3000');
  let autoConnect = $state(true);
  let loading = $state(true);
  let saving = $state(false);
  let message = $state('');
  let appInfo = $state<any>(null);
  let systemInfo = $state<any>(null);

  onMount(async () => {
    await loadSettings();
    await loadSystemInfo();
    loading = false;
  });

  async function loadSettings() {
    try {
      theme = await settingsManager.getTheme();
      notificationsEnabled = await settingsManager.getNotificationsEnabled();
      serverUrl = await settingsManager.getServerUrl();
      autoConnect = await settingsManager.getAutoConnect();
    } catch (error) {
      console.error('Failed to load settings:', error);
      message = 'Failed to load settings';
    }
  }

  async function loadSystemInfo() {
    try {
      appInfo = await tauriApi.getAppInfo();
      systemInfo = await tauriApi.getSystemInfo();
    } catch (error) {
      console.error('Failed to load system info:', error);
    }
  }

  async function saveSettings() {
    saving = true;
    message = '';    try {
      await settingsManager.setTheme(theme as 'light' | 'dark');
      await settingsManager.setNotificationsEnabled(notificationsEnabled);
      await settingsManager.setServerUrl(serverUrl);
      await settingsManager.setAutoConnect(autoConnect);

      message = 'Settings saved successfully!';
      
      // 显示通知（如果启用）
      if (notificationsEnabled) {
        await tauriApi.showNotification('Settings Updated', 'Your preferences have been saved.');
      }
    } catch (error) {
      console.error('Failed to save settings:', error);
      message = 'Failed to save settings';
    } finally {
      saving = false;
    }
  }

  async function resetToDefaults() {
    if (confirm('Are you sure you want to reset all settings to their default values?')) {
      saving = true;
      try {
        await tauriApi.resetSettings();
        await loadSettings();
        message = 'Settings reset to defaults';
        
        if (notificationsEnabled) {
          await tauriApi.showNotification('Settings Reset', 'All settings have been reset to default values.');
        }
      } catch (error) {
        console.error('Failed to reset settings:', error);
        message = 'Failed to reset settings';
      } finally {
        saving = false;
      }
    }
  }

  async function testConnection() {
    saving = true;
    try {
      const isConnected = await tauriApi.checkConnection(serverUrl);
      message = isConnected ? 'Connection successful!' : 'Connection failed';
      
      await tauriApi.showNotification(
        'Connection Test',
        isConnected ? 'Server is reachable' : 'Unable to connect to server'
      );
    } catch (error) {
      console.error('Connection test failed:', error);
      message = 'Connection test failed';
    } finally {
      saving = false;
    }
  }

  async function openGitHub() {
    try {
      await tauriApi.openExternalLink('https://github.com/your-username/rustchat');
    } catch (error) {
      console.error('Failed to open GitHub:', error);
    }
  }
</script>

<div 
  class="settings-overlay" 
  role="dialog" 
  aria-modal="true" 
  onclick={onClose} 
  onkeydown={(e) => e.key === 'Escape' && onClose?.()}
  tabindex="-1"
>
  <div 
    class="settings-modal" 
    role="document" 
    onclick={(e) => e.stopPropagation()}
    tabindex="0"
  >
    <div class="settings-header">
      <h2>⚙️ Settings</h2>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <p>Loading settings...</p>
      </div>
    {:else}
      <div class="settings-content">
        <!-- 外观设置 -->
        <section class="settings-section">
          <h3>🎨 Appearance</h3>
          
          <div class="setting-item">
            <label for="theme">Theme</label>
            <select id="theme" bind:value={theme} disabled={saving}>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="auto">Auto</option>
            </select>
          </div>
        </section>

        <!-- 通知设置 -->
        <section class="settings-section">
          <h3>🔔 Notifications</h3>
          
          <div class="setting-item">
            <label>
              <input
                type="checkbox"
                bind:checked={notificationsEnabled}
                disabled={saving}
              />
              Enable notifications
            </label>
          </div>
        </section>

        <!-- 连接设置 -->
        <section class="settings-section">
          <h3>🌐 Connection</h3>
          
          <div class="setting-item">
            <label for="serverUrl">Server URL</label>
            <div class="input-group">
              <input
                id="serverUrl"
                type="url"
                bind:value={serverUrl}
                disabled={saving}
                placeholder="http://localhost:3000"
              />
              <button onclick={testConnection} disabled={saving} class="test-btn">
                Test
              </button>
            </div>
          </div>

          <div class="setting-item">
            <label>
              <input
                type="checkbox"
                bind:checked={autoConnect}
                disabled={saving}
              />
              Auto-connect on startup
            </label>
          </div>
        </section>

        <!-- 关于信息 -->
        {#if appInfo || systemInfo}
          <section class="settings-section">
            <h3>ℹ️ About</h3>
            
            {#if appInfo}
              <div class="info-item">
                <strong>{appInfo.name}</strong>
                <span>v{appInfo.version}</span>
              </div>
              <div class="info-item">
                <span>{appInfo.description}</span>
              </div>
            {/if}

            {#if systemInfo}
              <div class="info-item">
                <strong>Platform:</strong>
                <span>{systemInfo.platform} ({systemInfo.arch})</span>
              </div>
            {/if}

            <div class="info-actions">
              <button onclick={openGitHub} class="link-btn">
                📂 GitHub Repository
              </button>
            </div>
          </section>
        {/if}

        <!-- 消息显示 -->
        {#if message}
          <div class="message {message.includes('Failed') || message.includes('failed') ? 'error' : 'success'}">
            {message}
          </div>
        {/if}

        <!-- 操作按钮 -->
        <div class="settings-actions">
          <button onclick={saveSettings} disabled={saving} class="primary">
            {saving ? 'Saving...' : 'Save Settings'}
          </button>
          
          <button onclick={resetToDefaults} disabled={saving} class="secondary">
            Reset to Defaults
          </button>
          
          <button onclick={onClose} disabled={saving} class="secondary">
            Cancel
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .settings-modal {
    background: white;
    border-radius: 12px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    width: 90%;
    max-width: 600px;
    max-height: 80vh;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid #e1e8ed;
    background: #f8f9fa;
  }

  .settings-header h2 {
    margin: 0;
    color: #333;
    font-size: 20px;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: #666;
    padding: 4px 8px;
    border-radius: 4px;
    transition: all 0.2s ease;
  }

  .close-btn:hover {
    background: #e1e8ed;
    color: #333;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px;
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

  .settings-content {
    overflow-y: auto;
    padding: 20px 24px;
  }

  .settings-section {
    margin-bottom: 32px;
  }

  .settings-section h3 {
    margin: 0 0 16px;
    color: #333;
    font-size: 16px;
    font-weight: 600;
  }

  .setting-item {
    margin-bottom: 16px;
  }

  .setting-item label {
    display: block;
    margin-bottom: 6px;
    color: #555;
    font-weight: 500;
    font-size: 14px;
  }

  .setting-item input[type="checkbox"] {
    margin-right: 8px;
  }

  .setting-item input,
  .setting-item select {
    width: 100%;
    padding: 10px 12px;
    border: 2px solid #e1e5e9;
    border-radius: 6px;
    font-size: 14px;
    transition: border-color 0.2s ease;
  }

  .setting-item input:focus,
  .setting-item select:focus {
    outline: none;
    border-color: #3498db;
  }

  .input-group {
    display: flex;
    gap: 8px;
  }

  .input-group input {
    flex: 1;
  }

  .test-btn {
    background: #2ecc71;
    color: white;
    border: none;
    padding: 10px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .test-btn:hover:not(:disabled) {
    background: #27ae60;
  }

  .test-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .info-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid #f0f0f0;
  }

  .info-item:last-child {
    border-bottom: none;
  }

  .info-actions {
    margin-top: 16px;
  }

  .link-btn {
    background: none;
    border: 1px solid #3498db;
    color: #3498db;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.2s ease;
  }

  .link-btn:hover {
    background: #3498db;
    color: white;
  }

  .message {
    padding: 12px 16px;
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 14px;
  }

  .message.success {
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
  }

  .message.error {
    background: #f8d7da;
    color: #721c24;
    border: 1px solid #f5c6cb;
  }

  .settings-actions {
    display: flex;
    gap: 12px;
    padding-top: 16px;
    border-top: 1px solid #e1e8ed;
  }

  .settings-actions button {
    flex: 1;
    padding: 12px 24px;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .settings-actions button.primary {
    background: #3498db;
    color: white;
  }

  .settings-actions button.primary:hover:not(:disabled) {
    background: #2980b9;
  }

  .settings-actions button.secondary {
    background: #f8f9fa;
    color: #666;
    border: 1px solid #ddd;
  }

  .settings-actions button.secondary:hover:not(:disabled) {
    background: #e8e8e8;
  }

  .settings-actions button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
