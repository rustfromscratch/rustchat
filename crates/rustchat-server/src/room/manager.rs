use super::{Room, RoomId, RoomError, CreateRoomRequest};
use rustchat_types::UserId;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 房间管理器
#[derive(Debug)]
pub struct RoomManager {
    /// 房间存储
    rooms: RwLock<HashMap<RoomId, Room>>,
    /// 用户到房间的映射（用户可以在多个房间中）
    user_rooms: RwLock<HashMap<UserId, Vec<RoomId>>>,
}

impl RoomManager {
    /// 创建新的房间管理器
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            user_rooms: RwLock::new(HashMap::new()),
        }
    }
      /// 创建房间
    pub async fn create_room(&self, request: CreateRoomRequest, owner: UserId) -> Result<Room, RoomError> {
        // 验证房间名称
        if request.name.trim().is_empty() {
            return Err(RoomError::InvalidRoomName);
        }
        
        // 创建房间
        let mut room = Room::new(request.name, owner.clone());
        room.set_description(request.description);
        room.set_max_members(request.max_members);
        
        let room_id = room.id;
        
        // 存储房间
        {
            let mut rooms = self.rooms.write().await;
            rooms.insert(room_id, room.clone());
        }
        
        // 更新用户房间映射
        {
            let mut user_rooms = self.user_rooms.write().await;
            user_rooms.entry(owner.clone()).or_insert_with(Vec::new).push(room_id);
        }
        
        info!("用户 {} 创建了房间 '{}' ({})", owner, room.name, room_id);
        Ok(room)
    }
      /// 加入房间
    pub async fn join_room(&self, room_id: RoomId, user_id: UserId) -> Result<Room, RoomError> {
        // 获取并修改房间
        let room = {
            let mut rooms = self.rooms.write().await;
            let room = rooms.get_mut(&room_id).ok_or(RoomError::RoomNotFound)?;
            
            // 检查用户是否已在房间中
            if room.is_member(&user_id) {
                return Err(RoomError::UserAlreadyInRoom);
            }
            
            // 添加成员
            room.add_member(&user_id)?;
            room.clone()
        };
        
        // 更新用户房间映射
        {
            let mut user_rooms = self.user_rooms.write().await;
            user_rooms.entry(user_id.clone()).or_insert_with(Vec::new).push(room_id);
        }
        
        info!("用户 {} 加入了房间 '{}' ({})", user_id, room.name, room_id);
        Ok(room)
    }
      /// 离开房间
    pub async fn leave_room(&self, room_id: RoomId, user_id: UserId) -> Result<Room, RoomError> {
        let room = {
            let mut rooms = self.rooms.write().await;
            let room = rooms.get_mut(&room_id).ok_or(RoomError::RoomNotFound)?;
            
            // 检查用户是否在房间中
            if !room.is_member(&user_id) {
                return Err(RoomError::UserNotInRoom);
            }
            
            // 移除成员
            room.remove_member(&user_id);
            
            // 如果房间为空且不是所有者，删除房间
            if room.members.is_empty() {
                let room_to_remove = room.clone();
                rooms.remove(&room_id);
                debug!("删除空房间: {} ({})", room_to_remove.name, room_id);
                return Ok(room_to_remove);
            }
            
            room.clone()
        };
        
        // 更新用户房间映射
        {
            let mut user_rooms = self.user_rooms.write().await;
            if let Some(rooms) = user_rooms.get_mut(&user_id) {
                rooms.retain(|&id| id != room_id);
                if rooms.is_empty() {
                    user_rooms.remove(&user_id);
                }
            }
        }
        
        info!("用户 {} 离开了房间 '{}' ({})", user_id, room.name, room_id);
        Ok(room)
    }
    
    /// 获取房间信息
    pub async fn get_room(&self, room_id: RoomId) -> Result<Room, RoomError> {
        let rooms = self.rooms.read().await;
        rooms.get(&room_id).cloned().ok_or(RoomError::RoomNotFound)
    }
      /// 获取用户加入的所有房间
    pub async fn get_user_rooms(&self, user_id: &UserId) -> Vec<Room> {
        let user_rooms = self.user_rooms.read().await;
        let rooms = self.rooms.read().await;
        
        if let Some(room_ids) = user_rooms.get(user_id) {
            room_ids.iter()
                .filter_map(|&room_id| rooms.get(&room_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// 获取所有房间列表（分页）
    pub async fn list_rooms(&self, offset: usize, limit: usize) -> Vec<Room> {
        let rooms = self.rooms.read().await;
        rooms.values()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }
      /// 检查用户是否在指定房间中
    pub async fn is_user_in_room(&self, room_id: RoomId, user_id: &UserId) -> bool {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(&room_id) {
            room.is_member(user_id)
        } else {
            false
        }
    }
      /// 获取房间成员列表
    pub async fn get_room_members(&self, room_id: RoomId) -> Result<Vec<UserId>, RoomError> {
        let rooms = self.rooms.read().await;
        let room = rooms.get(&room_id).ok_or(RoomError::RoomNotFound)?;
        Ok(room.members.iter().cloned().collect())
    }
    
    /// 删除房间（仅所有者可以）
    pub async fn delete_room(&self, room_id: RoomId, user_id: UserId) -> Result<Room, RoomError> {
        let room = {
            let mut rooms = self.rooms.write().await;
            let room = rooms.get(&room_id).ok_or(RoomError::RoomNotFound)?;
            
            // 检查权限
            if !room.is_owner(&user_id) {
                return Err(RoomError::PermissionDenied);
            }
            
            let room = room.clone();
            rooms.remove(&room_id);
            room
        };
        
        // 清理用户房间映射
        {
            let mut user_rooms = self.user_rooms.write().await;
            for member in &room.members {
                if let Some(rooms) = user_rooms.get_mut(member) {
                    rooms.retain(|&id| id != room_id);
                    if rooms.is_empty() {
                        user_rooms.remove(member);
                    }
                }
            }
        }
        
        info!("用户 {} 删除了房间 '{}' ({})", user_id, room.name, room_id);
        Ok(room)
    }
      /// 处理用户断线，清理相关数据
    pub async fn handle_user_disconnect(&self, user_id: UserId) {
        let user_room_ids = {
            let user_rooms = self.user_rooms.read().await;
            user_rooms.get(&user_id).cloned().unwrap_or_default()
        };
        
        for room_id in user_room_ids {
            if let Err(e) = self.leave_room(room_id, user_id.clone()).await {
                warn!("用户 {} 断线时离开房间 {} 失败: {}", user_id, room_id, e);
            }
        }
    }
    
    /// 获取房间统计信息
    pub async fn get_stats(&self) -> RoomStats {
        let rooms = self.rooms.read().await;
        let user_rooms = self.user_rooms.read().await;
        
        let total_rooms = rooms.len();
        let total_users = user_rooms.len();
        let total_memberships = user_rooms.values().map(|rooms| rooms.len()).sum();
        
        RoomStats {
            total_rooms,
            total_users,
            total_memberships,
        }
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 房间统计信息
#[derive(Debug, serde::Serialize)]
pub struct RoomStats {
    pub total_rooms: usize,
    pub total_users: usize,
    pub total_memberships: usize,
}
