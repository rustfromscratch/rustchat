use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rustchat_types::{Message, MessageId, MessageType, UserId};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use tracing::{debug, error};

/// 数据库消息记录结构
#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: String,
    pub from_user_id: String,
    pub content_type: String,
    pub content_data: String,
    pub timestamp: DateTime<Utc>,
    pub from_nickname: Option<String>,
}

impl From<&Message> for MessageRecord {
    fn from(msg: &Message) -> Self {
        let (content_type, content_data) = match &msg.content {
            MessageType::Text(text) => ("text".to_string(), text.clone()),
            MessageType::System(text) => ("system".to_string(), text.clone()),
            MessageType::NickChange { old_nick, new_nick } => (
                "nick_change".to_string(),
                serde_json::json!({
                    "old_nick": old_nick,
                    "new_nick": new_nick
                })
                .to_string(),
            ),
        };

        Self {
            id: msg.id.to_string(),
            from_user_id: msg.from.to_string(),
            content_type,
            content_data,
            timestamp: msg.timestamp,
            from_nickname: msg.from_nick.clone(),
        }
    }
}

impl TryFrom<MessageRecord> for Message {
    type Error = anyhow::Error;

    fn try_from(record: MessageRecord) -> Result<Self> {
        let id = MessageId::parse(&record.id)?;
        let from = UserId::parse(&record.from_user_id)?;

        let content = match record.content_type.as_str() {
            "text" => MessageType::Text(record.content_data),
            "system" => MessageType::System(record.content_data),
            "nick_change" => {
                let data: serde_json::Value = serde_json::from_str(&record.content_data)?;
                MessageType::NickChange {
                    old_nick: data["old_nick"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    new_nick: data["new_nick"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown message type: {}", record.content_type)),
        };

        Ok(Message {
            id,
            from,
            content,
            timestamp: record.timestamp,
            from_nick: record.from_nickname,
        })
    }
}

/// 消息历史数据库管理器
pub struct MessageDatabase {
    pool: SqlitePool,
}

impl MessageDatabase {    /// 创建新的数据库管理器
    pub async fn new() -> Result<Self> {
        let db_path = Self::get_database_path()?;
        
        // 确保数据库目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        let database_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
        let pool = SqlitePool::connect(&database_url)
            .await
            .context("Failed to connect to database")?;

        let db = Self { pool };
        db.init_tables().await?;
        
        Ok(db)
    }    /// 获取数据库文件路径
    fn get_database_path() -> Result<PathBuf> {
        // 开发环境：在项目目录下创建数据库
        if let Ok(current_dir) = std::env::current_dir() {
            let db_path = current_dir.join(".rustchat").join("messages.db");
            return Ok(db_path);
        }
        
        // 生产环境：在用户主目录下创建数据库
        let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
        Ok(home_dir.join(".rustchat").join("messages.db"))
    }

    /// 初始化数据库表
    async fn init_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                from_user_id TEXT NOT NULL,
                content_type TEXT NOT NULL,
                content_data TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                from_nickname TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create messages table")?;

        // 创建索引以提高查询性能
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp 
            ON messages(timestamp DESC)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create timestamp index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_user 
            ON messages(from_user_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create user index")?;

        Ok(())
    }    /// 保存消息到数据库
    pub async fn save_message(&self, message: &Message) -> Result<()> {
        let record = MessageRecord::from(message);        // 添加调试信息
        debug!("Saving message to database: id={}, from_user_id={}, content_type={}, content_data={}, timestamp={}, from_nickname={:?}", 
            record.id, record.from_user_id, record.content_type, record.content_data, record.timestamp.to_rfc3339(), record.from_nickname);        let result = sqlx::query(
            r#"
            INSERT OR REPLACE INTO messages (id, from_user_id, content_type, content_data, timestamp, from_nickname)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.from_user_id)
        .bind(&record.content_type)
        .bind(&record.content_data)
        .bind(record.timestamp.to_rfc3339())
        .bind(&record.from_nickname)
        .execute(&self.pool)
        .await;        match result {
            Ok(_) => {
                debug!("Message saved successfully to database");
                Ok(())
            }
            Err(e) => {
                error!("Database error when saving message: {}", e);
                Err(anyhow::Error::from(e).context("Failed to save message"))
            }
        }
    }

    /// 获取最近的消息（默认100条）
    pub async fn get_recent_messages(&self, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_user_id, content_type, content_data, timestamp, from_nickname
            FROM messages
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch recent messages")?;

        let mut messages = Vec::new();
        for row in rows {
            let record = MessageRecord {
                id: row.get("id"),
                from_user_id: row.get("from_user_id"),
                content_type: row.get("content_type"),
                content_data: row.get("content_data"),
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .context("Invalid timestamp format")?
                    .with_timezone(&Utc),
                from_nickname: row.get("from_nickname"),
            };

            match Message::try_from(record) {
                Ok(message) => messages.push(message),
                Err(e) => {
                    eprintln!("Failed to parse message from database: {}", e);
                    continue;
                }
            }
        }

        // 反转以获得正确的时间顺序（最老的在前）
        messages.reverse();
        Ok(messages)
    }

    /// 获取指定用户的消息历史
    pub async fn get_user_messages(&self, user_id: &UserId, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_user_id, content_type, content_data, timestamp, from_nickname
            FROM messages
            WHERE from_user_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(user_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch user messages")?;

        let mut messages = Vec::new();
        for row in rows {
            let record = MessageRecord {
                id: row.get("id"),
                from_user_id: row.get("from_user_id"),
                content_type: row.get("content_type"),
                content_data: row.get("content_data"),
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .context("Invalid timestamp format")?
                    .with_timezone(&Utc),
                from_nickname: row.get("from_nickname"),
            };

            match Message::try_from(record) {
                Ok(message) => messages.push(message),
                Err(e) => {
                    eprintln!("Failed to parse message from database: {}", e);
                    continue;
                }
            }
        }

        messages.reverse();
        Ok(messages)
    }

    /// 获取数据库中的消息总数
    pub async fn get_message_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM messages")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count messages")?;

        Ok(row.get("count"))
    }

    /// 清理旧消息（保留最近的N条）
    pub async fn cleanup_old_messages(&self, keep_count: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM messages
            WHERE id NOT IN (
                SELECT id FROM messages
                ORDER BY timestamp DESC
                LIMIT ?
            )
            "#,
        )
        .bind(keep_count)
        .execute(&self.pool)
        .await
        .context("Failed to cleanup old messages")?;

        Ok(result.rows_affected())
    }

    /// 关闭数据库连接
    pub async fn close(self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustchat_types::{Message, UserId};

    #[tokio::test]
    async fn test_database_operations() {
        // 使用内存数据库进行测试
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to memory database");

        let db = MessageDatabase { pool };
        db.init_tables().await.expect("Failed to init tables");

        // 创建测试消息
        let user_id = UserId::new();
        let message = Message::new_text(
            user_id.clone(),
            "Test message".to_string(),
            Some("TestUser".to_string()),
        );

        // 保存消息
        db.save_message(&message)
            .await
            .expect("Failed to save message");

        // 获取消息
        let messages = db
            .get_recent_messages(10)
            .await
            .expect("Failed to get messages");

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].get_text(), Some("Test message"));
        assert_eq!(messages[0].from_nick, Some("TestUser".to_string()));

        // 测试消息计数
        let count = db.get_message_count().await.expect("Failed to count messages");
        assert_eq!(count, 1);
    }
}
