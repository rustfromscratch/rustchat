use anyhow::Result;
use async_trait::async_trait;
use rustchat_types::{Message, MessageType, UserId};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, warn};

/// 机器人消息处理结果
#[derive(Debug, Clone)]
pub enum BotResponse {
    /// 回复消息
    Reply(String),
    /// 回复多条消息
    MultiReply(Vec<String>),
    /// 不响应
    NoResponse,
    /// 执行动作（如踢出用户等）
    Action(BotAction),
}

/// 机器人动作
#[derive(Debug, Clone)]
pub enum BotAction {
    /// 踢出用户
    KickUser(UserId),
    /// 禁言用户
    MuteUser(UserId, std::time::Duration),
    /// 发送系统消息
    SystemMessage(String),
}

/// 机器人配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub name: String,
    pub enabled: bool,
    pub triggers: Vec<String>,
    pub description: String,
    pub priority: i32,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            name: "Unknown Bot".to_string(),
            enabled: true,
            triggers: vec![],
            description: "A chat bot".to_string(),
            priority: 0,
        }
    }
}

/// 机器人特征，定义机器人行为接口
#[async_trait]
pub trait Bot: Send + Sync {
    /// 获取机器人配置
    fn config(&self) -> BotConfig;
    
    /// 检查是否应该处理此消息
    fn should_handle(&self, message: &Message) -> bool;
    
    /// 处理消息并返回响应
    async fn handle_message(&self, message: &Message) -> Result<BotResponse>;
    
    /// 初始化机器人（可选）
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 关闭机器人（可选）
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Echo机器人实现
pub struct EchoBot {
    config: BotConfig,
    user_id: UserId,
}

impl EchoBot {
    pub fn new() -> Self {
        Self {
            config: BotConfig {
                name: "Echo Bot".to_string(),
                enabled: true,
                triggers: vec!["@echo".to_string(), "@回声".to_string()],
                description: "回声机器人，会重复用户的消息".to_string(),
                priority: 1,
            },
            user_id: UserId::new(),
        }
    }
    
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
}

#[async_trait]
impl Bot for EchoBot {
    fn config(&self) -> BotConfig {
        self.config.clone()
    }
    
    fn should_handle(&self, message: &Message) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        // 检查消息内容是否包含触发词
        if let MessageType::Text(content) = &message.content {
            return self.config.triggers.iter().any(|trigger| {
                content.to_lowercase().contains(&trigger.to_lowercase())
            });
        }
        
        false
    }
    
    async fn handle_message(&self, message: &Message) -> Result<BotResponse> {
        if let MessageType::Text(content) = &message.content {
            // 移除触发词，获取要回声的内容
            let mut echo_content = content.clone();
            
            for trigger in &self.config.triggers {
                echo_content = echo_content.replace(trigger, "").trim().to_string();
            }
            
            if echo_content.is_empty() {
                return Ok(BotResponse::Reply(
                    "🤖 Echo Bot: 请在 @echo 后面输入要回声的内容".to_string()
                ));
            }
            
            // 添加一些简单的文本处理
            let response = match echo_content.to_lowercase().as_str() {
                "hello" | "你好" | "hi" => {
                    "🤖 Echo Bot: 你好！很高兴见到你！".to_string()
                }
                "time" | "时间" => {
                    let now = chrono::Local::now();
                    format!("🤖 Echo Bot: 当前时间是 {}", now.format("%Y-%m-%d %H:%M:%S"))
                }
                "help" | "帮助" => {
                    "🤖 Echo Bot: 使用方法：@echo <内容> 来让我重复你的话".to_string()
                }
                _ => {
                    format!("🤖 Echo Bot: {}", echo_content)
                }
            };
            
            info!("Echo Bot responding to message from {}: {}", 
                message.from_nick.as_deref().unwrap_or("匿名"), echo_content);
            
            Ok(BotResponse::Reply(response))
        } else {
            Ok(BotResponse::NoResponse)
        }
    }
    
    async fn initialize(&mut self) -> Result<()> {
        info!("Echo Bot 已启动，用户ID: {}", self.user_id);
        Ok(())
    }
}

/// 机器人管理器，负责管理所有机器人
pub struct BotManager {
    bots: Vec<Box<dyn Bot>>,
    message_sender: broadcast::Sender<Message>,
}

impl BotManager {
    pub fn new(message_sender: broadcast::Sender<Message>) -> Self {
        Self {
            bots: Vec::new(),
            message_sender,
        }
    }
    
    /// 注册机器人
    pub fn register_bot(&mut self, bot: Box<dyn Bot>) {
        info!("注册机器人: {}", bot.config().name);
        self.bots.push(bot);
    }
    
    /// 初始化所有机器人
    pub async fn initialize_all(&mut self) -> Result<()> {
        for bot in &mut self.bots {
            bot.initialize().await?;
        }
        Ok(())
    }
    
    /// 处理消息，让所有相关机器人处理
    pub async fn handle_message(&self, message: &Message) -> Result<()> {
        // 按优先级排序处理
        let mut bot_responses = Vec::new();
        
        for bot in &self.bots {
            if bot.should_handle(message) {
                match bot.handle_message(message).await {
                    Ok(response) => {
                        let priority = bot.config().priority;
                        bot_responses.push((priority, response));
                    }
                    Err(e) => {
                        warn!("机器人 {} 处理消息失败: {}", bot.config().name, e);
                    }
                }
            }
        }
        
        // 按优先级排序（高优先级先执行）
        bot_responses.sort_by(|a, b| b.0.cmp(&a.0));
        
        // 执行响应
        for (_, response) in bot_responses {
            self.execute_response(response).await?;
        }
        
        Ok(())
    }
    
    /// 执行机器人响应
    async fn execute_response(&self, response: BotResponse) -> Result<()> {
        match response {
            BotResponse::Reply(content) => {
                self.send_bot_message(content).await?;
            }
            BotResponse::MultiReply(messages) => {
                for content in messages {
                    self.send_bot_message(content).await?;
                    // 稍微延迟，避免消息太快
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
            BotResponse::NoResponse => {
                // 不做任何事
            }
            BotResponse::Action(action) => {
                self.execute_action(action).await?;
            }
        }
        Ok(())
    }
    
    /// 发送机器人消息
    async fn send_bot_message(&self, content: String) -> Result<()> {
        let bot_message = Message::new_text(
            UserId::new(), // 机器人消息使用特殊ID
            content,
            Some("Echo Bot".to_string()),
        );
        
        if let Err(_) = self.message_sender.send(bot_message) {
            warn!("发送机器人消息失败：没有活跃的接收者");
        }
        
        Ok(())
    }
    
    /// 执行机器人动作
    async fn execute_action(&self, _action: BotAction) -> Result<()> {
        // TODO: 实现机器人动作（踢出用户、禁言等）
        warn!("机器人动作暂未实现");
        Ok(())
    }
    
    /// 获取所有机器人信息
    pub fn get_bots_info(&self) -> Vec<BotConfig> {
        self.bots.iter().map(|bot| bot.config()).collect()
    }
    
    /// 关闭所有机器人
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for bot in &mut self.bots {
            bot.shutdown().await?;
        }
        Ok(())
    }
}
