use axum::{
    extract::{Path, Query, State, Extension},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
    middleware,
};
use serde::{Deserialize, Serialize};

use crate::room::{CreateRoomRequest, RoomId, RoomResponse, RoomError};
use crate::AppState;
use crate::auth::{AuthenticatedUser, middleware::auth_middleware, middleware::optional_auth_middleware};
use rustchat_types::{UserId, Message};

/// 创建需要认证的房间路由
pub fn create_protected_room_routes() -> Router<AppState> {
    Router::new()
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{room_id}", delete(delete_room))
        .route("/api/rooms/{room_id}/join", post(join_room))
        .route("/api/rooms/{room_id}/leave", post(leave_room))
        .route("/api/rooms/{room_id}/members", get(get_room_members))
        .route("/api/rooms/{room_id}/messages", get(get_room_messages))
        .route("/api/rooms/{room_id}/messages", post(send_room_message))
        .route("/api/user/rooms", get(get_user_rooms))
}

/// 创建公开的房间路由
pub fn create_public_room_routes() -> Router<AppState> {
    Router::new()
        .route("/api/rooms", get(list_rooms))
        .route("/api/rooms/{room_id}", get(get_room))
        .route("/api/rooms/stats", get(get_room_stats))
}

/// API 路由（兼容性保持）
pub fn create_room_routes() -> Router<AppState> {
    Router::new()
        .merge(create_protected_room_routes())
        .merge(create_public_room_routes())
}

/// 查询参数
#[derive(Debug, Deserialize)]
struct ListRoomsQuery {
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct MessagesQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    content: String,
}

/// API 响应类型
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// 创建房间
async fn create_room(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let user_id = auth_user.user_id;
    
    tracing::info!("create_room: 用户 {} ({}) 请求创建房间: name={}, description={:?}", 
        auth_user.email, user_id, request.name, request.description);
    
    // 创建房间
    match state.room_manager.create_room(request, user_id.clone()).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
            tracing::info!("create_room: 房间创建成功: {} (owner: {})", response.id, user_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("create_room: 创建房间失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 获取房间信息
async fn get_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    auth_user: Option<Extension<AuthenticatedUser>>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    // 解析房间ID
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // 如果有认证用户，使用其ID，否则生成临时ID用于显示
    let user_id = auth_user
        .map(|ext| ext.user_id.clone())
        .unwrap_or_else(|| UserId::new());
    
    match state.room_manager.get_room(room_id).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(RoomError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取房间信息失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 删除房间
async fn delete_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = auth_user.user_id;
    
    match state.room_manager.delete_room(room_id, user_id.clone()).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
            tracing::info!("用户 {} 删除房间: {}", user_id, room_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(RoomError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(RoomError::PermissionDenied) => Err(StatusCode::FORBIDDEN),
        Err(e) => {
            tracing::error!("删除房间失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 加入房间
async fn join_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = auth_user.user_id;
    
    match state.room_manager.join_room(room_id, user_id.clone()).await {
        Ok(room) => {
            // 在房间消息路由器中注册用户
            let _receiver = state.room_message_router.handle_user_enter_room(user_id.clone(), room_id).await;
            
            let response = RoomResponse::from_room(&room, &user_id);
            tracing::info!("用户 {} 加入房间: {} 并注册到消息路由器", user_id, room_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(RoomError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(RoomError::UserAlreadyInRoom) => {
            // 即使用户已经在房间中，也要确保在消息路由器中注册
            let _receiver = state.room_message_router.handle_user_enter_room(user_id.clone(), room_id).await;
            
            // 获取房间信息
            if let Ok(room) = state.room_manager.get_room(room_id).await {
                let response = RoomResponse::from_room(&room, &user_id);
                tracing::info!("用户 {} 已在房间 {} 中，重新注册到消息路由器", user_id, room_id);
                Ok(Json(ApiResponse::success(response)))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(RoomError::RoomFull) => Err(StatusCode::CONFLICT),
        Err(e) => {
            tracing::error!("加入房间失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 离开房间
async fn leave_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = auth_user.user_id;
    
    match state.room_manager.leave_room(room_id, user_id.clone()).await {
        Ok(room) => {
            // 从房间消息路由器中移除用户
            let _left_room_id = state.room_message_router.handle_user_leave_room(user_id.clone()).await;
            
            let response = RoomResponse::from_room(&room, &user_id);
            tracing::info!("用户 {} 离开房间: {} 并从消息路由器中移除", user_id, room_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(RoomError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(RoomError::UserNotInRoom) => Err(StatusCode::CONFLICT),
        Err(e) => {
            tracing::error!("离开房间失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 获取房间成员列表
async fn get_room_members(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = auth_user.user_id;
    
    // 检查权限（只有房间成员可以查看成员列表）
    if !state.room_manager.is_user_in_room(room_id, &user_id).await {
        return Err(StatusCode::FORBIDDEN);
    }
    
    match state.room_manager.get_room_members(room_id).await {
        Ok(members) => {
            let member_strings: Vec<String> = members.iter().map(|id| id.to_string()).collect();
            Ok(Json(ApiResponse::success(member_strings)))
        }
        Err(RoomError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取房间成员失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 获取用户房间列表
async fn get_user_rooms(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<Vec<RoomResponse>>>, StatusCode> {
    let user_id = auth_user.user_id;
    
    let rooms = state.room_manager.get_user_rooms(&user_id).await;
    let responses: Vec<RoomResponse> = rooms.iter()
        .map(|room| RoomResponse::from_room(room, &user_id))
        .collect();
    
    Ok(Json(ApiResponse::success(responses)))
}

/// 获取房间列表
async fn list_rooms(
    State(state): State<AppState>,
    Query(query): Query<ListRoomsQuery>,
    auth_user: Option<Extension<AuthenticatedUser>>,
) -> Result<Json<ApiResponse<Vec<RoomResponse>>>, StatusCode> {
    // 如果有认证用户，使用其ID；否则使用虚拟ID
    let user_id = auth_user
        .map(|ext| ext.user_id.clone())
        .unwrap_or_else(|| UserId::new());
    
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).min(100); // 最大限制100
    
    let rooms = state.room_manager.list_rooms(offset, limit).await;
    let responses: Vec<RoomResponse> = rooms.iter()
        .map(|room| RoomResponse::from_room(room, &user_id))
        .collect();
    
    Ok(Json(ApiResponse::success(responses)))
}

/// 获取房间统计信息
async fn get_room_stats(
    State(state): State<AppState>,
) -> Json<ApiResponse<crate::room::RoomStats>> {
    let stats = state.room_manager.get_stats().await;
    Json(ApiResponse::success(stats))
}

/// 获取房间消息
async fn get_room_messages(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Query(query): Query<MessagesQuery>,
    Extension(auth_user): Extension<AuthenticatedUser>,
) -> Result<Json<ApiResponse<Vec<Message>>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = auth_user.user_id;
    
    // 检查用户是否为房间成员
    if !state.room_manager.is_user_in_room(room_id, &user_id).await {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    
    // 从消息数据库获取房间消息
    match state.message_db.get_room_messages(&room_id.to_string(), limit, offset).await {
        Ok(messages) => Ok(Json(ApiResponse::success(messages))),
        Err(e) => {
            tracing::error!("获取房间消息失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 发送房间消息
async fn send_room_message(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(auth_user): Extension<AuthenticatedUser>,
    Json(request): Json<SendMessageRequest>,
) -> Result<Json<ApiResponse<Message>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = auth_user.user_id;
    
    // 检查用户是否为房间成员
    if !state.room_manager.is_user_in_room(room_id, &user_id).await {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // 创建消息
    let message = Message::new_text(
        user_id.clone(), 
        request.content.clone(), 
        None
    );
    
    // 设置消息的房间ID
    let mut room_message = message;
    room_message.additional_data = Some(serde_json::json!({
        "room_id": room_id.to_string()
    }));    // 保存消息到数据库
    if let Err(e) = state.message_db.save_message(&room_message).await {
        tracing::error!("保存房间消息失败: {}", e);
        return Ok(Json(ApiResponse::error(e.to_string())));
    }
    
    // 广播消息给房间成员（完整方案）
    if let Err(e) = state.room_message_router.route_message(room_message.clone(), user_id.clone()).await {
        tracing::error!("广播房间消息失败: {}", e);
    } else {
        tracing::info!("房间消息已广播: room_id={}, user_id={}", room_id, user_id);
    }
    
    Ok(Json(ApiResponse::success(room_message)))
}
