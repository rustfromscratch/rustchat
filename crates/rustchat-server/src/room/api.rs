use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::room::{CreateRoomRequest, RoomId, RoomResponse, RoomError};
use crate::AppState;
use rustchat_types::UserId;

/// API 路由
pub fn create_room_routes() -> Router<AppState> {
    Router::new()
        .route("/api/rooms", post(create_room))
        .route("/api/rooms", get(list_rooms))
        .route("/api/rooms/{room_id}", get(get_room))
        .route("/api/rooms/{room_id}", delete(delete_room))
        .route("/api/rooms/{room_id}/join", post(join_room))
        .route("/api/rooms/{room_id}/leave", post(leave_room))
        .route("/api/rooms/{room_id}/members", get(get_room_members))
        .route("/api/user/rooms", get(get_user_rooms))
        .route("/api/rooms/stats", get(get_room_stats))
}

/// 查询参数
#[derive(Debug, Deserialize)]
struct ListRoomsQuery {
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct UserIdQuery {
    user_id: String,
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
    Query(user_query): Query<UserIdQuery>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    // 解析用户ID
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // 创建房间
    match state.room_manager.create_room(request, user_id.clone()).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("创建房间失败: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// 获取房间信息
async fn get_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    // 解析ID
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
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
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match state.room_manager.delete_room(room_id, user_id.clone()).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
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
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match state.room_manager.join_room(room_id, user_id.clone()).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
            Ok(Json(ApiResponse::success(response)))
        }
        Err(RoomError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(RoomError::UserAlreadyInRoom) => Err(StatusCode::CONFLICT),
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
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<RoomResponse>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match state.room_manager.leave_room(room_id, user_id.clone()).await {
        Ok(room) => {
            let response = RoomResponse::from_room(&room, &user_id);
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
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let room_id = RoomId::parse(&room_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
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
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<Vec<RoomResponse>>>, StatusCode> {
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
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
    Query(user_query): Query<UserIdQuery>,
) -> Result<Json<ApiResponse<Vec<RoomResponse>>>, StatusCode> {
    let user_id = UserId::parse(&user_query.user_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
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
