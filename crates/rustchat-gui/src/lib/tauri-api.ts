// Tauri API 服务
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface AppInfo {
  name: string;
  version: string;
  description: string;
  author: string;
  build_date: string;
}

export interface SystemInfo {
  platform: string;
  arch: string;
  family: string;
  exe_suffix: string;
  dll_suffix: string;
}

export interface NotificationEvent {
  title: string;
  body: string;
  timestamp: string;
}

export interface WindowState {
  maximized: boolean;
  minimized: boolean;
  visible: boolean;
  focused: boolean;
}

export interface WindowSize {
  width: number;
  height: number;
}

export interface ServerConnectionResult {
  success: boolean;
  status?: number;
  status_text?: string;
  response_time_ms: number;
  server_info?: string;
  headers?: Record<string, string>;
  error?: string;
}

// Tauri 命令包装器
export const tauriApi = {
  // 基础命令
  async greet(name: string): Promise<string> {
    return await invoke('greet', { name });
  },

  async getAppInfo(): Promise<AppInfo> {
    return await invoke('get_app_info');
  },

  async getSystemInfo(): Promise<SystemInfo> {
    return await invoke('get_system_info');
  },

  // 设置管理
  async saveSetting(key: string, value: any): Promise<void> {
    return await invoke('save_setting', { key, value });
  },

  async getSetting(key: string): Promise<any> {
    return await invoke('get_setting', { key });
  },

  async getAllSettings(): Promise<Record<string, any>> {
    return await invoke('get_all_settings');
  },

  async loadSettings(): Promise<Record<string, any>> {
    return await invoke('load_settings');
  },

  async resetSettings(): Promise<void> {
    return await invoke('reset_settings');
  },

  async exportSettings(filePath: string): Promise<void> {
    return await invoke('export_settings', { filePath });
  },

  async importSettings(filePath: string): Promise<void> {
    return await invoke('import_settings', { filePath });
  },

  // 通知
  async showNotification(title: string, body: string): Promise<void> {
    return await invoke('show_notification', { title, body });
  },

  // 文件系统和目录
  async getAppDataDir(): Promise<string> {
    return await invoke('get_app_data_dir');
  },

  async getAppLogDir(): Promise<string> {
    return await invoke('get_app_log_dir');
  },

  // 日志管理
  async writeLog(level: 'info' | 'warn' | 'error' | 'debug', message: string): Promise<void> {
    return await invoke('write_log', { level, message });
  },

  async readLogs(lines?: number): Promise<string[]> {
    return await invoke('read_logs', { lines });
  },

  async clearLogs(): Promise<void> {
    return await invoke('clear_logs');
  },

  // 窗口管理
  async getWindowState(): Promise<WindowState> {
    return await invoke('get_window_state');
  },

  async setWindowState(action: 'minimize' | 'maximize' | 'unmaximize' | 'show' | 'hide' | 'focus' | 'center'): Promise<void> {
    return await invoke('set_window_state', { action });
  },

  async getWindowSize(): Promise<WindowSize> {
    return await invoke('get_window_size');
  },

  async setWindowSize(width: number, height: number): Promise<void> {
    return await invoke('set_window_size', { width, height });
  },

  // 网络
  async checkConnection(url: string): Promise<boolean> {
    return await invoke('check_connection', { url });
  },

  async validateServerConnection(url: string): Promise<ServerConnectionResult> {
    return await invoke('validate_server_connection', { url });
  },

  // 外部链接
  async openExternalLink(url: string): Promise<void> {
    return await invoke('open_external_link', { url });
  },

  // 事件监听
  async listenToNotifications(callback: (event: NotificationEvent) => void) {
    return await listen<NotificationEvent>('notification', (event) => {
      callback(event.payload);
    });
  },
};

// 设置管理的便捷包装
export const settingsManager = {
  async getTheme(): Promise<string> {
    return (await tauriApi.getSetting('theme')) || 'light';
  },

  async setTheme(theme: 'light' | 'dark'): Promise<void> {
    await tauriApi.saveSetting('theme', theme);
  },

  async getNotificationsEnabled(): Promise<boolean> {
    return (await tauriApi.getSetting('notifications')) ?? true;
  },

  async setNotificationsEnabled(enabled: boolean): Promise<void> {
    await tauriApi.saveSetting('notifications', enabled);
  },
  async getServerUrl(): Promise<string> {
    return (await tauriApi.getSetting('server_url')) || 'http://127.0.0.1:8080';
  },

  async setServerUrl(url: string): Promise<void> {
    await tauriApi.saveSetting('server_url', url);
  },

  async getAutoConnect(): Promise<boolean> {
    return (await tauriApi.getSetting('auto_connect')) ?? true;
  },

  async setAutoConnect(autoConnect: boolean): Promise<void> {
    await tauriApi.saveSetting('auto_connect', autoConnect);
  },
};

// 窗口管理的便捷包装
export const windowManager = {
  async minimize(): Promise<void> {
    await tauriApi.setWindowState('minimize');
  },

  async maximize(): Promise<void> {
    await tauriApi.setWindowState('maximize');
  },

  async unmaximize(): Promise<void> {
    await tauriApi.setWindowState('unmaximize');
  },

  async show(): Promise<void> {
    await tauriApi.setWindowState('show');
  },

  async hide(): Promise<void> {
    await tauriApi.setWindowState('hide');
  },

  async focus(): Promise<void> {
    await tauriApi.setWindowState('focus');
  },

  async center(): Promise<void> {
    await tauriApi.setWindowState('center');
  },

  async getState(): Promise<WindowState> {
    return await tauriApi.getWindowState();
  },

  async getSize(): Promise<WindowSize> {
    return await tauriApi.getWindowSize();
  },

  async setSize(width: number, height: number): Promise<void> {
    await tauriApi.setWindowSize(width, height);
  },
};

// 日志管理的便捷包装
export const logManager = {
  async info(message: string): Promise<void> {
    await tauriApi.writeLog('info', message);
  },

  async warn(message: string): Promise<void> {
    await tauriApi.writeLog('warn', message);
  },

  async error(message: string): Promise<void> {
    await tauriApi.writeLog('error', message);
  },

  async debug(message: string): Promise<void> {
    await tauriApi.writeLog('debug', message);
  },

  async getLogs(lines?: number): Promise<string[]> {
    return await tauriApi.readLogs(lines);
  },

  async clearLogs(): Promise<void> {
    await tauriApi.clearLogs();
  },
};

// 网络工具的便捷包装
export const networkManager = {
  async isServerReachable(url: string): Promise<boolean> {
    return await tauriApi.checkConnection(url);
  },

  async validateConnection(url: string): Promise<ServerConnectionResult> {
    return await tauriApi.validateServerConnection(url);
  },
  async testServerConnection(): Promise<ServerConnectionResult> {
    const serverUrl = await settingsManager.getServerUrl();
    // 测试健康检查端点
    const healthUrl = `${serverUrl}/health`;
    return await this.validateConnection(healthUrl);
  },
};

// 初始化函数
export async function initializeTauriApi() {
  try {
    // 加载设置
    await tauriApi.loadSettings();
    
    // 获取应用信息
    const appInfo = await tauriApi.getAppInfo();
    console.log('🦀 RustChat GUI initialized:', appInfo);
    
    // 获取系统信息
    const systemInfo = await tauriApi.getSystemInfo();
    console.log('💻 System Info:', systemInfo);
    
    // 记录初始化日志
    await logManager.info(`RustChat GUI ${appInfo.version} started on ${systemInfo.platform}`);
    
    return { appInfo, systemInfo };
  } catch (error) {
    console.error('Failed to initialize Tauri API:', error);
    await logManager.error(`Failed to initialize Tauri API: ${error}`);
    throw error;
  }
}

// 清理函数
export async function cleanupTauriApi() {
  try {
    await logManager.info('RustChat GUI is shutting down');
    console.log('🦀 RustChat GUI cleanup completed');
  } catch (error) {
    console.error('Failed to cleanup Tauri API:', error);
  }
}
