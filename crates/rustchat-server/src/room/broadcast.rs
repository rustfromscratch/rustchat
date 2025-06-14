use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, warn};

use crate::room::RoomId;
use crate::WsEvent;
use rustchat_types::{Message, UserId};

/// 房间广播管理器
#[derive(Debug, Clone)]
pub struct RoomBroadcastManager {
    /// 房间订阅者映射: room_id -> broadcast_sender
    room_channels: Arc<RwLock<HashMap<RoomId, broadcast::Sender<WsEvent>>>>,
    /// 用户当前所在房间映射
    user_current_room: Arc<RwLock<HashMap<UserId, RoomId>>>,
}

impl RoomBroadcastManager {
    /// 创建新的房间广播管理器
    pub fn new() -> Self {
        Self {
            room_channels: Arc::new(RwLock::new(HashMap::new())),
            user_current_room: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 为房间创建广播通道
    pub async fn create_room_channel(&self, room_id: RoomId) -> broadcast::Sender<WsEvent> {
        let mut channels = self.room_channels.write().await;
        
        if let Some(sender) = channels.get(&room_id) {
            sender.clone()
        } else {
            let (sender, _) = broadcast::channel(1000);
            channels.insert(room_id, sender.clone());
            debug!("为房间 {} 创建广播通道", room_id);
            sender
        }
    }
      /// 用户进入房间
    pub async fn user_enter_room(&self, user_id: UserId, room_id: RoomId) -> Option<broadcast::Receiver<WsEvent>> {
        // 创建或获取房间广播通道
        let sender = self.create_room_channel(room_id).await;
        let receiver = sender.subscribe();
        
        // 更新用户当前房间
        {
            let mut user_rooms = self.user_current_room.write().await;
            user_rooms.insert(user_id.clone(), room_id);
        }
        
        debug!("用户 {} 进入房间 {} 的广播通道", user_id, room_id);
        Some(receiver)
    }
    
    /// 用户离开当前房间
    pub async fn user_leave_current_room(&self, user_id: UserId) -> Option<RoomId> {
        let mut user_rooms = self.user_current_room.write().await;
        let room_id = user_rooms.remove(&user_id);
        
        if let Some(room_id) = room_id {
            debug!("用户 {} 离开房间 {}", user_id, room_id);
        }
        
        room_id
    }
    
    /// 获取用户当前所在房间
    pub async fn get_user_current_room(&self, user_id: UserId) -> Option<RoomId> {
        let user_rooms = self.user_current_room.read().await;
        user_rooms.get(&user_id).copied()
    }
    
    /// 向指定房间广播消息
    pub async fn broadcast_to_room(&self, room_id: RoomId, event: WsEvent) -> Result<usize, broadcast::error::SendError<WsEvent>> {
        let channels = self.room_channels.read().await;
        
        if let Some(sender) = channels.get(&room_id) {
            match sender.send(event) {
                Ok(receiver_count) => {
                    debug!("向房间 {} 广播消息，接收者数量: {}", room_id, receiver_count);
                    Ok(receiver_count)
                }
                Err(e) => {
                    warn!("向房间 {} 广播消息失败: {}", room_id, e);
                    Err(e)
                }
            }
        } else {
            warn!("房间 {} 的广播通道不存在", room_id);
            Ok(0)
        }
    }
      /// 向用户当前所在房间广播消息
    pub async fn broadcast_to_user_room(&self, user_id: UserId, event: WsEvent) -> Result<Option<usize>, broadcast::error::SendError<WsEvent>> {
        if let Some(room_id) = self.get_user_current_room(user_id.clone()).await {
            let count = self.broadcast_to_room(room_id, event).await?;
            Ok(Some(count))
        } else {
            debug!("用户 {} 不在任何房间中", user_id);
            Ok(None)
        }
    }
    
    /// 清理空的房间广播通道
    pub async fn cleanup_empty_channels(&self) {
        let mut channels = self.room_channels.write().await;
        let mut to_remove = Vec::new();
        
        for (room_id, sender) in channels.iter() {
            if sender.receiver_count() == 0 {
                to_remove.push(*room_id);
            }
        }
        
        for room_id in to_remove {
            channels.remove(&room_id);
            debug!("清理空的房间广播通道: {}", room_id);
        }
    }
    
    /// 获取房间广播统计信息
    pub async fn get_broadcast_stats(&self) -> BroadcastStats {
        let channels = self.room_channels.read().await;
        let user_rooms = self.user_current_room.read().await;
        
        let total_rooms = channels.len();
        let total_users_in_rooms = user_rooms.len();
        let total_subscribers: usize = channels.values()
            .map(|sender| sender.receiver_count())
            .sum();
        
        BroadcastStats {
            total_rooms,
            total_users_in_rooms,
            total_subscribers,
        }
    }
    
    /// 处理用户断线
    pub async fn handle_user_disconnect(&self, user_id: UserId) {
        self.user_leave_current_room(user_id).await;
    }
}

impl Default for RoomBroadcastManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 广播统计信息
#[derive(Debug, serde::Serialize)]
pub struct BroadcastStats {
    pub total_rooms: usize,
    pub total_users_in_rooms: usize,
    pub total_subscribers: usize,
}

/// 房间消息路由器
#[derive(Debug)]
pub struct RoomMessageRouter {
    broadcast_manager: RoomBroadcastManager,
}

impl RoomMessageRouter {
    /// 创建新的房间消息路由器
    pub fn new(broadcast_manager: RoomBroadcastManager) -> Self {
        Self {
            broadcast_manager,
        }
    }
    
    /// 路由消息到适当的房间
    pub async fn route_message(&self, message: Message, sender_id: UserId) -> Result<usize, String> {
        // 获取发送者当前所在房间
        if let Some(room_id) = self.broadcast_manager.get_user_current_room(sender_id).await {
            // 创建WebSocket事件
            let event = WsEvent::Message(message);
            
            // 广播到房间
            match self.broadcast_manager.broadcast_to_room(room_id, event).await {
                Ok(count) => Ok(count),
                Err(e) => Err(format!("广播消息失败: {}", e)),
            }
        } else {
            Err("用户不在任何房间中".to_string())
        }
    }
    
    /// 处理用户进入房间
    pub async fn handle_user_enter_room(&self, user_id: UserId, room_id: RoomId) -> Option<broadcast::Receiver<WsEvent>> {
        self.broadcast_manager.user_enter_room(user_id, room_id).await
    }
    
    /// 处理用户离开房间
    pub async fn handle_user_leave_room(&self, user_id: UserId) -> Option<RoomId> {
        self.broadcast_manager.user_leave_current_room(user_id).await
    }
      /// 向房间发送系统消息
    pub async fn send_system_message_to_room(&self, room_id: RoomId, content: String) -> Result<usize, String> {
        let message = Message::new_system(content);
        let event = WsEvent::Message(message);
        
        match self.broadcast_manager.broadcast_to_room(room_id, event).await {
            Ok(count) => Ok(count),
            Err(e) => Err(format!("发送系统消息失败: {}", e)),
        }
    }
}
