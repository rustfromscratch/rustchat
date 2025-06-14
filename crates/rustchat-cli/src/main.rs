mod colors;

use anyhow::{Context, Result};
use colors::ColorDisplay;
use crossterm::ExecutableCommand;
use futures_util::{SinkExt, StreamExt};
use rustchat_core::{UserConfigManager, MessageDatabase};
use rustchat_types::{Message, UserId};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use tracing::{error, info};

/// WebSocket事件类型（与服务器端保持一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum WsEvent {
    Connected { user_id: UserId },
    Message(Message),
    UserJoined { user_id: UserId, nickname: Option<String> },
    UserLeft { user_id: UserId },
    Ping,
    Pong,
    Error { message: String },
}

/// 客户端消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    SendMessage { content: String, nickname: Option<String> },
    SetNickname { nickname: String },
    Pong,
}

/// CLI应用状态
pub struct AppState {
    pub user_id: Option<UserId>,
    pub nickname: Option<String>,
    pub messages: Vec<Message>,
    pub connected: bool,
    pub color_display: ColorDisplay,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            user_id: None,
            nickname: None,
            messages: Vec::new(),
            connected: false,
            color_display: ColorDisplay::new(),
        }
    }
}

/// 显示消息（使用彩色显示）
fn display_message(msg: &Message, color_display: &ColorDisplay) {
    color_display.display_message(msg);
}

/// 处理WebSocket事件（通过通道发送）
async fn handle_ws_event_with_sender(
    event: WsEvent,
    state: Arc<Mutex<AppState>>,
    config_manager: &UserConfigManager,
    message_db: Arc<MessageDatabase>,
    ws_sender: &tokio::sync::mpsc::UnboundedSender<WsMessage>,
    color_display: &ColorDisplay,
) -> Result<()> {
    match event {
        WsEvent::Connected { user_id } => {
            info!("已连接到服务器，服务器分配的用户ID: {}", user_id);
            
            let mut app_state = state.lock().await;
            
            // 检查服务器返回的ID是否与本地存储的ID一致
            if let Some(local_user_id) = &app_state.user_id {
                if local_user_id != &user_id {
                    // 如果不一致，使用服务器分配的新ID并更新本地配置
                    info!("服务器分配了新的用户ID，更新本地配置");
                    let mut config = config_manager.load_config().await?;
                    config.user_id = user_id.clone();
                    config_manager.save_config(&config).await?;
                    
                    app_state.user_id = Some(user_id.clone());
                }
            } else {
                // 如果本地没有用户ID，使用服务器分配的ID
                app_state.user_id = Some(user_id.clone());
                let mut config = config_manager.load_config().await?;
                config.user_id = user_id.clone();
                config_manager.save_config(&config).await?;
            }
              app_state.connected = true;
            
            color_display.display_success("已连接到RustChat服务器");
            color_display.display_info(&format!("您的用户ID: {}", app_state.user_id.as_ref().unwrap()));
            if let Some(nick) = &app_state.nickname {
                color_display.display_success(&format!("当前昵称: {}", nick));
            } else {
                color_display.display_info("使用 /nick <昵称> 来设置您的昵称");
            }
            color_display.display_info("输入消息开始聊天，输入 /help 查看命令帮助");
            color_display.display_separator();
        }
        WsEvent::Message(msg) => {
            let mut app_state = state.lock().await;
            app_state.messages.push(msg.clone());
            drop(app_state);
            
            // 保存消息到数据库
            if let Err(err) = message_db.save_message(&msg).await {
                error!("保存消息到数据库失败: {}", err);
            }
            
            display_message(&msg, color_display);
        }        WsEvent::UserJoined { user_id: _, nickname } => {
            let nick = nickname.unwrap_or_else(|| "匿名用户".to_string());
            color_display.display_success(&format!("{} 加入了聊天室", nick));
        }
        WsEvent::UserLeft { user_id: _ } => {
            color_display.display_info("用户离开了聊天室");
        }
        WsEvent::Error { message } => {
            error!("服务器错误: {}", message);
            color_display.display_error(&format!("错误: {}", message));
        }
        WsEvent::Ping => {
            // 收到服务器心跳，立即回复Pong
            info!("收到服务器心跳，回复Pong");
            let pong_msg = ClientMessage::Pong;
            if let Ok(json) = serde_json::to_string(&pong_msg) {
                if let Err(err) = ws_sender.send(WsMessage::Text(json.into())) {
                    error!("发送心跳响应失败: {}", err);
                }
            }
        }
        WsEvent::Pong => {
            // 收到心跳响应（如果客户端主动发送心跳的话）
            info!("收到服务器心跳响应");
        }
    }
    
    Ok(())
}

/// 命令类型枚举
#[derive(Debug, Clone)]
pub enum Command {
    Help,
    Nick(String),
    Whoami,
    History(Option<i64>),
    Clear,
    Quit,
    Unknown(String),
}

/// 命令解析结果
#[derive(Debug)]
pub struct ParsedCommand {
    pub command: Command,
    pub raw_input: String,
}

/// 命令解析器
pub struct CommandParser;

impl CommandParser {
    /// 解析命令字符串
    pub fn parse_command(input: &str) -> ParsedCommand {
        let raw_input = input.to_string();
        
        if !input.starts_with('/') {
            return ParsedCommand {
                command: Command::Unknown("不是有效的命令".to_string()),
                raw_input,
            };
        }
        
        let parts: Vec<&str> = input[1..].split_whitespace().collect();
        if parts.is_empty() {
            return ParsedCommand {
                command: Command::Unknown("空命令".to_string()),
                raw_input,
            };
        }
        
        let command = match parts[0].to_lowercase().as_str() {
            "help" | "h" => Command::Help,
            "nick" | "nickname" => {
                if parts.len() < 2 {
                    Command::Unknown("昵称不能为空".to_string())
                } else {
                    let nickname = parts[1..].join(" ");
                    Command::Nick(nickname)
                }
            }
            "whoami" | "who" => Command::Whoami,
            "history" | "hist" => {
                let limit = if parts.len() > 1 {
                    parts[1].parse::<i64>().ok()
                } else {
                    None
                };
                Command::History(limit)
            }
            "clear" | "cls" => Command::Clear,
            "quit" | "exit" | "q" => Command::Quit,
            _ => Command::Unknown(format!("未知命令: {}", parts[0])),
        };
        
        ParsedCommand { command, raw_input }
    }
}

/// 命令执行器
pub struct CommandExecutor;

impl CommandExecutor {    /// 执行命令
    pub async fn execute_command(
        parsed_cmd: ParsedCommand,
        state: Arc<Mutex<AppState>>,
        config_manager: &UserConfigManager,
        message_db: Arc<MessageDatabase>,
        ws_sender: &tokio::sync::mpsc::UnboundedSender<WsMessage>,
        color_display: &ColorDisplay,
    ) -> Result<bool> {
        match parsed_cmd.command {
            Command::Help => {
                Self::execute_help_command(color_display).await;
                Ok(true)
            }
            Command::Nick(nickname) => {
                Self::execute_nick_command(nickname, state, config_manager, ws_sender, color_display).await
            }
            Command::Whoami => {
                Self::execute_whoami_command(state, color_display).await;
                Ok(true)
            }
            Command::History(limit) => {
                Self::execute_history_command(limit, message_db, color_display).await;
                Ok(true)
            }
            Command::Clear => {
                Self::execute_clear_command(color_display).await;
                Ok(true)
            }
            Command::Quit => {
                Self::execute_quit_command(color_display).await;
                Ok(false) // 返回 false 表示应该退出
            }
            Command::Unknown(error) => {
                color_display.display_error(&error);
                color_display.display_info("输入 /help 查看可用命令");
                Ok(true)
            }
        }
    }    /// 执行帮助命令
    async fn execute_help_command(color_display: &ColorDisplay) {
        use crossterm::style::{Color, SetForegroundColor, ResetColor};
        use std::io::{self, Write};
        
        let mut stdout = io::stdout();
        
        // 标题
        stdout.execute(SetForegroundColor(Color::Cyan)).unwrap();
        println!("📚 RustChat 命令帮助");
        
        // 表格边框颜色
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("┌─────────────────────────────────────────────────────────┐");
        
        stdout.execute(SetForegroundColor(Color::Yellow)).unwrap();
        println!("│                      基础命令                           │");
        
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("├─────────────────────────────────────────────────────────┤");
        
        stdout.execute(SetForegroundColor(Color::Green)).unwrap();
        println!("│ /help, /h           - 显示此帮助信息                    │");
        println!("│ /quit, /exit, /q    - 退出程序                         │");
        println!("│ /clear, /cls        - 清空屏幕                          │");
        
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("├─────────────────────────────────────────────────────────┤");
        
        stdout.execute(SetForegroundColor(Color::Yellow)).unwrap();
        println!("│                      用户命令                           │");
        
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("├─────────────────────────────────────────────────────────┤");
        
        stdout.execute(SetForegroundColor(Color::Green)).unwrap();
        println!("│ /nick <昵称>        - 设置用户昵称                      │");
        println!("│ /whoami, /who       - 显示当前用户信息                  │");
        
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("├─────────────────────────────────────────────────────────┤");
        
        stdout.execute(SetForegroundColor(Color::Yellow)).unwrap();
        println!("│                      消息命令                           │");
        
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("├─────────────────────────────────────────────────────────┤");
        
        stdout.execute(SetForegroundColor(Color::Green)).unwrap();
        println!("│ /history [数量]     - 显示消息历史 (默认20条)           │");
        println!("│ /hist [数量]        - history的简写                    │");
        
        stdout.execute(SetForegroundColor(Color::DarkGrey)).unwrap();
        println!("└─────────────────────────────────────────────────────────┘");
        
        stdout.execute(ResetColor).unwrap();
        println!();
        
        color_display.display_info("使用技巧:");
        color_display.display_success("   • 直接输入消息内容即可发送（无需命令前缀）");
        color_display.display_success("   • 命令支持简写，如 /h 代替 /help");
        color_display.display_success("   • 昵称可以包含空格，如: /nick 张三 李四");
        
        println!();
        color_display.display_info("📝 使用示例:");
        color_display.display_success("   /nick 小明          - 设置昵称为「小明」");
        color_display.display_success("   /history 50         - 查看最近50条消息");
        color_display.display_success("   /h                  - 显示帮助（简写）");
        color_display.display_success("   /clear              - 清空屏幕");
        color_display.display_success("   你好大家!           - 发送普通消息");
        
        println!();
        color_display.display_info("🔄 连接状态:");
        color_display.display_success("   • 支持自动重连，断线后会自动尝试重新连接");
        color_display.display_success("   • 最多重试10次，使用指数退避策略");
        color_display.display_success("   • 心跳保活机制确保连接稳定性");
        
        stdout.flush().unwrap();
    }
      /// 执行昵称设置命令
    async fn execute_nick_command(
        nickname: String,
        state: Arc<Mutex<AppState>>,
        config_manager: &UserConfigManager,
        ws_sender: &tokio::sync::mpsc::UnboundedSender<WsMessage>,
        color_display: &ColorDisplay,
    ) -> Result<bool> {        // 验证昵称格式
        if nickname.trim().is_empty() {
            color_display.display_error("昵称不能为空");
            return Ok(true);
        }
        
        if nickname.len() > 32 {
            color_display.display_error("昵称长度不能超过32个字符");
            return Ok(true);
        }
        
        // 检查是否包含非法字符
        if nickname.contains(['\n', '\r', '\t']) {
            color_display.display_error("昵称不能包含换行符或制表符");
            return Ok(true);
        }
        
        let nickname = nickname.trim().to_string();
        
        // 发送昵称设置消息到服务器
        let msg = ClientMessage::SetNickname { nickname: nickname.clone() };
        let json = serde_json::to_string(&msg)?;
        ws_sender.send(WsMessage::Text(json.into()))?;
          // 更新本地配置
        if let Err(err) = config_manager.update_nickname(nickname.clone()).await {
            error!("更新本地配置失败: {}", err);
            color_display.display_error("昵称已发送到服务器，但本地配置更新失败");
        }
          let mut app_state = state.lock().await;
        app_state.nickname = Some(nickname.clone());
        
        color_display.display_success(&format!("昵称已设置为: {}", nickname));
        Ok(true)
    }
    
    /// 执行用户信息查询命令
    async fn execute_whoami_command(state: Arc<Mutex<AppState>>, color_display: &ColorDisplay) {
        let app_state = state.lock().await;
        color_display.display_info("👤 用户信息:");
        
        let user_id = app_state.user_id.as_ref().map(|id| id.to_string()).unwrap_or_else(|| "未知".to_string());
        color_display.display_success(&format!("  🆔 用户ID: {}", user_id));
        
        if let Some(nickname) = &app_state.nickname {
            color_display.display_success(&format!("  📝 昵称: {}", nickname));
        } else {
            color_display.display_info("  📝 昵称: 未设置 (使用 /nick <昵称> 设置)");
        }
        
        let connection_status = if app_state.connected { "已连接" } else { "未连接" };
        color_display.display_success(&format!("  🔗 连接状态: {}", connection_status));
    }
    
    /// 执行历史消息查询命令
    async fn execute_history_command(limit: Option<i64>, message_db: Arc<MessageDatabase>, color_display: &ColorDisplay) {
        let limit = limit.unwrap_or(20);
        
        if limit <= 0 {
            color_display.display_error("消息数量必须大于0");
            return;
        }
        
        if limit > 1000 {
            color_display.display_error("一次最多只能查看1000条消息");
            return;
        }
          match message_db.get_recent_messages(limit).await {
            Ok(messages) => {
                if messages.is_empty() {
                    color_display.display_info("暂无消息历史");
                } else {
                    color_display.display_history_separator(messages.len());
                    for msg in &messages {
                        display_message(msg, color_display);
                    }
                    color_display.display_separator();
                }
            }
            Err(err) => {
                error!("获取消息历史失败: {}", err);
                color_display.display_error(&format!("获取消息历史失败: {}", err));
            }
        }
    }
      /// 执行清屏命令
    async fn execute_clear_command(color_display: &ColorDisplay) {
        color_display.clear_screen();
        color_display.display_welcome();
        color_display.display_info("输入消息开始聊天，输入 /help 查看命令帮助");
        color_display.display_separator();
    }
    
    /// 执行退出命令
    async fn execute_quit_command(color_display: &ColorDisplay) {
        color_display.display_info("正在关闭连接...");
        color_display.display_info("💾 保存配置和消息历史...");
        color_display.display_info("🔒 清理资源...");
        color_display.display_success("👋 再见！感谢使用 RustChat！");
    }
}

/// 处理命令
async fn handle_command_via_channel(
    input: &str,
    state: Arc<Mutex<AppState>>,
    config_manager: &UserConfigManager,
    message_db: Arc<MessageDatabase>,
    ws_sender: &tokio::sync::mpsc::UnboundedSender<WsMessage>,
    color_display: &ColorDisplay,
) -> Result<bool> {
    let parsed_command = CommandParser::parse_command(input);
    CommandExecutor::execute_command(parsed_command, state, config_manager, message_db, ws_sender, color_display).await
}

/// 发送消息
async fn send_message_via_channel(
    content: String,
    state: Arc<Mutex<AppState>>,
    ws_sender: &tokio::sync::mpsc::UnboundedSender<WsMessage>,
) -> Result<()> {
    let app_state = state.lock().await;
    let nickname = app_state.nickname.clone();
    drop(app_state);
      let msg = ClientMessage::SendMessage { content, nickname };
    let json = serde_json::to_string(&msg)?;
    ws_sender.send(WsMessage::Text(json.into()))?;
    
    Ok(())
}

/// 连接配置
#[derive(Clone)]
pub struct ConnectionConfig {
    pub url: String,
    pub max_reconnect_attempts: u32,
    pub initial_retry_delay: Duration,
    pub max_retry_delay: Duration,
    pub retry_backoff_factor: f64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:8080/ws".to_string(),
            max_reconnect_attempts: 10,
            initial_retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(30),
            retry_backoff_factor: 2.0,
        }
    }
}

/// 连接到WebSocket服务器
async fn connect_to_server(
    url: &str,
) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
    let (ws_stream, _) = connect_async(url)
        .await
        .context("无法连接到WebSocket服务器")?;
    Ok(ws_stream)
}

/// 运行单次连接会话
async fn run_connection_session(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    state: Arc<Mutex<AppState>>,
    config_manager: UserConfigManager,
    message_db: Arc<MessageDatabase>,
    input_rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<bool> {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // 创建用于WebSocket任务中发送消息的通道
    let (ws_send_tx, mut ws_send_rx) = tokio::sync::mpsc::unbounded_channel::<WsMessage>();
    let ws_send_tx_clone = ws_send_tx.clone();
      // WebSocket发送任务
    let mut ws_sender_task = tokio::spawn(async move {
        while let Some(message) = ws_send_rx.recv().await {
            if let Err(err) = ws_sender.send(message).await {
                error!("WebSocket发送失败: {}", err);
                break;
            }
        }
    });
      // WebSocket接收任务
    let state_clone = state.clone();
    let config_manager_clone = config_manager.clone();
    let message_db_clone = message_db.clone();
    let mut ws_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {                    if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
                        // 获取color_display引用
                        let color_display = {
                            let app_state = state_clone.lock().await;
                            app_state.color_display.clone()
                        };
                        
                        if let Err(err) = handle_ws_event_with_sender(
                            event, 
                            state_clone.clone(), 
                            &config_manager_clone,
                            message_db_clone.clone(),
                            &ws_send_tx_clone,
                            &color_display
                        ).await {
                            error!("处理WebSocket事件失败: {}", err);
                        }
                    }
                }                Ok(WsMessage::Close(_)) => {
                    let app_state = state_clone.lock().await;
                    app_state.color_display.display_info("🔌 服务器连接已关闭");
                    break;
                }
                Err(err) => {
                    error!("WebSocket错误: {}", err);
                    break;
                }
                _ => {}
            }
        }
    });
    
    // 处理用户输入
    let mut should_quit = false;
    
    loop {
        tokio::select! {
            // 处理用户输入
            input = input_rx.recv() => {
                match input {
                    Some(input) => {
                        if input.is_empty() {
                            continue;
                        }
                          if input.starts_with('/') {
                            let color_display = {
                                let app_state = state.lock().await;
                                app_state.color_display.clone()
                            };
                            
                            let should_continue = handle_command_via_channel(
                                &input,
                                state.clone(),
                                &config_manager,
                                message_db.clone(),
                                &ws_send_tx,
                                &color_display,
                            ).await?;
                            
                            if !should_continue {
                                should_quit = true;
                                break;
                            }
                        } else {
                            if let Err(err) = send_message_via_channel(input, state.clone(), &ws_send_tx).await {
                                error!("发送消息失败: {}", err);
                                break;
                            }
                        }
                    }
                    None => {
                        // 输入通道已关闭
                        should_quit = true;
                        break;
                    }
                }
            }
              // 检查WebSocket任务是否结束
            _ = &mut ws_task => {
                // WebSocket连接断开
                {
                    let app_state = state.lock().await;
                    app_state.color_display.display_error("WebSocket连接断开");
                    let mut app_state = state.lock().await;
                    app_state.connected = false;
                }
                break;
            }
            
            _ = &mut ws_sender_task => {
                // WebSocket发送任务结束
                error!("WebSocket发送任务意外结束");
                break;
            }
        }
    }
    
    // 清理任务
    drop(ws_send_tx);
    ws_task.abort();
    ws_sender_task.abort();
    
    Ok(should_quit)
}

/// 带重连的客户端运行函数
async fn run_client_with_reconnect() -> Result<()> {
    let config = ConnectionConfig::default();
    let mut reconnect_attempts = 0;
    let mut current_retry_delay = config.initial_retry_delay;
    
    // 初始化配置管理器
    let config_manager = UserConfigManager::new()?;
      // 初始化消息数据库
    let message_db = Arc::new(MessageDatabase::new().await
        .context("Failed to initialize message database")?);
    
    // 创建临时ColorDisplay用于启动信息
    let temp_color_display = ColorDisplay::new();
    
    // 加载历史消息
    temp_color_display.display_info("正在加载消息历史...");
    let history_messages = message_db.get_recent_messages(100).await
        .context("Failed to load message history")?;
        
    // 加载或创建用户配置
    let user_config = config_manager.load_config().await?;
    info!("用户ID已加载: {}", user_config.user_id);
        
    let state = Arc::new(Mutex::new(AppState::new()));
    
    // 初始化应用状态，使用已加载的用户配置
    {
        let mut app_state = state.lock().await;
        app_state.user_id = Some(user_config.user_id.clone());
        app_state.nickname = user_config.nickname.clone();
        app_state.messages.extend(history_messages.clone());
    }
    
    // 显示欢迎消息（使用彩色显示）
    {
        let app_state = state.lock().await;
        app_state.color_display.display_welcome();
        
        if let Some(nickname) = &user_config.nickname {
            app_state.color_display.display_success(&format!("欢迎回来，{}!", nickname));
        } else {
            app_state.color_display.display_info("首次使用RustChat！");
            app_state.color_display.display_info("使用 /nick <昵称> 来设置您的昵称");
        }
    }
    
    // 显示历史消息（使用彩色显示）
    if !history_messages.is_empty() {
        let app_state = state.lock().await;
        app_state.color_display.display_history_separator(history_messages.len());
        for msg in &history_messages {
            display_message(msg, &app_state.color_display);
        }
        app_state.color_display.display_separator();
    }
      // 创建用户输入通道
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // 创建共享的ColorDisplay实例用于输入提示
    let color_display_for_input = ColorDisplay::new();
    
    // 启动用户输入处理任务
    tokio::spawn(async move {
        let mut input = String::new();
        loop {
            color_display_for_input.display_prompt();
            
            input.clear();
            if let Err(_) = io::stdin().read_line(&mut input) {
                break;
            }
            
            let input_trimmed = input.trim().to_string();
            if let Err(_) = input_tx.send(input_trimmed) {
                break;
            }
        }
    });
    
    loop {
        // 尝试连接
        info!("正在连接到服务器: {}", config.url);
          match connect_to_server(&config.url).await {
            Ok(ws_stream) => {
                temp_color_display.display_success("已连接到RustChat服务器");
                reconnect_attempts = 0;
                current_retry_delay = config.initial_retry_delay;
                
                // 运行连接会话
                match run_connection_session(
                    ws_stream,
                    state.clone(),
                    config_manager.clone(),
                    message_db.clone(),
                    &mut input_rx,
                ).await {                    Ok(should_quit) => {
                        if should_quit {
                            temp_color_display.display_success("👋 再见!");
                            break;
                        }
                        // 连接断开，但用户没有主动退出，需要重连
                    }
                    Err(err) => {
                        error!("连接会话错误: {}", err);
                    }
                }
            }
            Err(err) => {                error!("连接失败: {}", err);
                
                if reconnect_attempts == 0 {
                    temp_color_display.display_error(&format!("连接失败: {}", err));
                }
                
                reconnect_attempts += 1;
                
                if reconnect_attempts > config.max_reconnect_attempts {
                    temp_color_display.display_error(&format!("重连次数已达上限 ({})，停止重连", config.max_reconnect_attempts));
                    temp_color_display.display_info("请确保服务器正在运行: cargo run --bin rustchatd");
                    break;
                }
                
                temp_color_display.display_info(&format!("🔄 连接失败，{:.1}秒后重试 ({}/{})", 
                    current_retry_delay.as_secs_f64(), 
                    reconnect_attempts, 
                    config.max_reconnect_attempts
                ));
                
                time::sleep(current_retry_delay).await;
                
                // 指数退避
                current_retry_delay = Duration::from_millis(
                    ((current_retry_delay.as_millis() as f64) * config.retry_backoff_factor) as u64
                ).min(config.max_retry_delay);
                
                continue;
            }
        }
          // 如果到这里，说明连接断开了，需要重连
        temp_color_display.display_info("🔄 连接断开，正在尝试重连...");
        
        // 等待一小段时间再重连
        time::sleep(Duration::from_secs(2)).await;
    }
    
    Ok(())
}

/// 运行CLI客户端（现在使用带重连的版本）
async fn run_client() -> Result<()> {
    run_client_with_reconnect().await
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()        .init();
    
    // 创建临时ColorDisplay用于启动信息
    let temp_color_display = ColorDisplay::new();
    temp_color_display.display_welcome();
    temp_color_display.display_info("正在启动...");
    
    // 显示用户配置信息
    match UserConfigManager::new() {
        Ok(config_manager) => {
            match config_manager.load_config().await {
                Ok(config) => {
                    temp_color_display.display_info("📁 配置目录: ~/.rustchat/");
                    temp_color_display.display_success(&format!("🆔 用户ID: {}", config.user_id));
                    if let Some(nickname) = &config.nickname {
                        temp_color_display.display_success(&format!("👤 昵称: {}", nickname));
                    }
                    println!();
                }
                Err(err) => {
                    error!("加载配置失败: {}", err);
                }
            }
        }
        Err(err) => {
            error!("初始化配置管理器失败: {}", err);
        }
    }
      if let Err(err) = run_client().await {
        error!("客户端运行失败: {}", err);
        temp_color_display.display_error(&format!("连接失败: {}", err));
        temp_color_display.display_info("请确保服务器正在运行: cargo run --bin rustchatd");
        std::process::exit(1);
    }
    
    Ok(())
}
