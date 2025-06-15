use rustchat_types::{UserId, FriendRequest, FriendRequestStatus};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use tracing::info;

/// 好友管理器
pub struct FriendManager {
    friend_requests: RwLock<HashMap<String, FriendRequest>>,
    friendships: RwLock<HashMap<UserId, HashSet<UserId>>>,
}

impl FriendManager {
    /// 创建新的好友管理器
    pub fn new() -> Self {
        Self {
            friend_requests: RwLock::new(HashMap::new()),
            friendships: RwLock::new(HashMap::new()),
        }
    }
      /// 发送好友请求
    pub async fn send_friend_request(&mut self, from_user_id: UserId, to_user_id: UserId, message: Option<String>) -> Result<FriendRequest, FriendError> {
        // 检查是否为自己
        if from_user_id == to_user_id {
            return Err(FriendError::CannotAddSelf);
        }
        
        // 检查是否已经是好友
        if self.are_friends(&from_user_id, &to_user_id).await {
            return Err(FriendError::RelationshipAlreadyExists);
        }
        
        // 检查是否已有待处理的请求
        let friend_requests = self.friend_requests.read().await;
        for request in friend_requests.values() {
            if request.from_user_id == from_user_id && request.to_user_id == to_user_id && request.status == FriendRequestStatus::Pending {
                return Err(FriendError::RelationshipAlreadyExists);
            }
        }
        drop(friend_requests);
        
        let request = FriendRequest::new(from_user_id.clone(), to_user_id.clone(), message);
        let request_id = request.id.clone();
        
        // 存储好友请求
        {
            let mut friend_requests = self.friend_requests.write().await;
            friend_requests.insert(request_id, request.clone());
        }
        
        info!("用户 {} 向用户 {} 发送了好友请求", from_user_id, to_user_id);
        Ok(request)
    }
    
    /// 接受好友请求
    pub async fn accept_friend_request(&mut self, request_id: &str) -> Result<FriendRequest, FriendError> {
        let mut friend_requests = self.friend_requests.write().await;
        let request = friend_requests.get_mut(request_id).ok_or(FriendError::FriendshipNotFound)?;
        
        // 检查状态是否为pending
        if request.status != FriendRequestStatus::Pending {
            return Err(FriendError::InvalidStatus);
        }
        
        request.accept();
        
        // 添加到好友列表
        {
            let mut friendships = self.friendships.write().await;
            friendships.entry(request.from_user_id.clone()).or_insert_with(HashSet::new).insert(request.to_user_id.clone());
            friendships.entry(request.to_user_id.clone()).or_insert_with(HashSet::new).insert(request.from_user_id.clone());
        }
        
        info!("用户 {} 接受了来自用户 {} 的好友请求", request.to_user_id, request.from_user_id);
        Ok(request.clone())
    }
    
    /// 拒绝好友请求
    pub async fn reject_friend_request(&mut self, request_id: &str) -> Result<FriendRequest, FriendError> {
        let mut friend_requests = self.friend_requests.write().await;
        let request = friend_requests.get_mut(request_id).ok_or(FriendError::FriendshipNotFound)?;
        
        // 检查状态是否为pending
        if request.status != FriendRequestStatus::Pending {
            return Err(FriendError::InvalidStatus);
        }
        
        request.reject();
        
        info!("用户 {} 拒绝了来自用户 {} 的好友请求", request.to_user_id, request.from_user_id);
        Ok(request.clone())
    }
    
    /// 获取用户的好友请求（收到的和发送的）
    pub async fn get_friend_requests(&self, user_id: UserId) -> Result<Vec<FriendRequest>, FriendError> {
        let friend_requests = self.friend_requests.read().await;
        
        let requests: Vec<FriendRequest> = friend_requests.values()
            .filter(|request| {
                (request.from_user_id == user_id || request.to_user_id == user_id) &&
                request.status == FriendRequestStatus::Pending
            })
            .cloned()
            .collect();
        
        Ok(requests)
    }
    
    /// 获取用户的好友列表
    pub async fn get_friends(&self, user_id: UserId) -> Result<Vec<UserId>, FriendError> {
        let friendships = self.friendships.read().await;
        
        if let Some(friends) = friendships.get(&user_id) {
            Ok(friends.iter().cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }
    
    /// 删除好友关系
    pub async fn remove_friend(&mut self, user_id: UserId, friend_user_id: UserId) -> Result<(), FriendError> {
        let mut friendships = self.friendships.write().await;
        
        // 从两个用户的好友列表中移除
        if let Some(friends) = friendships.get_mut(&user_id) {
            friends.remove(&friend_user_id);
        }
        if let Some(friends) = friendships.get_mut(&friend_user_id) {
            friends.remove(&user_id);
        }
        
        info!("用户 {} 删除了与用户 {} 的好友关系", user_id, friend_user_id);
        Ok(())
    }
    
    /// 检查两个用户是否为好友
    pub async fn are_friends(&self, user1: &UserId, user2: &UserId) -> bool {
        let friendships = self.friendships.read().await;
        
        if let Some(friends) = friendships.get(user1) {
            friends.contains(user2)
        } else {
            false
        }
    }
}

/// 好友相关错误
#[derive(Debug, thiserror::Error)]
pub enum FriendError {
    #[error("好友关系不存在")]
    FriendshipNotFound,
    #[error("不能添加自己为好友")]
    CannotAddSelf,
    #[error("好友关系已存在")]
    RelationshipAlreadyExists,
    #[error("没有权限执行此操作")]
    NotAuthorized,
    #[error("好友关系状态无效")]
    InvalidStatus,
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] anyhow::Error),
}
