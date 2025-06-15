use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::user::UserId;

/// 消息唯一标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(uuid::Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    /// 从字符串解析MessageId
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(uuid::Uuid::parse_str(s)?))
    }

    /// 获取内部UUID
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<uuid::Uuid> for MessageId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl From<MessageId> for uuid::Uuid {
    fn from(message_id: MessageId) -> Self {
        message_id.0
    }
}

impl std::str::FromStr for MessageId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// 消息类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    /// 普通文本消息
    Text(String),
    /// 系统消息
    System(String),
    /// 昵称变更消息
    NickChange { old_nick: String, new_nick: String },
}

/// 消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息ID
    pub id: MessageId,
    /// 发送者ID
    pub from: UserId,
    /// 消息内容
    pub content: MessageType,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 发送者昵称（可选，用于显示）
    pub from_nick: Option<String>,
    /// 房间ID（可选，用于房间消息）
    pub room_id: Option<String>,
    /// 附加数据（可选，JSON格式）
    pub additional_data: Option<serde_json::Value>,
}

impl Message {    /// 创建新的文本消息
    pub fn new_text(from: UserId, text: String, from_nick: Option<String>) -> Self {
        Self {
            id: MessageId::new(),
            from,
            content: MessageType::Text(text),
            timestamp: Utc::now(),
            from_nick,
            room_id: None,
            additional_data: None,
        }
    }    /// 创建系统消息
    pub fn new_system(text: String) -> Self {
        Self {
            id: MessageId::new(),
            from: UserId::new(), // 系统消息使用随机ID
            content: MessageType::System(text),
            timestamp: Utc::now(),
            from_nick: Some("System".to_string()),
            room_id: None,
            additional_data: None,
        }
    }    /// 创建昵称变更消息
    pub fn new_nick_change(
        from: UserId,
        old_nick: String,
        new_nick: String,
        from_nick: Option<String>,
    ) -> Self {
        Self {
            id: MessageId::new(),
            from,
            content: MessageType::NickChange { old_nick, new_nick },
            timestamp: Utc::now(),
            from_nick,
            room_id: None,
            additional_data: None,
        }
    }

    /// 创建房间文本消息
    pub fn new_room_text(
        from: UserId, 
        text: String, 
        from_nick: Option<String>,
        room_id: String
    ) -> Self {
        Self {
            id: MessageId::new(),
            from,
            content: MessageType::Text(text),
            timestamp: Utc::now(),
            from_nick,
            room_id: Some(room_id.clone()),
            additional_data: Some(serde_json::json!({
                "room_id": room_id
            })),
        }
    }

    /// 设置房间ID
    pub fn set_room_id(&mut self, room_id: String) {
        self.room_id = Some(room_id.clone());
        if self.additional_data.is_none() {
            self.additional_data = Some(serde_json::json!({}));
        }
        if let Some(ref mut data) = self.additional_data {
            data["room_id"] = serde_json::Value::String(room_id);
        }
    }

    /// 获取房间ID
    pub fn get_room_id(&self) -> Option<&str> {
        self.room_id.as_deref()
    }

    /// 获取消息的文本内容（如果是文本消息）
    pub fn get_text(&self) -> Option<&str> {
        match &self.content {
            MessageType::Text(text) => Some(text),
            _ => None,
        }
    }

    /// 获取消息内容的字符串表示（用于显示）
    pub fn get_body(&self) -> String {
        match &self.content {
            MessageType::Text(text) => text.clone(),
            MessageType::System(text) => format!("[系统] {}", text),
            MessageType::NickChange { old_nick, new_nick } => {
                format!("{} 将昵称改为 {}", old_nick, new_nick)
            }
        }
    }

    /// 检查是否为系统消息
    pub fn is_system(&self) -> bool {
        matches!(self.content, MessageType::System(_))
    }

    /// 检查是否为文本消息
    pub fn is_text(&self) -> bool {
        matches!(self.content, MessageType::Text(_))
    }

    /// 检查是否为昵称变更消息
    pub fn is_nick_change(&self) -> bool {
        matches!(self.content, MessageType::NickChange { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_id = UserId::new();
        let message = Message::new_text(
            user_id.clone(),
            "Hello, world!".to_string(),
            Some("Alice".to_string()),
        );

        assert_eq!(message.from, user_id);
        if let MessageType::Text(text) = &message.content {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text message");
        }
        assert_eq!(message.from_nick, Some("Alice".to_string()));
    }

    #[test]
    fn test_message_serialization() {
        let user_id = UserId::new();
        let message = Message::new_text(
            user_id,
            "Test message".to_string(),
            Some("Bob".to_string()),
        );

        let json = serde_json::to_string(&message).expect("Should serialize");
        let deserialized: Message = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(message.id.to_string(), deserialized.id.to_string());
        assert_eq!(message.from, deserialized.from);
    }

    #[test]
    fn test_message_body_methods() {
        let user_id = UserId::new();

        // 测试文本消息
        let text_msg = Message::new_text(
            user_id.clone(),
            "Hello World".to_string(),
            Some("Alice".to_string()),
        );
        assert_eq!(text_msg.get_text(), Some("Hello World"));
        assert_eq!(text_msg.get_body(), "Hello World");
        assert!(text_msg.is_text());
        assert!(!text_msg.is_system());
        assert!(!text_msg.is_nick_change());

        // 测试系统消息
        let system_msg = Message::new_system("Server started".to_string());
        assert_eq!(system_msg.get_text(), None);
        assert_eq!(system_msg.get_body(), "[系统] Server started");
        assert!(!system_msg.is_text());
        assert!(system_msg.is_system());
        assert!(!system_msg.is_nick_change());

        // 测试昵称变更消息
        let nick_msg = Message::new_nick_change(
            user_id,
            "OldName".to_string(),
            "NewName".to_string(),
            Some("OldName".to_string()),
        );
        assert_eq!(nick_msg.get_text(), None);
        assert_eq!(nick_msg.get_body(), "OldName 将昵称改为 NewName");
        assert!(!nick_msg.is_text());
        assert!(!nick_msg.is_system());
        assert!(nick_msg.is_nick_change());
    }

    #[test]
    fn test_message_id_generation() {
        let user_id = UserId::new();
        let msg1 = Message::new_text(user_id.clone(), "Test 1".to_string(), None);
        let msg2 = Message::new_text(user_id, "Test 2".to_string(), None);

        // 每个消息应该有唯一的ID
        assert_ne!(msg1.id.to_string(), msg2.id.to_string());
    }

    #[test]
    fn test_message_id_parsing() {
        let id = MessageId::new();
        let id_str = id.to_string();

        // 测试解析
        let parsed_id = MessageId::parse(&id_str).expect("Should parse valid UUID");
        assert_eq!(id.to_string(), parsed_id.to_string());

        // 测试FromStr trait
        let from_str_id: MessageId = id_str.parse().expect("Should parse via FromStr");
        assert_eq!(id.to_string(), from_str_id.to_string());
    }

    #[test]
    fn test_message_id_display() {
        let id = MessageId::new();
        let display_str = format!("{}", id);
        let to_string_str = id.to_string();

        assert_eq!(display_str, to_string_str);
    }    #[test]
    fn test_message_id_conversions() {
        let uuid = uuid::Uuid::new_v4();
        let id = MessageId::from(uuid);
        
        // 测试as_uuid方法
        assert_eq!(&uuid, id.as_uuid());
        
        // 测试转换为UUID（会消费id）
        let converted_uuid: uuid::Uuid = id.into();
        assert_eq!(uuid, converted_uuid);
    }
}
