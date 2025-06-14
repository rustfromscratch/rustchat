use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rustchat_core::UserConfigManager;
use rustchat_types::{Message, MessageType, UserId};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            user_id: None,
            nickname: None,
            messages: Vec::new(),
            connected: false,
        }
    }
}

/// 格式化消息显示
fn format_message(msg: &Message) -> String {
    let time = msg.timestamp.format("%H:%M:%S");
    let sender = msg.from_nick.as_deref().unwrap_or("匿名用户");
    
    match &msg.content {
        MessageType::Text(text) => {
            format!("[{}] {}: {}", time, sender, text)
        }
        MessageType::System(text) => {
            format!("[{}] [系统]: {}", time, text)
        }
        MessageType::NickChange { old_nick, new_nick } => {
            format!("[{}] [系统]: {} 将昵称改为 {}", time, old_nick, new_nick)
        }
    }
}

/// 显示消息
fn display_message(msg: &Message) {
    println!("{}", format_message(msg));
}

/// 处理WebSocket事件
async fn handle_ws_event(
    event: WsEvent,
    state: Arc<Mutex<AppState>>,
    config_manager: &UserConfigManager,
) -> Result<()> {
    match event {        WsEvent::Connected { user_id } => {
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
            
            println!("✅ 已连接到RustChat服务器");
            println!("🆔 您的用户ID: {}", app_state.user_id.as_ref().unwrap());
            if let Some(nick) = &app_state.nickname {
                println!("👤 当前昵称: {}", nick);
            } else {
                println!("💡 使用 /nick <昵称> 来设置您的昵称");
            }
            println!("💬 输入消息开始聊天，输入 /help 查看命令帮助");
            println!("---");
        }
        WsEvent::Message(msg) => {
            let mut app_state = state.lock().await;
            app_state.messages.push(msg.clone());
            drop(app_state);
            
            display_message(&msg);
        }        WsEvent::UserJoined { user_id: _, nickname } => {
            let nick = nickname.unwrap_or_else(|| "匿名用户".to_string());
            println!("👋 {} 加入了聊天室", nick);
        }
        WsEvent::UserLeft { user_id: _ } => {
            println!("👋 用户离开了聊天室");
        }
        WsEvent::Error { message } => {
            error!("服务器错误: {}", message);
            println!("❌ 错误: {}", message);
        }
        _ => {}
    }
    
    Ok(())
}

/// 处理命令
async fn handle_command(
    input: &str,
    state: Arc<Mutex<AppState>>,
    config_manager: &UserConfigManager,
    ws_sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
) -> Result<bool> {
    if !input.starts_with('/') {
        return Ok(false);
    }

    let parts: Vec<&str> = input[1..].split_whitespace().collect();
    if parts.is_empty() {
        return Ok(true);
    }

    match parts[0] {
        "help" => {
            println!("📚 可用命令:");
            println!("  /help        - 显示帮助信息");
            println!("  /nick <name> - 设置昵称");
            println!("  /quit        - 退出程序");
            println!("  其他输入     - 发送消息");
        }
        "nick" => {
            if parts.len() < 2 {
                println!("❌ 用法: /nick <昵称>");
                return Ok(true);
            }
            
            let nickname = parts[1..].join(" ");
              // 发送昵称设置消息到服务器
            let msg = ClientMessage::SetNickname { nickname: nickname.clone() };
            let json = serde_json::to_string(&msg)?;
            ws_sender.send(WsMessage::Text(json.into())).await?;
            
            // 更新本地配置
            config_manager.update_nickname(nickname.clone()).await?;
            
            let mut app_state = state.lock().await;
            app_state.nickname = Some(nickname.clone());
            
            println!("✅ 昵称已设置为: {}", nickname);
        }
        "quit" => {
            println!("👋 再见!");
            return Ok(false);
        }
        _ => {
            println!("❌ 未知命令: {}", parts[0]);
            println!("💡 输入 /help 查看可用命令");
        }
    }
    
    Ok(true)
}

/// 发送消息
async fn send_message(
    content: String,
    state: Arc<Mutex<AppState>>,
    ws_sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
) -> Result<()> {
    let app_state = state.lock().await;
    let nickname = app_state.nickname.clone();
    drop(app_state);
      let msg = ClientMessage::SendMessage { content, nickname };
    let json = serde_json::to_string(&msg)?;
    ws_sender.send(WsMessage::Text(json.into())).await?;
    
    Ok(())
}

/// 运行CLI客户端
async fn run_client() -> Result<()> {
    // 初始化配置管理器
    let config_manager = UserConfigManager::new()?;
    
    // 加载或创建用户配置
    let user_config = config_manager.load_config().await?;
    info!("用户ID已加载: {}", user_config.user_id);
    
    if let Some(nickname) = &user_config.nickname {
        info!("已加载昵称: {}", nickname);
        println!("👤 欢迎回来，{}!", nickname);
    } else {
        println!("👋 首次使用RustChat！");
        println!("💡 使用 /nick <昵称> 来设置您的昵称");
    }
    
    // 连接到WebSocket服务器
    let url = "ws://127.0.0.1:8080/ws";
    info!("正在连接到服务器: {}", url);
    
    let (ws_stream, _) = connect_async(url)
        .await
        .context("无法连接到WebSocket服务器")?;
      let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let state = Arc::new(Mutex::new(AppState::new()));
    
    // 初始化应用状态，使用已加载的用户配置
    {
        let mut app_state = state.lock().await;
        app_state.user_id = Some(user_config.user_id.clone());
        app_state.nickname = user_config.nickname.clone();
    }
    
    // 处理WebSocket消息的任务
    let state_clone = state.clone();
    let config_manager_clone = config_manager;
    let ws_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
                        if let Err(err) = handle_ws_event(event, state_clone.clone(), &config_manager_clone).await {
                            error!("处理WebSocket事件失败: {}", err);
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("🔌 服务器连接已关闭");
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
    let mut input = String::new();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        input.clear();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        if input.starts_with('/') {
            let should_continue = handle_command(
                input,
                state.clone(),
                &UserConfigManager::new()?,
                &mut ws_sender,
            ).await?;
            
            if !should_continue {
                break;
            }
        } else {
            send_message(input.to_string(), state.clone(), &mut ws_sender).await?;
        }
    }
    
    // 关闭WebSocket连接
    ws_sender.close().await?;
    ws_task.abort();
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    println!("🚀 RustChat CLI v0.1.0");
    println!("正在启动...");
    
    // 显示用户配置信息
    match UserConfigManager::new() {
        Ok(config_manager) => {
            match config_manager.load_config().await {
                Ok(config) => {
                    println!("📁 配置目录: ~/.rustchat/");
                    println!("🆔 用户ID: {}", config.user_id);
                    if let Some(nickname) = &config.nickname {
                        println!("👤 昵称: {}", nickname);
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
        println!("❌ 连接失败: {}", err);
        println!("💡 请确保服务器正在运行: cargo run --bin rustchatd");
        std::process::exit(1);
    }
    
    Ok(())
}
