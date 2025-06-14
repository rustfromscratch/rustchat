use anyhow::{Context, Result};
use rustchat_types::UserId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// 用户配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// 用户ID
    pub user_id: UserId,
    /// 用户昵称
    pub nickname: Option<String>,
    /// 配置文件版本
    pub version: String,
}

impl UserConfig {
    /// 创建新的用户配置
    pub fn new() -> Self {
        Self {
            user_id: UserId::new(),
            nickname: None,
            version: "0.1.0".to_string(),
        }
    }

    /// 设置昵称
    pub fn with_nickname(mut self, nickname: String) -> Self {
        self.nickname = Some(nickname);
        self
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 用户配置管理器
#[derive(Clone)]
pub struct UserConfigManager {
    config_dir: PathBuf,
}

impl UserConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        Ok(Self { config_dir })
    }

    /// 获取配置目录路径
    fn get_config_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .context("无法获取用户主目录")?;
        
        Ok(home_dir.join(".rustchat"))
    }

    /// 获取配置文件路径
    fn get_config_file_path(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    /// 确保配置目录存在
    async fn ensure_config_dir_exists(&self) -> Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)
                .await
                .with_context(|| format!("无法创建配置目录: {:?}", self.config_dir))?;
        }
        Ok(())
    }

    /// 加载用户配置
    pub async fn load_config(&self) -> Result<UserConfig> {
        let config_path = self.get_config_file_path();
        
        if !config_path.exists() {
            // 如果配置文件不存在，创建新的配置
            let config = UserConfig::new();
            self.save_config(&config).await?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .await
            .with_context(|| format!("无法读取配置文件: {:?}", config_path))?;

        let config: UserConfig = serde_json::from_str(&content)
            .with_context(|| format!("无法解析配置文件: {:?}", config_path))?;

        Ok(config)
    }

    /// 保存用户配置
    pub async fn save_config(&self, config: &UserConfig) -> Result<()> {
        self.ensure_config_dir_exists().await?;
        
        let config_path = self.get_config_file_path();
        let content = serde_json::to_string_pretty(config)
            .context("无法序列化配置")?;

        fs::write(&config_path, content)
            .await
            .with_context(|| format!("无法写入配置文件: {:?}", config_path))?;

        Ok(())
    }    /// 更新昵称
    pub async fn update_nickname(&self, nickname: String) -> Result<UserConfig> {
        let mut config = self.load_config().await?;
        config.nickname = Some(nickname);
        self.save_config(&config).await?;
        Ok(config)
    }

    /// 清除昵称
    pub async fn clear_nickname(&self) -> Result<UserConfig> {
        let mut config = self.load_config().await?;
        config.nickname = None;
        self.save_config(&config).await?;
        Ok(config)
    }

    /// 获取用户ID，如果不存在则创建新的
    pub async fn get_or_create_user_id(&self) -> Result<UserId> {
        let config = self.load_config().await?;
        Ok(config.user_id)
    }
}

/// 生成新的用户ID
pub fn generate_user_id() -> UserId {
    UserId::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_config_creation() {
        let config = UserConfig::new();
        assert!(config.nickname.is_none());
        assert_eq!(config.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_user_config_with_nickname() {
        let config = UserConfig::new().with_nickname("Alice".to_string());
        assert_eq!(config.nickname, Some("Alice".to_string()));
    }

    #[tokio::test]
    async fn test_generate_user_id() {
        let id1 = generate_user_id();
        let id2 = generate_user_id();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_config_serialization() {
        let config = UserConfig::new().with_nickname("Bob".to_string());
        
        let json = serde_json::to_string(&config).expect("Should serialize");
        let deserialized: UserConfig = serde_json::from_str(&json).expect("Should deserialize");
        
        assert_eq!(config.user_id, deserialized.user_id);
        assert_eq!(config.nickname, deserialized.nickname);
    }
}
