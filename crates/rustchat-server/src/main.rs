mod room;

use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rustchat_core::{generate_user_id, MessageDatabase, BotManager, EchoBot};
use rustchat_types::{Message, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Mutex};
use tokio::time;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

// 导入房间相关模块
use room::{RoomManager, RoomBroadcastManager, RoomMessageRouter, create_room_routes};

/// WebSocket事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum WsEvent {
    /// 连接建立，服务器返回用户ID
    Connected { user_id: UserId },
    /// 新消息
    Message(Message),
    /// 用户加入
    UserJoined { user_id: UserId, nickname: Option<String> },
    /// 用户离开
    UserLeft { user_id: UserId },
    /// 心跳ping
    Ping,
    /// 心跳pong
    Pong,
    /// 错误消息
    Error { message: String },
}

/// 客户端消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    /// 发送文本消息
    SendMessage { content: String, nickname: Option<String> },
    /// 设置昵称
    SetNickname { nickname: String },
    /// 心跳响应
    Pong,
}

/// 连接的客户端信息
#[derive(Debug, Clone)]
pub struct ConnectedClient {
    pub user_id: UserId,
    pub nickname: Option<String>,
    pub sender: tokio::sync::mpsc::UnboundedSender<WsEvent>,
    pub last_pong: Arc<Mutex<Instant>>,
    pub connected_at: Instant,
}

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// 广播通道发送端
    pub tx: broadcast::Sender<WsEvent>,
    /// 连接的客户端
    pub clients: Arc<Mutex<HashMap<UserId, ConnectedClient>>>,
    /// 消息数据库
    pub message_db: Arc<MessageDatabase>,
    /// 机器人管理器
    pub bot_manager: Arc<Mutex<BotManager>>,
    /// 消息广播发送端（用于机器人发送消息）
    pub message_tx: broadcast::Sender<Message>,
    /// 房间管理器
    pub room_manager: Arc<RoomManager>,
    /// 房间广播管理器
    pub room_broadcast_manager: RoomBroadcastManager,
    /// 房间消息路由器
    pub room_message_router: Arc<RoomMessageRouter>,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let (tx, _rx) = broadcast::channel(1000);
        let (message_tx, _message_rx) = broadcast::channel(1000);
        let message_db = MessageDatabase::new().await?;
        
        // 创建并初始化机器人管理器
        let mut bot_manager = BotManager::new(message_tx.clone());
        
        // 注册Echo机器人
        let echo_bot = EchoBot::new();
        bot_manager.register_bot(Box::new(echo_bot));
        
        // 初始化所有机器人
        bot_manager.initialize_all().await?;
        
        // 创建房间相关组件
        let room_manager = Arc::new(RoomManager::new());
        let room_broadcast_manager = RoomBroadcastManager::new();
        let room_message_router = Arc::new(RoomMessageRouter::new(room_broadcast_manager.clone()));
        
        Ok(Self {
            tx,
            clients: Arc::new(Mutex::new(HashMap::new())),
            message_db: Arc::new(message_db),
            bot_manager: Arc::new(Mutex::new(bot_manager)),
            message_tx,
            room_manager,
            room_broadcast_manager,
            room_message_router,
        })
    }/// 广播事件给所有客户端
    pub fn broadcast(&self, event: WsEvent) {
        // 只有在有订阅者时才发送消息
        if self.tx.receiver_count() > 0 {
            if let Err(err) = self.tx.send(event) {
                warn!("广播消息失败: {}", err);
            }
        }
    }    /// 添加客户端连接
    pub async fn add_client(&self, client: ConnectedClient) {
        let user_id = client.user_id.clone();
        let nickname = client.nickname.clone();
        
        self.clients.lock().await.insert(user_id.clone(), client);
        
        // 广播用户加入事件
        self.broadcast(WsEvent::UserJoined { user_id, nickname });
        
        info!("客户端已连接，总连接数: {}", self.clients.lock().await.len());
    }

    /// 移除客户端连接
    pub async fn remove_client(&self, user_id: &UserId) {
        self.clients.lock().await.remove(user_id);
        
        // 广播用户离开事件
        self.broadcast(WsEvent::UserLeft { user_id: user_id.clone() });
        
        info!("客户端已断开，总连接数: {}", self.clients.lock().await.len());
    }
}

/// WebSocket升级处理
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// 处理WebSocket连接
async fn handle_socket(socket: WebSocket, state: AppState) {
    // 为新连接生成用户ID
    let user_id = generate_user_id();
    
    info!("新的WebSocket连接，用户ID: {}", user_id);    let (mut ws_sender, ws_receiver) = socket.split();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<WsEvent>();

    // 发送连接建立事件
    let connected_event = WsEvent::Connected { user_id: user_id.clone() };
    if let Ok(msg) = serde_json::to_string(&connected_event) {
        if ws_sender.send(WsMessage::Text(msg.into())).await.is_err() {
            error!("发送连接建立消息失败");
            return;
        }
    }    // 创建客户端信息（但先不添加到列表中）
    let now = Instant::now();
    let client = ConnectedClient {
        user_id: user_id.clone(),
        nickname: None,
        sender: tx.clone(),
        last_pong: Arc::new(Mutex::new(now)),
        connected_at: now,
    };// 订阅广播频道
    let broadcast_rx = state.tx.subscribe();

    // 启动广播消息处理任务
    let broadcast_task = tokio::spawn(broadcast_message_task(broadcast_rx, tx.clone()));

    // 现在添加到客户端列表（此时广播频道已有订阅者）
    state.add_client(client).await;    // 启动消息发送任务
    let send_task = tokio::spawn(message_send_task(ws_sender, rx));

    // 启动心跳任务
    let heartbeat_task = tokio::spawn(heartbeat_task(user_id.clone(), state.clone()));

    // 启动消息接收循环
    let receive_task = tokio::spawn(message_receive_loop(ws_receiver, user_id.clone(), state.clone()));

    // 等待任何一个任务完成
    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
        _ = broadcast_task => {},
        _ = heartbeat_task => {},
    }    // 清理客户端连接
    state.remove_client(&user_id).await;
}

/// 处理客户端消息
async fn handle_client_message(
    text: &str,
    user_id: &UserId,
    state: &AppState,
) -> anyhow::Result<()> {
    // 消息解析
    let client_msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| anyhow::anyhow!("解析客户端消息失败: {}", e))?;

    info!("收到来自用户 {} 的消息: {:?}", user_id, client_msg);    // 消息分发逻辑
    match client_msg {        ClientMessage::SendMessage { content, nickname } => {
            // 处理文本消息
            let message = Message::new_text(user_id.clone(), content.clone(), nickname.clone());
            info!("广播文本消息: {} 来自用户 {}", content, user_id);
            debug!("创建的消息ID: {}", message.id);
            
            // 保存消息到数据库
            if let Err(err) = state.message_db.save_message(&message).await {
                error!("保存消息到数据库失败: {}", err);
            } else {
                debug!("消息已保存到服务器数据库");
            }
            
            // 广播消息给所有客户端
            debug!("广播消息给所有客户端: ID={}", message.id);
            state.broadcast(WsEvent::Message(message.clone()));// 让机器人处理消息
            {
                let bot_manager = state.bot_manager.lock().await;
                if let Err(err) = bot_manager.handle_message(&message).await {
                    error!("机器人处理消息失败: {}", err);
                }
            }
        }
        ClientMessage::SetNickname { nickname } => {
            // 验证昵称
            let nickname = nickname.trim().to_string();
            if nickname.is_empty() {
                return Err(anyhow::anyhow!("昵称不能为空"));
            }
            
            if nickname.len() > 32 {
                return Err(anyhow::anyhow!("昵称长度不能超过32个字符"));
            }
            
            if nickname.contains(['\n', '\r', '\t']) {
                return Err(anyhow::anyhow!("昵称不能包含非法字符"));
            }
              // 处理昵称设置
            let nick_change_msg = {
                let mut clients = state.clients.lock().await;
                if let Some(client) = clients.get_mut(user_id) {
                    let old_nick = client.nickname.clone().unwrap_or_else(|| "匿名用户".to_string());
                    
                    // 如果昵称没有变化，不需要广播
                    if client.nickname.as_ref() == Some(&nickname) {
                        info!("用户 {} 昵称无变化: {}", user_id, nickname);
                        return Ok(());
                    }
                    
                    client.nickname = Some(nickname.clone());
                    
                    info!("用户 {} 昵称变更: {} -> {}", user_id, old_nick, nickname);
                    
                    // 创建昵称变更消息
                    Some(Message::new_nick_change(
                        user_id.clone(),
                        old_nick,
                        nickname.clone(),
                        Some(nickname),
                    ))
                } else {
                    None
                }
            }; // 这里释放锁
            
            if let Some(nick_change_msg) = nick_change_msg {
                // 保存昵称变更消息到数据库
                if let Err(err) = state.message_db.save_message(&nick_change_msg).await {
                    error!("保存昵称变更消息到数据库失败: {}", err);
                }
                
                state.broadcast(WsEvent::Message(nick_change_msg));
            } else {
                return Err(anyhow::anyhow!("用户 {} 不在连接列表中", user_id));
            }
        }        ClientMessage::Pong => {
            // 处理心跳响应
            info!("收到用户 {} 的心跳响应", user_id);            // 更新最后心跳时间
            {
                let clients = state.clients.lock().await;
                if let Some(client) = clients.get(user_id) {
                    *client.last_pong.lock().await = Instant::now();
                }
            }
        }
    }

    Ok(())
}

/// 异步消息接收循环
async fn message_receive_loop(
    mut ws_receiver: futures_util::stream::SplitStream<WebSocket>,
    user_id: UserId,
    state: AppState,
) {
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(WsMessage::Text(text)) => {
                if let Err(err) = handle_client_message(&text, &user_id, &state).await {
                    error!("处理客户端消息失败: {}", err);
                }
            }
            Ok(WsMessage::Close(_)) => {
                info!("客户端主动关闭连接: {}", user_id);
                break;
            }
            Ok(WsMessage::Ping(_data)) => {
                // 处理Ping消息
                info!("收到Ping消息从用户: {}", user_id);
                // TODO: 实现Pong响应 (将在NET-002中实现)
            }
            Ok(WsMessage::Pong(_)) => {
                // 处理Pong消息
                info!("收到Pong响应从用户: {}", user_id);
            }
            Err(err) => {
                error!("WebSocket错误: {}", err);
                break;
            }
            _ => {
                // 忽略其他消息类型
            }
        }
    }
}

/// 异步消息发送任务
async fn message_send_task(
    mut ws_sender: futures_util::stream::SplitSink<WebSocket, WsMessage>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<WsEvent>,
) {
    while let Some(event) = rx.recv().await {
        if let Ok(msg) = serde_json::to_string(&event) {
            if ws_sender.send(WsMessage::Text(msg.into())).await.is_err() {
                error!("发送消息失败，连接可能已断开");
                break;
            }
        } else {
            error!("序列化消息失败");
        }
    }
}

/// 广播消息处理任务
async fn broadcast_message_task(
    mut broadcast_rx: broadcast::Receiver<WsEvent>,
    tx: tokio::sync::mpsc::UnboundedSender<WsEvent>,
) {
    while let Ok(event) = broadcast_rx.recv().await {
        if tx.send(event).is_err() {
            // 客户端通道已关闭，退出任务
            break;
        }
    }
}

/// 心跳任务
async fn heartbeat_task(user_id: UserId, state: AppState) {
    let mut interval = time::interval(Duration::from_secs(30)); // 30秒心跳间隔
    let timeout_duration = Duration::from_secs(90); // 90秒超时
    
    loop {
        interval.tick().await;
          // 检查客户端是否仍然连接
        let client_exists = {
            let clients = state.clients.lock().await;
            clients.contains_key(&user_id)
        };
        
        if !client_exists {
            info!("用户 {} 已断开连接，停止心跳任务", user_id);
            break;
        }
        
        // 检查是否超时
        let should_disconnect = {
            let clients = state.clients.lock().await;
            if let Some(client) = clients.get(&user_id) {
                let last_pong = *client.last_pong.lock().await;
                let elapsed = last_pong.elapsed();
                
                if elapsed > timeout_duration {
                    warn!("用户 {} 心跳超时 ({}s)，将断开连接", user_id, elapsed.as_secs());
                    true
                } else {
                    false
                }
            } else {
                true // 客户端不存在，退出
            }
        };
          if should_disconnect {
            // 移除超时的客户端
            state.remove_client(&user_id).await;
            break;
        }
          // 发送心跳Ping
        {
            let clients = state.clients.lock().await;
            if let Some(client) = clients.get(&user_id) {
                if let Err(err) = client.sender.send(WsEvent::Ping) {
                    warn!("发送心跳到用户 {} 失败: {}", user_id, err);
                    break;
                }
            }
        }
        
        info!("发送心跳到用户 {}", user_id);
    }
}

/// 健康检查端点
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "RustChat Server is running")
}

/// 创建应用路由
async fn create_app() -> anyhow::Result<Router> {
    let state = AppState::new().await?;

    // 启动机器人消息监听任务
    start_bot_message_listener(state.clone()).await;

    Ok(Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
        .merge(create_room_routes()) // 添加房间API路由
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let app = create_app().await?;
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
        
    info!("RustChat服务器启动在 http://127.0.0.1:8080");
    info!("WebSocket端点: ws://127.0.0.1:8080/ws");
    info!("健康检查: http://127.0.0.1:8080/health");
    info!("消息历史功能已启用 (SQLite数据库)");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

/// 启动机器人消息监听任务
async fn start_bot_message_listener(state: AppState) {
    let mut message_rx = state.message_tx.subscribe();
    
    tokio::spawn(async move {
        info!("机器人消息监听器已启动");
        
        while let Ok(bot_message) = message_rx.recv().await {
            info!("收到机器人消息: {:?}", bot_message);
            
            // 保存机器人消息到数据库
            if let Err(err) = state.message_db.save_message(&bot_message).await {
                error!("保存机器人消息到数据库失败: {}", err);
            }
            
            // 广播机器人消息给所有客户端
            state.broadcast(WsEvent::Message(bot_message));
        }
        
        warn!("机器人消息监听器已停止");
    });
}
