use tauri::{Manager, Emitter};
use std::collections::HashMap;
use std::sync::Mutex;
use std::io::Write;

// 全局状态管理
pub struct AppState {
    pub settings: Mutex<HashMap<String, serde_json::Value>>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut default_settings = HashMap::new();
        default_settings.insert("theme".to_string(), serde_json::json!("light"));
        default_settings.insert("notifications".to_string(), serde_json::json!(true));
        default_settings.insert("server_url".to_string(), serde_json::json!("http://localhost:3000"));
        default_settings.insert("auto_connect".to_string(), serde_json::json!(true));
        
        Self {
            settings: Mutex::new(default_settings),
        }
    }
}

// 学习更多关于 Tauri 命令的信息：https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 获取应用信息
#[tauri::command]
fn get_app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "RustChat",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "A modern chat application built with Rust and Tauri",
        "author": "RustChat Team",
        "build_date": env!("CARGO_PKG_VERSION")
    })
}

// 保存用户设置到内存和文件
#[tauri::command]
async fn save_setting(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    // 更新内存中的设置
    {
        let mut settings = state.settings.lock().map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.insert(key.clone(), value.clone());
    }
    
    // 保存到文件
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    // 确保目录存在
    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app directory: {}", e))?;
    
    let settings_path = app_dir.join("settings.json");
    
    // 读取现有设置或创建新的
    let mut all_settings: HashMap<String, serde_json::Value> = if settings_path.exists() {
        let settings_str = std::fs::read_to_string(&settings_path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;
        serde_json::from_str(&settings_str)
            .map_err(|e| format!("Failed to parse settings: {}", e))?
    } else {
        HashMap::new()
    };
    
    // 更新设置
    all_settings.insert(key, value);
    
    // 写回文件
    let settings_str = serde_json::to_string_pretty(&all_settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    std::fs::write(settings_path, settings_str)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    
    Ok(())
}

// 获取单个设置
#[tauri::command]
fn get_setting(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().map_err(|e| format!("Failed to lock settings: {}", e))?;
    
    Ok(settings.get(&key).cloned().unwrap_or(serde_json::Value::Null))
}

// 获取所有设置
#[tauri::command]
fn get_all_settings(
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let settings = state.settings.lock().map_err(|e| format!("Failed to lock settings: {}", e))?;
    Ok(settings.clone())
}

// 加载用户设置从文件
#[tauri::command]
async fn load_settings(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let settings_path = app_dir.join("settings.json");
    
    let loaded_settings: HashMap<String, serde_json::Value> = if settings_path.exists() {
        let settings_str = std::fs::read_to_string(settings_path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        serde_json::from_str(&settings_str)
            .map_err(|e| format!("Failed to parse settings: {}", e))?
    } else {
        HashMap::new()
    };
    
    // 合并默认设置和加载的设置
    {
        let mut settings = state.settings.lock().map_err(|e| format!("Failed to lock settings: {}", e))?;
        for (key, value) in loaded_settings.iter() {
            settings.insert(key.clone(), value.clone());
        }
    }
    
    Ok(loaded_settings)
}

// 重置设置到默认值
#[tauri::command]
async fn reset_settings(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 重置内存中的设置
    {
        let mut settings = state.settings.lock().map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.clear();
        settings.insert("theme".to_string(), serde_json::json!("light"));
        settings.insert("notifications".to_string(), serde_json::json!(true));
        settings.insert("server_url".to_string(), serde_json::json!("http://localhost:3000"));
        settings.insert("auto_connect".to_string(), serde_json::json!(true));
    }
    
    // 删除设置文件
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let settings_path = app_dir.join("settings.json");
    if settings_path.exists() {
        std::fs::remove_file(settings_path)
            .map_err(|e| format!("Failed to remove settings file: {}", e))?;
    }
    
    Ok(())
}

// 显示系统通知
#[tauri::command]
async fn show_notification(
    app_handle: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    // 注意：在实际应用中，您可能想要使用 tauri-plugin-notification
    // 这里我们使用一个简单的实现
    println!("Notification: {} - {}", title, body);
    
    // 可以发送事件到前端
    app_handle.emit("notification", serde_json::json!({
        "title": title,
        "body": body,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })).map_err(|e| format!("Failed to emit notification event: {}", e))?;
    
    Ok(())
}

// 获取系统信息
#[tauri::command]
fn get_system_info() -> serde_json::Value {
    serde_json::json!({
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "family": std::env::consts::FAMILY,
        "exe_suffix": std::env::consts::EXE_SUFFIX,
        "dll_suffix": std::env::consts::DLL_SUFFIX
    })
}

// 检查网络连接状态
#[tauri::command]
async fn check_connection(url: String) -> Result<bool, String> {
    // 简单的连接检查
    match reqwest::get(&url).await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

// 获取应用数据目录路径
#[tauri::command]
async fn get_app_data_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    Ok(app_dir.to_string_lossy().to_string())
}

// 获取应用日志目录路径
#[tauri::command]
async fn get_app_log_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get app log directory: {}", e))?;
    
    Ok(log_dir.to_string_lossy().to_string())
}

// 写入日志文件
#[tauri::command]
async fn write_log(
    app_handle: tauri::AppHandle,
    level: String,
    message: String,
) -> Result<(), String> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get app log directory: {}", e))?;
    
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("Failed to create log directory: {}", e))?;
    
    let log_file = log_dir.join("rustchat.log");
    let timestamp = chrono::Utc::now().to_rfc3339();
    let log_entry = format!("[{}] [{}] {}\n", timestamp, level.to_uppercase(), message);
    
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .map_err(|e| format!("Failed to open log file: {}", e))?
        .write_all(log_entry.as_bytes())
        .map_err(|e| format!("Failed to write log: {}", e))?;
    
    Ok(())
}

// 读取日志文件（最近N行）
#[tauri::command]
async fn read_logs(
    app_handle: tauri::AppHandle,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get app log directory: {}", e))?;
    
    let log_file = log_dir.join("rustchat.log");
    
    if !log_file.exists() {
        return Ok(vec![]);
    }
    
    let content = std::fs::read_to_string(log_file)
        .map_err(|e| format!("Failed to read log file: {}", e))?;
    
    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    
    let result = if let Some(n) = lines {
        all_lines.into_iter().rev().take(n).rev().collect()
    } else {
        all_lines
    };
    
    Ok(result)
}

// 清理日志文件
#[tauri::command]
async fn clear_logs(app_handle: tauri::AppHandle) -> Result<(), String> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get app log directory: {}", e))?;
    
    let log_file = log_dir.join("rustchat.log");
    
    if log_file.exists() {
        std::fs::remove_file(log_file)
            .map_err(|e| format!("Failed to remove log file: {}", e))?;
    }
    
    Ok(())
}

// 获取窗口状态
#[tauri::command]
async fn get_window_state(app_handle: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let main_window = app_handle.get_webview_window("main")
        .ok_or("Main window not found")?;
    
    let is_maximized = main_window.is_maximized()
        .map_err(|e| format!("Failed to check if maximized: {}", e))?;
    
    let is_minimized = main_window.is_minimized()
        .map_err(|e| format!("Failed to check if minimized: {}", e))?;
    
    let is_visible = main_window.is_visible()
        .map_err(|e| format!("Failed to check if visible: {}", e))?;
    
    let is_focused = main_window.is_focused()
        .map_err(|e| format!("Failed to check if focused: {}", e))?;
    
    Ok(serde_json::json!({
        "maximized": is_maximized,
        "minimized": is_minimized,
        "visible": is_visible,
        "focused": is_focused
    }))
}

// 控制窗口状态
#[tauri::command]
async fn set_window_state(
    app_handle: tauri::AppHandle,
    action: String,
) -> Result<(), String> {
    let main_window = app_handle.get_webview_window("main")
        .ok_or("Main window not found")?;
    
    match action.as_str() {
        "minimize" => main_window.minimize().map_err(|e| format!("Failed to minimize: {}", e))?,
        "maximize" => main_window.maximize().map_err(|e| format!("Failed to maximize: {}", e))?,
        "unmaximize" => main_window.unmaximize().map_err(|e| format!("Failed to unmaximize: {}", e))?,
        "show" => main_window.show().map_err(|e| format!("Failed to show: {}", e))?,
        "hide" => main_window.hide().map_err(|e| format!("Failed to hide: {}", e))?,
        "focus" => main_window.set_focus().map_err(|e| format!("Failed to focus: {}", e))?,
        "center" => main_window.center().map_err(|e| format!("Failed to center: {}", e))?,
        _ => return Err(format!("Unknown action: {}", action)),
    }
    
    Ok(())
}

// 获取和设置窗口大小
#[tauri::command]
async fn get_window_size(app_handle: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let main_window = app_handle.get_webview_window("main")
        .ok_or("Main window not found")?;
    
    let size = main_window.inner_size()
        .map_err(|e| format!("Failed to get window size: {}", e))?;
    
    Ok(serde_json::json!({
        "width": size.width,
        "height": size.height
    }))
}

#[tauri::command]
async fn set_window_size(
    app_handle: tauri::AppHandle,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let main_window = app_handle.get_webview_window("main")
        .ok_or("Main window not found")?;
    
    let size = tauri::LogicalSize::new(width, height);
    main_window.set_size(size)
        .map_err(|e| format!("Failed to set window size: {}", e))?;
    
    Ok(())
}

// 验证服务器连接
#[tauri::command]
async fn validate_server_connection(url: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    
    match client.get(&url).timeout(std::time::Duration::from_secs(10)).send().await {
        Ok(response) => {
            let duration = start_time.elapsed();
            let status = response.status();
            let headers = response.headers().clone();
            
            // 尝试获取服务器信息
            let server_info = if let Ok(text) = response.text().await {
                if text.len() < 1000 { // 避免返回过大的响应
                    Some(text)
                } else {
                    Some(format!("Response too large ({} chars)", text.len()))
                }
            } else {
                None
            };
            
            Ok(serde_json::json!({
                "success": true,
                "status": status.as_u16(),
                "status_text": status.to_string(),
                "response_time_ms": duration.as_millis(),
                "server_info": server_info,
                "headers": headers.iter().map(|(k, v)| {
                    (k.to_string(), v.to_str().unwrap_or("").to_string())
                }).collect::<std::collections::HashMap<String, String>>()
            }))
        }
        Err(e) => {
            let duration = start_time.elapsed();
            Ok(serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "response_time_ms": duration.as_millis()
            }))
        }
    }
}

// 导出设置到文件
#[tauri::command]
async fn export_settings(
    app_handle: tauri::AppHandle,
    file_path: String,
) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let settings_path = app_dir.join("settings.json");
    
    if settings_path.exists() {
        std::fs::copy(settings_path, file_path)
            .map_err(|e| format!("Failed to export settings: {}", e))?;
    } else {
        return Err("No settings file found to export".to_string());
    }
    
    Ok(())
}

// 导入设置从文件
#[tauri::command]
async fn import_settings(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    file_path: String,
) -> Result<(), String> {
    // 验证文件是否存在
    if !std::path::Path::new(&file_path).exists() {
        return Err("Settings file does not exist".to_string());
    }
    
    // 读取并验证JSON格式
    let settings_content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    let imported_settings: HashMap<String, serde_json::Value> = serde_json::from_str(&settings_content)
        .map_err(|e| format!("Invalid settings file format: {}", e))?;
    
    // 更新内存中的设置
    {
        let mut settings = state.settings.lock().map_err(|e| format!("Failed to lock settings: {}", e))?;
        settings.clear();
        for (key, value) in imported_settings.iter() {
            settings.insert(key.clone(), value.clone());
        }
    }
    
    // 保存到应用设置文件
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create app directory: {}", e))?;
    
    let settings_path = app_dir.join("settings.json");
    std::fs::copy(file_path, settings_path)
        .map_err(|e| format!("Failed to import settings: {}", e))?;
    
    Ok(())
}

// 打开外部链接
#[tauri::command]
async fn open_external_link(url: String) -> Result<(), String> {
    match open::that(&url) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to open link: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::default();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            get_app_info,
            save_setting,
            get_setting,
            get_all_settings,
            load_settings,
            reset_settings,
            show_notification,
            get_system_info,
            check_connection,
            get_app_data_dir,
            get_app_log_dir,
            write_log,
            read_logs,
            clear_logs,
            get_window_state,
            set_window_state,
            get_window_size,
            set_window_size,
            validate_server_connection,
            export_settings,
            import_settings,
            open_external_link
        ])
        .setup(|app| {
            // 在这里可以进行应用初始化
            println!("🦀 RustChat GUI is starting...");
            
            // 加载保存的设置
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = load_settings_on_startup(app_handle).await {
                    eprintln!("Failed to load settings on startup: {}", e);
                }
            });
            
            Ok(())
        })        .on_window_event(|_app_handle, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api: _, .. } => {
                    // 在窗口关闭时可以进行清理工作
                    println!("🦀 RustChat GUI is closing...");
                    // api.prevent_close(); // 如果需要阻止关闭
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 启动时加载设置的辅助函数
async fn load_settings_on_startup(app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let state = app_handle.state::<AppState>();
    let app_handle_clone = app_handle.clone();
    let _settings = load_settings(app_handle_clone, state).await?;
    println!("✅ Settings loaded successfully");
    Ok(())
}
