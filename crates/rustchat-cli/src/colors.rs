use crossterm::{
    style::{Color, ResetColor, SetForegroundColor, Stylize},
    ExecutableCommand,
};
use rustchat_types::{Message, MessageType};
use std::io::{self, Write};

/// 颜色主题配置
#[derive(Clone)]
pub struct ColorTheme {
    pub timestamp_color: Color,
    pub username_color: Color,
    pub bot_color: Color,
    pub system_color: Color,
    pub text_color: Color,
    pub error_color: Color,
    pub success_color: Color,
    pub info_color: Color,
}

impl ColorTheme {
    /// 默认颜色主题
    pub fn default() -> Self {
        Self {
            timestamp_color: Color::DarkGrey,
            username_color: Color::Cyan,
            bot_color: Color::Green,
            system_color: Color::Yellow,
            text_color: Color::White,
            error_color: Color::Red,
            success_color: Color::Green,
            info_color: Color::Blue,
        }
    }

    /// 彩虹颜色主题（为用户名分配不同颜色）
    pub fn rainbow() -> Self {
        Self {
            timestamp_color: Color::DarkGrey,
            username_color: Color::Magenta, // 这个会被动态替换
            bot_color: Color::Green,
            system_color: Color::Yellow,
            text_color: Color::White,
            error_color: Color::Red,
            success_color: Color::Green,
            info_color: Color::Cyan,
        }
    }
}

/// 颜色显示工具
#[derive(Clone)]
pub struct ColorDisplay {
    theme: ColorTheme,
    username_colors: Vec<Color>,
}

impl ColorDisplay {
    pub fn new() -> Self {
        Self {
            theme: ColorTheme::default(),
            username_colors: vec![
                Color::Cyan,
                Color::Magenta,
                Color::Blue,
                Color::Green,
                Color::Yellow,
                Color::Red,
                Color::DarkCyan,
                Color::DarkMagenta,
            ],
        }
    }

    /// 获取用户名颜色（基于用户名哈希分配）
    fn get_username_color(&self, username: &str) -> Color {
        let hash = username.chars().map(|c| c as usize).sum::<usize>();
        let index = hash % self.username_colors.len();
        self.username_colors[index]
    }

    /// 格式化并显示消息
    pub fn display_message(&self, msg: &Message) {
        let mut stdout = io::stdout();
        
        // 显示时间戳
        let time = msg.timestamp.format("%H:%M:%S");
        stdout
            .execute(SetForegroundColor(self.theme.timestamp_color))
            .unwrap();
        print!("[{}] ", time);
        
        match &msg.content {
            MessageType::Text(text) => {
                let sender = msg.from_nick.as_deref().unwrap_or("匿名用户");
                
                // 检查是否是机器人消息
                if sender.contains("Bot") || sender.contains("机器人") {
                    stdout
                        .execute(SetForegroundColor(self.theme.bot_color))
                        .unwrap();
                    print!("{}: ", sender);
                } else {
                    let username_color = self.get_username_color(sender);
                    stdout
                        .execute(SetForegroundColor(username_color))
                        .unwrap();
                    print!("{}: ", sender);
                }
                
                // 显示消息内容
                stdout
                    .execute(SetForegroundColor(self.theme.text_color))
                    .unwrap();
                println!("{}", text);
            }
            MessageType::System(text) => {
                stdout
                    .execute(SetForegroundColor(self.theme.system_color))
                    .unwrap();
                println!("[系统]: {}", text);
            }
            MessageType::NickChange { old_nick, new_nick } => {
                stdout
                    .execute(SetForegroundColor(self.theme.system_color))
                    .unwrap();
                println!("[系统]: {} 将昵称改为 {}", old_nick, new_nick);
            }
        }
        
        // 重置颜色
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示成功消息
    pub fn display_success(&self, message: &str) {
        let mut stdout = io::stdout();
        stdout
            .execute(SetForegroundColor(self.theme.success_color))
            .unwrap();
        println!("✅ {}", message);
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示错误消息
    pub fn display_error(&self, message: &str) {
        let mut stdout = io::stdout();
        stdout
            .execute(SetForegroundColor(self.theme.error_color))
            .unwrap();
        println!("❌ {}", message);
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示信息消息
    pub fn display_info(&self, message: &str) {
        let mut stdout = io::stdout();
        stdout
            .execute(SetForegroundColor(self.theme.info_color))
            .unwrap();
        println!("💡 {}", message);
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示连接状态
    pub fn display_connection_status(&self, connected: bool) {
        let mut stdout = io::stdout();
        if connected {
            stdout
                .execute(SetForegroundColor(self.theme.success_color))
                .unwrap();
            println!("🔗 已连接到服务器");
        } else {
            stdout
                .execute(SetForegroundColor(self.theme.error_color))
                .unwrap();
            println!("🔌 与服务器断开连接");
        }
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示欢迎消息
    pub fn display_welcome(&self) {
        self.clear_screen();
        
        let mut stdout = io::stdout();
        
        // 显示Banner
        stdout
            .execute(SetForegroundColor(Color::Cyan))
            .unwrap();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║                    🚀 RustChat CLI v0.1.0                ║");
        println!("║              现代化Rust聊天应用 - 终端客户端              ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        
        stdout
            .execute(SetForegroundColor(self.theme.info_color))
            .unwrap();
        println!("💬 开始你的聊天之旅！输入 /help 查看可用命令");
        println!("🎨 支持彩色显示：昵称会显示不同颜色，机器人消息为绿色");
        
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 清空屏幕
    pub fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
    }

    /// 显示输入提示符
    pub fn display_prompt(&self) {
        let mut stdout = io::stdout();
        stdout
            .execute(SetForegroundColor(Color::DarkGreen))
            .unwrap();
        print!("> ");
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示历史消息分隔符
    pub fn display_history_separator(&self, count: usize) {
        let mut stdout = io::stdout();
        stdout
            .execute(SetForegroundColor(self.theme.info_color))
            .unwrap();
        println!("┌─────────────────────────────────────────────────────────┐");
        println!("│              📚 最近 {} 条消息历史              │", count);
        println!("└─────────────────────────────────────────────────────────┘");
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// 显示分隔线
    pub fn display_separator(&self) {
        let mut stdout = io::stdout();
        stdout
            .execute(SetForegroundColor(Color::DarkGrey))
            .unwrap();
        println!("─────────────────────────────────────────────────────────────");
        stdout.execute(ResetColor).unwrap();
        stdout.flush().unwrap();
    }
}

impl Default for ColorDisplay {
    fn default() -> Self {
        Self::new()
    }
}
