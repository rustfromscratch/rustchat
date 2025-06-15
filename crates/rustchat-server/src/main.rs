mod auth;
mod room;
mod friend;

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
use room::{RoomManager, RoomBroadcastManager, RoomMessageRouter, create_protected_room_routes, create_public_room_routes};

// 导入认证相关模块
use auth::{AuthService, create_auth_routes};

// 导入好友相关模块
use friend::{FriendManager, create_friend_routes};

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
    /// 房间消息
    RoomMessage { room_id: String, message: Message },
    /// 用户加入房间
    UserJoinedRoom { room_id: String, user_id: UserId },
    /// 用户离开房间
    UserLeftRoom { room_id: String, user_id: UserId },
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
    /// 发送房间消息
    SendRoomMessage { room_id: String, content: String },
    /// 加入房间
    JoinRoom { room_id: String },
    /// 离开房间
    LeaveRoom { room_id: String },
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
    pub email: Option<String>,
    pub sender: tokio::sync::mpsc::UnboundedSender<WsEvent>,
    pub last_pong: Arc<Mutex<Instant>>,
    pub connected_at: Instant,
    /// 当前所在房间的广播接收器
    pub room_receiver: Arc<Mutex<Option<tokio::sync::broadcast::Receiver<WsEvent>>>>,
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
    /// 认证服务
    pub auth_service: AuthService,
    /// 好友管理器
    pub friend_manager: Arc<Mutex<FriendManager>>,
}

impl AppState {    pub async fn new() -> anyhow::Result<Self> {
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
          // 创建认证服务
        let auth_service = AuthService::new(message_db.get_pool().clone());
        
        // 初始化认证数据库表
        auth_service.initialize_database().await?;
        
        // 创建好友管理器
        let friend_manager = Arc::new(Mutex::new(FriendManager::new()));
        
        Ok(Self {
            tx,
            clients: Arc::new(Mutex::new(HashMap::new())),
            message_db: Arc::new(message_db),
            bot_manager: Arc::new(Mutex::new(bot_manager)),
            message_tx,
            room_manager,
            room_broadcast_manager,
            room_message_router,
            auth_service,
            friend_manager,
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
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    // 尝试从headers或query参数中提取认证信息
    let auth_user = extract_user_from_headers(&state, &headers).await;
    let auth_user = if auth_user.is_none() {
        extract_user_from_query(&state, &params).await
    } else {
        auth_user
    };
    
    ws.on_upgrade(move |socket| handle_socket(socket, state, auth_user))
}

/// 从query参数中提取认证用户信息
async fn extract_user_from_query(
    state: &AppState,
    params: &HashMap<String, String>
) -> Option<auth::AuthenticatedUser> {
    use auth::TokenType;
    
    // 从query参数中提取token
    let token = params.get("token")?;

    // 验证token并提取用户信息
    match state.auth_service.verify_token(token, TokenType::Access) {
        Ok(claims) => {
            // 从claims.sub解析AccountId
            let account_id = auth::AccountId::parse(&claims.sub).ok()?;
            
            // 从数据库获取完整的用户信息
            match state.auth_service.get_account_by_id(&account_id).await {
                Ok(account) => {
                    let user_id = UserId::parse(&account.id.to_string()).ok()?;
                    
                    Some(auth::AuthenticatedUser {
                        user_id,
                        account_id: account.id.to_string(),
                        email: account.email,
                    })
                }
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

/// 从请求头中提取认证用户信息
async fn extract_user_from_headers(
    state: &AppState,
    headers: &axum::http::HeaderMap
) -> Option<auth::AuthenticatedUser> {
    use axum::http::header::AUTHORIZATION;
    use auth::TokenType;
    
    // 从Authorization header中提取token
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())?;

    if !auth_header.starts_with("Bearer ") {
        return None;
    }

    let token = &auth_header[7..]; // 移除 "Bearer " 前缀

    // 验证token并提取用户信息
    match state.auth_service.verify_token(token, TokenType::Access) {
        Ok(claims) => {
            // 从claims.sub解析AccountId
            let account_id = auth::AccountId::parse(&claims.sub).ok()?;
            
            // 从数据库获取完整的用户信息
            match state.auth_service.get_account_by_id(&account_id).await {
                Ok(account) => {
                    let user_id = UserId::parse(&account.id.to_string()).ok()?;
                    
                    Some(auth::AuthenticatedUser {
                        user_id,
                        account_id: account.id.to_string(),
                        email: account.email,
                    })
                }
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

/// 处理WebSocket连接
async fn handle_socket(socket: WebSocket, state: AppState, auth_user: Option<auth::AuthenticatedUser>) {
    // 使用认证用户的ID或生成新的用户ID
    let (user_id, user_email) = if let Some(auth) = auth_user {
        (auth.user_id, Some(auth.email))
    } else {
        (generate_user_id(), None)
    };
    
    info!("新的WebSocket连接，用户ID: {}，邮箱: {:?}", user_id, user_email);let (mut ws_sender, ws_receiver) = socket.split();
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
        email: user_email,
        sender: tx.clone(),
        last_pong: Arc::new(Mutex::new(now)),
        connected_at: now,
        room_receiver: Arc::new(Mutex::new(None)),
    };// 订阅广播频道
    let broadcast_rx = state.tx.subscribe();    // 启动广播消息处理任务
    let broadcast_task = tokio::spawn(broadcast_message_task(broadcast_rx, tx.clone()));

    // 启动房间消息监听任务
    let room_message_task = tokio::spawn(room_message_task(user_id.clone(), state.clone(), tx.clone()));

    // 现在添加到客户端列表（此时广播频道已有订阅者）
    state.add_client(client).await;// 启动消息发送任务
    let send_task = tokio::spawn(message_send_task(ws_sender, rx));

    // 启动心跳任务
    let heartbeat_task = tokio::spawn(heartbeat_task(user_id.clone(), state.clone()));

    // 启动消息接收循环
    let receive_task = tokio::spawn(message_receive_loop(ws_receiver, user_id.clone(), state.clone()));    // 等待任何一个任务完成
    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
        _ = broadcast_task => {},
        _ = room_message_task => {},
        _ = heartbeat_task => {},
    }// 清理客户端连接
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
        ClientMessage::SendRoomMessage { room_id, content } => {
            // 处理房间消息
            let room_id_parsed = match room::RoomId::parse(&room_id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("无效的房间ID: {}", room_id)),
            };

            // 检查用户是否在房间中
            if !state.room_manager.is_user_in_room(room_id_parsed, user_id).await {
                return Err(anyhow::anyhow!("用户不在房间 {} 中", room_id));
            }

            // 创建房间消息
            let mut message = Message::new_text(user_id.clone(), content.clone(), None);
            message.additional_data = Some(serde_json::json!({
                "room_id": room_id
            }));

            info!("广播房间消息: {} 来自用户 {} 到房间 {}", content, user_id, room_id);

            // 保存消息到数据库
            if let Err(err) = state.message_db.save_message(&message).await {
                error!("保存房间消息到数据库失败: {}", err);
            }

            // 通过房间消息路由器广播
            if let Err(e) = state.room_message_router.route_message(message.clone(), user_id.clone()).await {
                error!("广播房间消息失败: {}", e);
            }
        }
        ClientMessage::JoinRoom { room_id } => {
            // 处理加入房间
            let room_id_parsed = match room::RoomId::parse(&room_id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("无效的房间ID: {}", room_id)),
            };

            // 尝试加入房间
            match state.room_manager.join_room(room_id_parsed, user_id.clone()).await {
                Ok(_) => {
                    // 在房间消息路由器中注册用户并获取接收器
                    if let Some(room_receiver) = state.room_message_router.handle_user_enter_room(user_id.clone(), room_id_parsed).await {
                        // 更新客户端的房间接收器
                        {
                            let clients = state.clients.lock().await;
                            if let Some(client) = clients.get(user_id) {
                                *client.room_receiver.lock().await = Some(room_receiver);
                            }
                        }

                        info!("用户 {} 通过WebSocket加入房间: {}", user_id, room_id);
                        
                        // 广播用户加入房间事件
                        state.broadcast(WsEvent::UserJoinedRoom { 
                            room_id: room_id.clone(), 
                            user_id: user_id.clone() 
                        });
                    }
                }
                Err(room::RoomError::UserAlreadyInRoom) => {
                    // 用户已经在房间中，仍然需要设置接收器
                    if let Some(room_receiver) = state.room_message_router.handle_user_enter_room(user_id.clone(), room_id_parsed).await {
                        {
                            let clients = state.clients.lock().await;
                            if let Some(client) = clients.get(user_id) {
                                *client.room_receiver.lock().await = Some(room_receiver);
                            }
                        }
                        info!("用户 {} 重新连接到房间: {}", user_id, room_id);
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("加入房间失败: {}", e));
                }
            }
        }
        ClientMessage::LeaveRoom { room_id } => {
            // 处理离开房间
            let room_id_parsed = match room::RoomId::parse(&room_id) {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("无效的房间ID: {}", room_id)),
            };

            // 尝试离开房间
            match state.room_manager.leave_room(room_id_parsed, user_id.clone()).await {
                Ok(_) => {
                    // 从房间消息路由器中移除用户
                    state.room_message_router.handle_user_leave_room(user_id.clone()).await;

                    // 清除客户端的房间接收器
                    {
                        let clients = state.clients.lock().await;
                        if let Some(client) = clients.get(user_id) {
                            *client.room_receiver.lock().await = None;
                        }
                    }

                    info!("用户 {} 通过WebSocket离开房间: {}", user_id, room_id);
                    
                    // 广播用户离开房间事件
                    state.broadcast(WsEvent::UserLeftRoom { 
                        room_id: room_id.clone(), 
                        user_id: user_id.clone() 
                    });
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("离开房间失败: {}", e));
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
        // 需要认证的房间路由
        .merge(room::create_protected_room_routes()
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth::middleware::auth_middleware
            )))
        // 公开的房间路由
        .merge(room::create_public_room_routes()
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth::middleware::optional_auth_middleware
            )))
        .merge(create_auth_routes()) // 添加认证API路由
        .nest("/api/friends", create_friend_routes()) // 添加好友API路由
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

/// 房间消息监听任务
async fn room_message_task(
    user_id: UserId,
    state: AppState,
    tx: tokio::sync::mpsc::UnboundedSender<WsEvent>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
    
    loop {
        interval.tick().await;
        
        // 检查用户是否有房间接收器
        let room_receiver = {
            let clients = state.clients.lock().await;
            if let Some(client) = clients.get(&user_id) {
                let mut room_receiver_guard = client.room_receiver.lock().await;
                room_receiver_guard.take()
            } else {
                // 用户已断开连接
                break;
            }
        };
        
        if let Some(mut receiver) = room_receiver {
            // 尝试接收房间消息
            match receiver.try_recv() {
                Ok(event) => {
                    // 转发房间消息到WebSocket
                    if let Err(_) = tx.send(event) {
                        error!("转发房间消息失败，用户可能已断开连接: {}", user_id);
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    // 没有消息，继续监听
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                    // 房间通道已关闭
                    debug!("房间消息通道已关闭，用户: {}", user_id);
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                    // 消息滞后，继续监听
                    warn!("房间消息滞后，用户: {}", user_id);
                }
            }
            
            // 将接收器放回
            {
                let clients = state.clients.lock().await;
                if let Some(client) = clients.get(&user_id) {
                    *client.room_receiver.lock().await = Some(receiver);
                }
            }
        }
    }
    
    debug!("房间消息监听任务结束，用户: {}", user_id);
}
