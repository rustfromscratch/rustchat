use serde::{Deserialize, Serialize};
use crate::user::UserId;

/// 好友请求状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FriendRequestStatus {
    /// 待处理
    Pending,
    /// 已接受
    Accepted,
    /// 已拒绝
    Rejected,
}

/// 好友请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    /// 请求ID
    pub id: String,
    /// 发送者用户ID
    pub from_user_id: UserId,
    /// 接收者用户ID
    pub to_user_id: UserId,
    /// 请求消息
    pub message: Option<String>,
    /// 请求状态
    pub status: FriendRequestStatus,
    /// 创建时间（时间戳）
    pub created_at: i64,
    /// 更新时间（时间戳）
    pub updated_at: i64,
}

/// 好友关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friendship {
    /// 用户ID
    pub user_id: UserId,
    /// 好友用户ID
    pub friend_user_id: UserId,
    /// 创建时间（时间戳）
    pub created_at: i64,
}

impl FriendRequest {
    /// 创建新的好友请求
    pub fn new(from_user_id: UserId, to_user_id: UserId, message: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from_user_id,
            to_user_id,
            message,
            status: FriendRequestStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    /// 接受好友请求
    pub fn accept(&mut self) {
        self.status = FriendRequestStatus::Accepted;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// 拒绝好友请求
    pub fn reject(&mut self) {
        self.status = FriendRequestStatus::Rejected;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

impl Friendship {
    /// 创建新的好友关系
    pub fn new(user_id: UserId, friend_user_id: UserId) -> Self {
        Self {
            user_id,
            friend_user_id,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}
