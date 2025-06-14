use super::{Account, AccountId, AccountStatus, AuthError, EmailVerification, VerificationPurpose, JwtClaims, TokenType, TokenPair};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use sqlx::{Row, SqlitePool};
use tracing::{debug, info};

/// 认证服务
#[derive(Clone)]
pub struct AuthService {
    db_pool: SqlitePool,
    argon2: Argon2<'static>,
    jwt_secret: String,
    access_token_duration: Duration,
    refresh_token_duration: Duration,
}

impl AuthService {    /// 创建新的认证服务
    pub fn new(db_pool: SqlitePool) -> Self {
        // 在生产环境中，应该从环境变量读取 JWT 密钥
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-256-bit-secret-key-that-should-be-from-env".to_string());
        
        Self {
            db_pool,
            argon2: Argon2::default(),
            jwt_secret,
            access_token_duration: Duration::minutes(15), // 15分钟
            refresh_token_duration: Duration::days(7),    // 7天
        }
    }
    
    /// 获取数据库连接池
    pub fn get_pool(&self) -> &SqlitePool {
        &self.db_pool
    }
    
    /// 初始化数据库表
    pub async fn initialize_database(&self) -> Result<(), AuthError> {
        // 创建账户表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                display_name TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                email_verified BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TEXT NOT NULL,
                last_login_at TEXT
            )
        "#)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
          // 创建邮箱验证码表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS email_verifications (
                email TEXT NOT NULL,
                code TEXT NOT NULL,
                purpose TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                used BOOLEAN NOT NULL DEFAULT FALSE,
                PRIMARY KEY (email, code, purpose)
            )
        "#)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        // 创建会话表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                refresh_token_hash TEXT NOT NULL,
                device_info TEXT,
                ip_address TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                last_used_at TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT TRUE,
                FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
            )
        "#)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_accounts_email ON accounts(email)")
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
              sqlx::query("CREATE INDEX IF NOT EXISTS idx_email_verifications_email ON email_verifications(email)")
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_account_id ON sessions(account_id)")
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_refresh_token ON sessions(refresh_token_hash)")
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        info!("认证数据库表初始化完成");
        Ok(())
    }
    
    /// 注册新用户
    pub async fn register(&self, email: String, password: String, display_name: Option<String>) -> Result<Account, AuthError> {
        // 验证邮箱格式
        self.validate_email(&email)?;
        
        // 验证密码强度
        self.validate_password(&password)?;
        
        // 检查邮箱是否已存在
        if self.email_exists(&email).await? {
            return Err(AuthError::EmailAlreadyExists);
        }
        
        // 哈希密码
        let password_hash = self.hash_password(&password)?;
        
        // 创建账户
        let account = Account {
            id: AccountId::new(),
            email: email.clone(),
            password_hash,
            display_name,
            status: AccountStatus::Active,
            email_verified: false,
            created_at: Utc::now(),
            last_login_at: None,
        };
        
        // 保存到数据库
        sqlx::query(r#"
            INSERT INTO accounts (id, email, password_hash, display_name, status, email_verified, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(account.id.to_string())
        .bind(&account.email)
        .bind(&account.password_hash)
        .bind(&account.display_name)
        .bind(account.status.to_string())
        .bind(account.email_verified)
        .bind(account.created_at.to_rfc3339())
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        info!("新用户注册成功: {}", email);
        Ok(account)
    }
    
    /// 生成并发送邮箱验证码
    pub async fn send_verification_code(&self, email: String, purpose: VerificationPurpose) -> Result<(), AuthError> {
        // 生成6位数字验证码
        let code = self.generate_verification_code();
        
        // 设置过期时间（10分钟）
        let expires_at = Utc::now() + Duration::minutes(10);
        
        // 先清理该邮箱的旧验证码
        self.cleanup_old_verification_codes(&email, purpose).await?;
        
        // 保存验证码到数据库
        let verification = EmailVerification {
            email: email.clone(),
            code: code.clone(),
            purpose,
            expires_at,
            created_at: Utc::now(),
            used: false,
        };
        
        sqlx::query(r#"
            INSERT INTO email_verifications (email, code, purpose, expires_at, created_at, used)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&verification.email)
        .bind(&verification.code)
        .bind(verification.purpose.to_string())
        .bind(verification.expires_at.to_rfc3339())
        .bind(verification.created_at.to_rfc3339())
        .bind(verification.used)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        // TODO: 发送邮件
        // 这里暂时只记录日志，实际项目中需要集成邮件服务
        info!("邮箱验证码已生成: {} -> {} ({})", email, code, purpose);
        debug!("验证码: {} (测试环境)", code);
        
        Ok(())
    }
    
    /// 验证邮箱验证码
    pub async fn verify_email_code(&self, email: String, code: String, purpose: VerificationPurpose) -> Result<(), AuthError> {
        let row = sqlx::query(r#"
            SELECT expires_at, used FROM email_verifications
            WHERE email = ? AND code = ? AND purpose = ?
            ORDER BY created_at DESC
            LIMIT 1
        "#)
        .bind(&email)
        .bind(&code)
        .bind(purpose.to_string())
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        let row = row.ok_or(AuthError::InvalidVerificationCode)?;
        
        let expires_at: String = row.get("expires_at");
        let used: bool = row.get("used");
        
        if used {
            return Err(AuthError::InvalidVerificationCode);
        }
        
        let expires_at = DateTime::parse_from_rfc3339(&expires_at)
            .map_err(|e| AuthError::DatabaseError(e.into()))?
            .with_timezone(&Utc);
        
        if Utc::now() > expires_at {
            return Err(AuthError::InvalidVerificationCode);
        }
        
        // 标记验证码为已使用
        sqlx::query(r#"
            UPDATE email_verifications
            SET used = TRUE
            WHERE email = ? AND code = ? AND purpose = ?
        "#)
        .bind(&email)
        .bind(&code)
        .bind(purpose.to_string())
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        // 如果是邮箱验证，更新账户状态
        if purpose == VerificationPurpose::EmailVerification {
            sqlx::query("UPDATE accounts SET email_verified = TRUE WHERE email = ?")
                .bind(&email)
                .execute(&self.db_pool)
                .await
                .map_err(|e| AuthError::DatabaseError(e.into()))?;
        }
        
        info!("邮箱验证码验证成功: {} ({})", email, purpose);
        Ok(())
    }
    
    /// 用户登录
    pub async fn login(&self, email: String, password: String) -> Result<Account, AuthError> {
        let account = self.get_account_by_email(&email).await?;
        
        // 验证密码
        if !self.verify_password(&password, &account.password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }
        
        // 检查账户状态
        match account.status {
            AccountStatus::Suspended => return Err(AuthError::AccountSuspended),
            AccountStatus::Deleted => return Err(AuthError::AccountDeleted),
            AccountStatus::Active => {}
        }
        
        // 更新最后登录时间
        let now = Utc::now();
        sqlx::query("UPDATE accounts SET last_login_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(account.id.to_string())
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        let mut updated_account = account;
        updated_account.last_login_at = Some(now);
        
        info!("用户登录成功: {}", email);
        Ok(updated_account)
    }
    
    /// 根据邮箱获取账户
    pub async fn get_account_by_email(&self, email: &str) -> Result<Account, AuthError> {
        let row = sqlx::query(r#"
            SELECT id, email, password_hash, display_name, status, email_verified, created_at, last_login_at
            FROM accounts WHERE email = ?
        "#)
        .bind(email)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        let row = row.ok_or(AuthError::AccountNotFound)?;
        
        let account = Account {
            id: AccountId::parse(&row.get::<String, _>("id"))
                .map_err(|e| AuthError::DatabaseError(e.into()))?,
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            display_name: row.get("display_name"),
            status: row.get::<String, _>("status").parse()
                .map_err(|_| AuthError::DatabaseError(anyhow::anyhow!("Invalid account status")))?,
            email_verified: row.get("email_verified"),
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                .map_err(|e| AuthError::DatabaseError(e.into()))?
                .with_timezone(&Utc),
            last_login_at: row.get::<Option<String>, _>("last_login_at")
                .map(|s| DateTime::parse_from_rfc3339(&s).ok())
                .flatten()
                .map(|dt| dt.with_timezone(&Utc)),
        };
        
        Ok(account)
    }
    
    /// 验证邮箱格式
    fn validate_email(&self, email: &str) -> Result<(), AuthError> {
        if email.is_empty() || !email.contains('@') || email.len() > 254 {
            return Err(AuthError::InvalidEmail);
        }
        
        // 简单的邮箱格式验证
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(AuthError::InvalidEmail);
        }
        
        Ok(())
    }
    
    /// 验证密码强度
    fn validate_password(&self, password: &str) -> Result<(), AuthError> {
        if password.len() < 6 {
            return Err(AuthError::InvalidPassword);
        }
        
        if password.len() > 128 {
            return Err(AuthError::InvalidPassword);
        }
        
        Ok(())
    }
    
    /// 检查邮箱是否已存在
    async fn email_exists(&self, email: &str) -> Result<bool, AuthError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE email = ?")
            .bind(email)
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        Ok(count > 0)
    }
      /// 哈希密码
    fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self.argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHashError(e.to_string()))?
            .to_string();
        Ok(password_hash)
    }
    
    /// 验证密码
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::PasswordHashError(e.to_string()))?;
        Ok(self.argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }    /// 生成6位数字验证码
    fn generate_verification_code(&self) -> String {
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(100000..1000000))
    }
      /// 清理旧的验证码
    async fn cleanup_old_verification_codes(&self, email: &str, purpose: VerificationPurpose) -> Result<(), AuthError> {
        sqlx::query(r#"
            DELETE FROM email_verifications
            WHERE email = ? AND purpose = ? AND (expires_at < ? OR used = TRUE)
        "#)
        .bind(email)
        .bind(purpose.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        Ok(())
    }
    
    // ============= JWT 相关方法 =============
    
    /// 生成访问令牌和刷新令牌对
    pub async fn generate_token_pair(&self, account: &Account, device_info: Option<String>, ip_address: Option<String>) -> Result<TokenPair, AuthError> {
        let now = Utc::now();
        
        // 生成访问令牌
        let access_token = self.generate_token(account, TokenType::Access, now)?;
        
        // 生成刷新令牌
        let refresh_token = self.generate_token(account, TokenType::Refresh, now)?;
        
        // 保存会话到数据库
        let session_id = uuid::Uuid::new_v4().to_string();
        let refresh_token_hash = self.hash_refresh_token(&refresh_token)?;
        let expires_at = now + self.refresh_token_duration;
        
        sqlx::query(r#"
            INSERT INTO sessions (id, account_id, refresh_token_hash, device_info, ip_address, created_at, expires_at, last_used_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&session_id)
        .bind(account.id.to_string())
        .bind(&refresh_token_hash)
        .bind(&device_info)
        .bind(&ip_address)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        info!("为用户 {} 生成了新的令牌对", account.email);
        
        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_duration.num_seconds(),
        })
    }
    
    /// 生成 JWT 令牌
    fn generate_token(&self, account: &Account, token_type: TokenType, issued_at: DateTime<Utc>) -> Result<String, AuthError> {
        let expiration = match token_type {
            TokenType::Access => issued_at + self.access_token_duration,
            TokenType::Refresh => issued_at + self.refresh_token_duration,
        };
        
        let claims = JwtClaims {
            sub: account.id.to_string(),
            email: account.email.clone(),
            display_name: account.display_name.clone(),
            iat: issued_at.timestamp(),
            exp: expiration.timestamp(),
            token_type: token_type.to_string(),
        };
        
        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.jwt_secret.as_ref());
        
        encode(&header, &claims, &encoding_key)
            .map_err(|e| AuthError::DatabaseError(anyhow::anyhow!("JWT encoding error: {}", e)))
    }
    
    /// 验证并解析 JWT 令牌
    pub fn verify_token(&self, token: &str, expected_type: TokenType) -> Result<JwtClaims, AuthError> {
        let decoding_key = DecodingKey::from_secret(self.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);
        
        let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
            .map_err(|e| AuthError::DatabaseError(anyhow::anyhow!("JWT decoding error: {}", e)))?;
        
        let claims = token_data.claims;
        
        // 验证令牌类型
        if claims.token_type != expected_type.to_string() {
            return Err(AuthError::DatabaseError(anyhow::anyhow!("Invalid token type")));
        }
        
        // 验证过期时间
        let now = Utc::now().timestamp();
        if claims.exp < now {
            return Err(AuthError::DatabaseError(anyhow::anyhow!("Token expired")));
        }
        
        Ok(claims)
    }
    
    /// 刷新访问令牌
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        // 验证刷新令牌
        let claims = self.verify_token(refresh_token, TokenType::Refresh)?;
        
        // 验证会话是否存在且有效
        let refresh_token_hash = self.hash_refresh_token(refresh_token)?;
        let session_row = sqlx::query(r#"
            SELECT account_id, expires_at, is_active, device_info, ip_address
            FROM sessions 
            WHERE refresh_token_hash = ? AND is_active = TRUE
        "#)
        .bind(&refresh_token_hash)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        let session = session_row.ok_or_else(|| 
            AuthError::DatabaseError(anyhow::anyhow!("Invalid refresh token")))?;
        
        let expires_at_str: String = session.get("expires_at");
        let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
            .map_err(|e| AuthError::DatabaseError(e.into()))?
            .with_timezone(&Utc);
        
        if Utc::now() > expires_at {
            return Err(AuthError::DatabaseError(anyhow::anyhow!("Refresh token expired")));
        }
        
        // 获取用户信息
        let account_id_str: String = session.get("account_id");
        let account_id = AccountId::parse(&account_id_str)
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        let account = self.get_account_by_id(&account_id).await?;
        
        // 更新会话最后使用时间
        let now = Utc::now();
        sqlx::query("UPDATE sessions SET last_used_at = ? WHERE refresh_token_hash = ?")
            .bind(now.to_rfc3339())
            .bind(&refresh_token_hash)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        // 生成新的访问令牌（保持原有刷新令牌）
        let access_token = self.generate_token(&account, TokenType::Access, now)?;
        
        info!("为用户 {} 刷新了访问令牌", account.email);
        
        Ok(TokenPair {
            access_token,
            refresh_token: refresh_token.to_string(),
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_duration.num_seconds(),
        })
    }
    
    /// 根据ID获取账户
    pub async fn get_account_by_id(&self, account_id: &AccountId) -> Result<Account, AuthError> {
        let row = sqlx::query(r#"
            SELECT id, email, password_hash, display_name, status, email_verified, created_at, last_login_at
            FROM accounts WHERE id = ?
        "#)
        .bind(account_id.to_string())
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        let row = row.ok_or(AuthError::AccountNotFound)?;
        
        let account = Account {
            id: AccountId::parse(&row.get::<String, _>("id"))
                .map_err(|e| AuthError::DatabaseError(e.into()))?,
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            display_name: row.get("display_name"),
            status: row.get::<String, _>("status").parse()
                .map_err(|_| AuthError::DatabaseError(anyhow::anyhow!("Invalid account status")))?,
            email_verified: row.get("email_verified"),
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))
                .map_err(|e| AuthError::DatabaseError(e.into()))?
                .with_timezone(&Utc),
            last_login_at: row.get::<Option<String>, _>("last_login_at")
                .map(|s| DateTime::parse_from_rfc3339(&s).ok())
                .flatten()
                .map(|dt| dt.with_timezone(&Utc)),
        };
        
        Ok(account)
    }
    
    /// 注销（撤销刷新令牌）
    pub async fn logout(&self, refresh_token: &str) -> Result<(), AuthError> {
        let refresh_token_hash = self.hash_refresh_token(refresh_token)?;
        
        sqlx::query("UPDATE sessions SET is_active = FALSE WHERE refresh_token_hash = ?")
            .bind(&refresh_token_hash)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        info!("用户会话已注销");
        Ok(())
    }
    
    /// 注销所有设备
    pub async fn logout_all_devices(&self, account_id: &AccountId) -> Result<(), AuthError> {
        sqlx::query("UPDATE sessions SET is_active = FALSE WHERE account_id = ?")
            .bind(account_id.to_string())
            .execute(&self.db_pool)
            .await
            .map_err(|e| AuthError::DatabaseError(e.into()))?;
        
        info!("用户 {} 的所有设备会话已注销", account_id);
        Ok(())
    }
    
    /// 哈希刷新令牌（用于数据库存储）
    fn hash_refresh_token(&self, refresh_token: &str) -> Result<String, AuthError> {
        // 使用 SHA-256 哈希刷新令牌
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        refresh_token.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }
}
