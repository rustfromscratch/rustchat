use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rustchat_core::{generate_user_id, MessageDatabase};
use rustchat_types::{Message, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

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
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let (tx, _rx) = broadcast::channel(1000);
        let message_db = MessageDatabase::new().await?;
        
        Ok(Self {
            tx,
            clients: Arc::new(Mutex::new(HashMap::new())),
            message_db: Arc::new(message_db),
        })
    }/// 广播事件给所有客户端
    pub fn broadcast(&self, event: WsEvent) {
        // 只有在有订阅者时才发送消息
        if self.tx.receiver_count() > 0 {
            if let Err(err) = self.tx.send(event) {
                warn!("广播消息失败: {}", err);
            }
        }
    }

    /// 添加客户端连接
    pub fn add_client(&self, client: ConnectedClient) {
        let user_id = client.user_id.clone();
        let nickname = client.nickname.clone();
        
        self.clients.lock().unwrap().insert(user_id.clone(), client);
        
        // 广播用户加入事件
        self.broadcast(WsEvent::UserJoined { user_id, nickname });
        
        info!("客户端已连接，总连接数: {}", self.clients.lock().unwrap().len());
    }

    /// 移除客户端连接
    pub fn remove_client(&self, user_id: &UserId) {
        self.clients.lock().unwrap().remove(user_id);
        
        // 广播用户离开事件
        self.broadcast(WsEvent::UserLeft { user_id: user_id.clone() });
        
        info!("客户端已断开，总连接数: {}", self.clients.lock().unwrap().len());
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
    }

    // 创建客户端信息（但先不添加到列表中）
    let client = ConnectedClient {
        user_id: user_id.clone(),
        nickname: None,
        sender: tx.clone(),
    };    // 订阅广播频道
    let broadcast_rx = state.tx.subscribe();

    // 启动广播消息处理任务
    let broadcast_task = tokio::spawn(broadcast_message_task(broadcast_rx, tx.clone()));

    // 现在添加到客户端列表（此时广播频道已有订阅者）
    state.add_client(client);

    // 启动消息发送任务
    let send_task = tokio::spawn(message_send_task(ws_sender, rx));

    // 启动消息接收循环
    let receive_task = tokio::spawn(message_receive_loop(ws_receiver, user_id.clone(), state.clone()));

    // 等待任何一个任务完成
    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
        _ = broadcast_task => {},
    }

    // 清理客户端连接
    state.remove_client(&user_id);
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

    info!("收到来自用户 {} 的消息: {:?}", user_id, client_msg);

    // 消息分发逻辑
    match client_msg {        ClientMessage::SendMessage { content, nickname } => {
            // 处理文本消息
            let message = Message::new_text(user_id.clone(), content.clone(), nickname.clone());
            info!("广播文本消息: {} 来自用户 {}", content, user_id);
            
            // 保存消息到数据库
            if let Err(err) = state.message_db.save_message(&message).await {
                error!("保存消息到数据库失败: {}", err);
            }
            
            state.broadcast(WsEvent::Message(message));
        }        ClientMessage::SetNickname { nickname } => {
            // 处理昵称设置
            let nick_change_msg = {
                let mut clients = state.clients.lock().unwrap();
                if let Some(client) = clients.get_mut(user_id) {
                    let old_nick = client.nickname.clone().unwrap_or_else(|| "匿名用户".to_string());
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
        }
        ClientMessage::Pong => {
            // 处理心跳响应
            info!("收到用户 {} 的心跳响应", user_id);
            // TODO: 重置心跳计时器 (将在NET-002中实现)
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

/// 健康检查端点
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "RustChat Server is running")
}

/// 创建应用路由
async fn create_app() -> anyhow::Result<Router> {
    let state = AppState::new().await?;

    Ok(Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
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
