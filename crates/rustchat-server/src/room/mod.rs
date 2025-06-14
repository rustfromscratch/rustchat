mod manager;
mod api;
mod broadcast;

pub use manager::{RoomManager, RoomStats};
pub use api::create_room_routes;
pub use broadcast::{RoomBroadcastManager, RoomMessageRouter, BroadcastStats};

use rustchat_types::UserId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// 房间唯一标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub Uuid);

impl RoomId {
    /// 生成新的房间ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// 从字符串解析房间ID
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
    
    /// 转换为字符串
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for RoomId {
    fn default() -> Self {
        Self::new()
    }
}

/// 房间信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// 房间ID
    pub id: RoomId,
    /// 房间名称
    pub name: String,
    /// 房间创建者
    pub owner: UserId,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 房间成员
    pub members: HashSet<UserId>,
    /// 房间描述（可选）
    pub description: Option<String>,
    /// 最大成员数（可选，None表示无限制）
    pub max_members: Option<usize>,
}

impl Room {    /// 创建新房间
    pub fn new(name: String, owner: UserId) -> Self {
        let mut members = HashSet::new();
        members.insert(owner.clone());
        
        Self {
            id: RoomId::new(),
            name,
            owner,
            created_at: chrono::Utc::now(),
            members,
            description: None,
            max_members: None,
        }
    }
      /// 添加成员
    pub fn add_member(&mut self, user_id: &UserId) -> Result<bool, RoomError> {
        // 检查房间是否已满
        if let Some(max) = self.max_members {
            if self.members.len() >= max {
                return Err(RoomError::RoomFull);
            }
        }
        
        // 添加成员（如果已存在则返回false）
        Ok(self.members.insert(user_id.clone()))
    }
      /// 移除成员
    pub fn remove_member(&mut self, user_id: &UserId) -> bool {
        self.members.remove(user_id)
    }
    
    /// 检查用户是否为房间成员
    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members.contains(user_id)
    }
    
    /// 检查用户是否为房间所有者
    pub fn is_owner(&self, user_id: &UserId) -> bool {
        self.owner == *user_id
    }
    
    /// 获取成员数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
    
    /// 设置房间描述
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }
    
    /// 设置最大成员数
    pub fn set_max_members(&mut self, max_members: Option<usize>) {
        self.max_members = max_members;
    }
}

/// 房间相关错误
#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("房间不存在")]
    RoomNotFound,
    #[error("用户未加入房间")]
    UserNotInRoom,
    #[error("用户已在房间中")]
    UserAlreadyInRoom,
    #[error("房间已满")]
    RoomFull,
    #[error("权限不足")]
    PermissionDenied,
    #[error("房间名称无效")]
    InvalidRoomName,
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] anyhow::Error),
}

/// 房间创建请求
#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub description: Option<String>,
    pub max_members: Option<usize>,
}

/// 房间信息响应
#[derive(Debug, Serialize)]
pub struct RoomResponse {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub member_count: usize,
    pub description: Option<String>,
    pub max_members: Option<usize>,
    pub is_member: bool,
    pub is_owner: bool,
}

impl RoomResponse {
    pub fn from_room(room: &Room, requester: &UserId) -> Self {
        Self {
            id: room.id.to_string(),
            name: room.name.clone(),
            owner: room.owner.to_string(),
            created_at: room.created_at,
            member_count: room.member_count(),
            description: room.description.clone(),
            max_members: room.max_members,
            is_member: room.is_member(requester),
            is_owner: room.is_owner(requester),
        }
    }
}
