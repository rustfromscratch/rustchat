use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use rustchat_types::UserId;
use serde::Deserialize;
use tracing::{debug, error, info};

/// 创建好友路由
pub fn create_friend_routes() -> Router<crate::AppState> {
    Router::new()
        .route("/request", post(send_friend_request))
        .route("/request/respond", post(respond_friend_request))
        .route("/requests", get(get_friend_requests))
        .route("/list", get(get_friends))
        .route("/remove", delete(remove_friend))
}

/// 发送好友请求的请求体
#[derive(Debug, Deserialize)]
struct SendFriendRequestBody {
    to_user_id: UserId,
    message: Option<String>,
}

/// 响应好友请求的请求体
#[derive(Debug, Deserialize)]
struct RespondFriendRequestBody {
    request_id: String,
    accept: bool,
}

/// 删除好友的查询参数
#[derive(Debug, Deserialize)]
struct RemoveFriendQuery {
    user_id: UserId,
    friend_user_id: UserId,
}

/// 获取好友列表/请求的查询参数
#[derive(Debug, Deserialize)]
struct GetFriendsQuery {
    user_id: UserId,
}

/// 发送好友请求
async fn send_friend_request(
    Query(query): Query<GetFriendsQuery>,
    State(state): State<crate::AppState>,
    Json(body): Json<SendFriendRequestBody>,
) -> impl IntoResponse {
    debug!(
        "Sending friend request from {} to {} with message: {:?}",
        query.user_id, body.to_user_id, body.message
    );

    let mut manager = state.friend_manager.lock().await;
    
    match manager.send_friend_request(query.user_id.clone(), body.to_user_id.clone(), body.message).await {
        Ok(request) => {
            info!(
                "Friend request sent from {} to {}, request_id: {}",
                query.user_id, body.to_user_id, request.id
            );
            Json(request).into_response()
        }
        Err(e) => {
            error!("Failed to send friend request: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to send friend request: {}", e)).into_response()
        }
    }
}

/// 响应好友请求（接受或拒绝）
async fn respond_friend_request(
    Query(query): Query<GetFriendsQuery>,
    State(state): State<crate::AppState>,
    Json(body): Json<RespondFriendRequestBody>,
) -> impl IntoResponse {
    debug!(
        "User {} responding to friend request {}: {}",
        query.user_id, body.request_id, if body.accept { "accept" } else { "reject" }
    );

    let mut manager = state.friend_manager.lock().await;
    
    let result = if body.accept {
        manager.accept_friend_request(&body.request_id).await
    } else {
        manager.reject_friend_request(&body.request_id).await
    };

    match result {
        Ok(request) => {
            info!(
                "Friend request {} {} by user {}",
                body.request_id,
                if body.accept { "accepted" } else { "rejected" },
                query.user_id
            );
            Json(request).into_response()
        }
        Err(e) => {
            error!("Failed to respond to friend request: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to respond to friend request: {}", e)).into_response()
        }
    }
}

/// 获取好友请求列表
async fn get_friend_requests(
    Query(query): Query<GetFriendsQuery>,
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    debug!("Getting friend requests for user {}", query.user_id);

    let manager = state.friend_manager.lock().await;
    
    match manager.get_friend_requests(query.user_id.clone()).await {
        Ok(requests) => {
            debug!("Found {} friend requests for user {}", requests.len(), query.user_id);
            Json(requests).into_response()
        }
        Err(e) => {
            error!("Failed to get friend requests: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get friend requests: {}", e)).into_response()
        }
    }
}

/// 获取好友列表
async fn get_friends(
    Query(query): Query<GetFriendsQuery>,
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    debug!("Getting friends for user {}", query.user_id);

    let manager = state.friend_manager.lock().await;
    
    match manager.get_friends(query.user_id.clone()).await {
        Ok(friends) => {
            debug!("Found {} friends for user {}", friends.len(), query.user_id);
            Json(friends).into_response()
        }
        Err(e) => {
            error!("Failed to get friends: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get friends: {}", e)).into_response()
        }
    }
}

/// 删除好友
async fn remove_friend(
    Query(query): Query<RemoveFriendQuery>,
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    debug!("Removing friend {} for user {}", query.friend_user_id, query.user_id);

    let mut manager = state.friend_manager.lock().await;
    
    match manager.remove_friend(query.user_id.clone(), query.friend_user_id.clone()).await {
        Ok(_) => {
            info!("Friend {} removed for user {}", query.friend_user_id, query.user_id);
            StatusCode::OK.into_response()
        }
        Err(e) => {
            error!("Failed to remove friend: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to remove friend: {}", e)).into_response()
        }
    }
}
