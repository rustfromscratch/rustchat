use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// 用户唯一标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户ID
    pub id: UserId,
    /// 用户昵称
    pub nickname: Option<String>,
    /// 用户名（登录用）
    pub username: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 创建时间（时间戳）
    pub created_at: i64,
    /// 最后活跃时间（时间戳）
    pub last_active_at: i64,
}

impl User {
    /// 创建新用户
    pub fn new(id: UserId) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            nickname: None,
            username: None,
            email: None,
            created_at: now,
            last_active_at: now,
        }
    }

    /// 创建新用户并设置昵称
    pub fn with_nickname(id: UserId, nickname: String) -> Self {
        let mut user = Self::new(id);
        user.nickname = Some(nickname);
        user
    }
}

impl UserId {
    /// 生成新的用户ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 从UUID创建用户ID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// 从字符串解析用户ID
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// 获取内部UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// 转换为字符串
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<UserId> for Uuid {
    fn from(user_id: UserId) -> Self {
        user_id.0
    }
}

impl std::str::FromStr for UserId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_generation() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        
        // 两个新生成的ID应该不同
        assert_ne!(id1, id2);
        
        // ID应该是有效的UUID v4
        assert_eq!(id1.as_uuid().get_version_num(), 4);
    }

    #[test]
    fn test_user_id_parsing() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let user_id = UserId::parse(uuid_str).expect("Should parse valid UUID");
        
        assert_eq!(user_id.to_string(), uuid_str);
    }

    #[test]
    fn test_user_id_serialization() {
        let user_id = UserId::new();
        
        // 测试JSON序列化
        let json = serde_json::to_string(&user_id).expect("Should serialize");
        let deserialized: UserId = serde_json::from_str(&json).expect("Should deserialize");
        
        assert_eq!(user_id, deserialized);
    }
}
