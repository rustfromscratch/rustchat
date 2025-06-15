pub mod api;
pub mod service;
pub mod middleware;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// 重新导出主要类型和函数
pub use api::create_auth_routes;
pub use service::AuthService;
pub use middleware::{auth_middleware, optional_auth_middleware, AuthenticatedUser};

/// 用户账户ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Uuid);

impl AccountId {
    /// 生成新的账户ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// 从字符串解析账户ID
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
    
    /// 转换为字符串
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::new()
    }
}

/// 用户账户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// 账户ID
    pub id: AccountId,
    /// 邮箱地址
    pub email: String,
    /// 密码哈希
    pub password_hash: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 账户状态
    pub status: AccountStatus,
    /// 邮箱验证状态
    pub email_verified: bool,
    /// 注册时间
    pub created_at: DateTime<Utc>,
    /// 最后登录时间
    pub last_login_at: Option<DateTime<Utc>>,
}

/// 账户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    /// 活跃状态
    Active,
    /// 暂停状态
    Suspended,
    /// 删除状态
    Deleted,
}

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountStatus::Active => write!(f, "active"),
            AccountStatus::Suspended => write!(f, "suspended"),
            AccountStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl std::str::FromStr for AccountStatus {
    type Err = &'static str;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(AccountStatus::Active),
            "suspended" => Ok(AccountStatus::Suspended),
            "deleted" => Ok(AccountStatus::Deleted),
            _ => Err("Invalid account status"),
        }
    }
}

/// 邮箱验证码
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerification {
    /// 邮箱地址
    pub email: String,
    /// 验证码
    pub code: String,
    /// 验证码类型
    pub purpose: VerificationPurpose,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 是否已使用
    pub used: bool,
}

/// 验证码用途
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationPurpose {
    /// 邮箱验证
    EmailVerification,
    /// 密码重置
    PasswordReset,
    /// 登录验证
    LoginVerification,
}

impl std::fmt::Display for VerificationPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationPurpose::EmailVerification => write!(f, "email_verification"),
            VerificationPurpose::PasswordReset => write!(f, "password_reset"),
            VerificationPurpose::LoginVerification => write!(f, "login_verification"),
        }
    }
}

impl std::str::FromStr for VerificationPurpose {
    type Err = &'static str;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email_verification" => Ok(VerificationPurpose::EmailVerification),
            "password_reset" => Ok(VerificationPurpose::PasswordReset),
            "login_verification" => Ok(VerificationPurpose::LoginVerification),
            _ => Err("Invalid verification purpose"),
        }    }
}

/// 认证相关错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("邮箱地址无效")]
    InvalidEmail,
    #[error("密码不符合要求")]
    InvalidPassword,
    #[error("邮箱已被注册")]
    EmailAlreadyExists,
    #[error("账户不存在")]
    AccountNotFound,
    #[error("密码错误")]
    InvalidCredentials,
    #[error("验证码无效或已过期")]
    InvalidVerificationCode,
    #[error("账户未验证")]
    AccountNotVerified,
    #[error("账户已被暂停")]
    AccountSuspended,
    #[error("账户已被删除")]
    AccountDeleted,
    #[error("验证码发送失败")]
    VerificationSendFailed,
    #[error("令牌已过期")]
    TokenExpired,
    #[error("令牌无效")]
    InvalidToken,
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] anyhow::Error),
    #[error("密码哈希错误: {0}")]
    PasswordHashError(String),
    #[error("邮件发送错误: {0}")]
    EmailSendError(#[from] lettre::error::Error),
}

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// 邮箱验证请求
#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub code: String,
}

/// 重发验证码请求
#[derive(Debug, Deserialize)]
pub struct ResendCodeRequest {
    pub email: String,
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /// 用户ID
    pub sub: String, // subject (用户ID)
    /// 邮箱
    pub email: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 令牌颁发时间
    pub iat: i64, // issued at
    /// 令牌过期时间
    pub exp: i64, // expiration time
    /// 令牌类型 (access/refresh)
    pub token_type: String,
}

/// 令牌类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Access,
    Refresh,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::Access => write!(f, "access"),
            TokenType::Refresh => write!(f, "refresh"),
        }
    }
}

/// 令牌对
#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64, // access token 过期时间（秒）
}

/// 刷新令牌请求
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// 认证响应
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub account_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub tokens: Option<TokenPair>,
}

impl AuthResponse {
    pub fn from_account(account: &Account) -> Self {
        Self {
            account_id: account.id.to_string(),
            email: account.email.clone(),
            display_name: account.display_name.clone(),
            email_verified: account.email_verified,
            created_at: account.created_at,
            tokens: None,
        }
    }
    
    pub fn from_account_with_tokens(account: &Account, tokens: TokenPair) -> Self {
        Self {
            account_id: account.id.to_string(),
            email: account.email.clone(),
            display_name: account.display_name.clone(),
            email_verified: account.email_verified,
            created_at: account.created_at,
            tokens: Some(tokens),
        }
    }
}
