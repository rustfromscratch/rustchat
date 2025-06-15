use super::{
    AuthError, AuthResponse, LoginRequest, RegisterRequest, 
    ResendCodeRequest, VerificationPurpose, VerifyEmailRequest, RefreshTokenRequest
};
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::json;
use tracing::{error, info, warn};

/// 创建认证相关的路由
pub fn create_auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/verify-email", post(verify_email))
        .route("/api/auth/resend-code", post(resend_verification_code))
        .route("/api/auth/refresh", post(refresh_token))
        .route("/api/auth/me", get(get_current_user))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/health", get(auth_health_check))
}

/// 用户注册
async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> impl IntoResponse {
    info!("收到注册请求: email={}", request.email);

    match state.auth_service.register(
        request.email.clone(),
        request.password,
        request.display_name,
    ).await {
        Ok(account) => {
            info!("用户注册成功: {}", account.email);
            
            // 自动发送邮箱验证码
            if let Err(e) = state.auth_service.send_verification_code(
                account.email.clone(),
                VerificationPurpose::EmailVerification,
            ).await {
                error!("发送邮箱验证码失败: {}", e);
                // 注册成功但验证码发送失败，返回警告
                return (
                    StatusCode::CREATED,
                    Json(json!({
                        "success": true,
                        "message": "注册成功，但邮箱验证码发送失败，请稍后重试",
                        "account": AuthResponse::from_account(&account),
                        "warning": "邮箱验证码发送失败"
                    }))
                );
            }
            
            (
                StatusCode::CREATED,
                Json(json!({
                    "success": true,
                    "message": "注册成功，邮箱验证码已发送",
                    "account": AuthResponse::from_account(&account)
                }))
            )
        }
        Err(e) => {
            warn!("用户注册失败: {} - {}", request.email, e);
            handle_auth_error(e)
        }
    }
}

/// 用户登录
async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    info!("收到登录请求: email={}", request.email);

    match state.auth_service.login(request.email.clone(), request.password).await {
        Ok(account) => {
            info!("用户登录成功: {}", account.email);
            
            // 生成 JWT 令牌对
            match state.auth_service.generate_token_pair(&account, None, None).await {
                Ok(tokens) => {
                    (
                        StatusCode::OK,
                        Json(json!({
                            "success": true,
                            "message": "登录成功",
                            "account": AuthResponse::from_account_with_tokens(&account, tokens)
                        }))
                    )
                }
                Err(e) => {
                    error!("生成令牌失败: {} - {}", request.email, e);
                    handle_auth_error(e)
                }
            }
        }
        Err(e) => {
            warn!("用户登录失败: {} - {}", request.email, e);
            handle_auth_error(e)
        }
    }
}

/// 验证邮箱
async fn verify_email(
    State(state): State<AppState>,
    Json(request): Json<VerifyEmailRequest>,
) -> impl IntoResponse {
    info!("收到邮箱验证请求: email={}", request.email);

    match state.auth_service.verify_email_code(
        request.email.clone(),
        request.code,
        VerificationPurpose::EmailVerification,
    ).await {
        Ok(()) => {
            info!("邮箱验证成功: {}", request.email);
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "邮箱验证成功"
                }))
            )
        }
        Err(e) => {
            warn!("邮箱验证失败: {} - {}", request.email, e);
            handle_auth_error(e)
        }
    }
}

/// 重新发送验证码
async fn resend_verification_code(
    State(state): State<AppState>,
    Json(request): Json<ResendCodeRequest>,
) -> impl IntoResponse {
    info!("收到重发验证码请求: email={}", request.email);

    // 检查邮箱是否已注册
    match state.auth_service.get_account_by_email(&request.email).await {
        Ok(account) => {
            if account.email_verified {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "success": false,
                        "message": "邮箱已验证，无需重复验证"
                    }))
                );
            }
            
            // 发送验证码
            match state.auth_service.send_verification_code(
                request.email.clone(),
                VerificationPurpose::EmailVerification,
            ).await {
                Ok(()) => {
                    info!("重发验证码成功: {}", request.email);
                    (
                        StatusCode::OK,
                        Json(json!({
                            "success": true,
                            "message": "验证码已重新发送"
                        }))
                    )
                }
                Err(e) => {
                    error!("重发验证码失败: {} - {}", request.email, e);
                    handle_auth_error(e)
                }
            }
        }
        Err(AuthError::AccountNotFound) => {
            // 为了安全，不暴露邮箱不存在的信息
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "如果邮箱已注册，验证码将被发送"
                }))
            )
        }
        Err(e) => {
            error!("查询账户失败: {} - {}", request.email, e);
            handle_auth_error(e)
        }
    }
}

/// 认证健康检查
async fn auth_health_check(State(state): State<AppState>) -> impl IntoResponse {
    // 测试数据库连接
    match sqlx::query("SELECT 1").execute(state.auth_service.get_pool()).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "healthy",
                "service": "auth",
                "database": "connected"
            }))
        ),
        Err(e) => {
            error!("认证服务数据库连接失败: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "unhealthy",
                    "service": "auth",
                    "database": "disconnected",
                    "error": e.to_string()
                }))
            )
        }
    }
}

/// 刷新访问令牌
async fn refresh_token(
    State(state): State<AppState>,
    Json(request): Json<RefreshTokenRequest>,
) -> impl IntoResponse {
    info!("收到刷新令牌请求");

    match state.auth_service.refresh_access_token(&request.refresh_token).await {
        Ok(tokens) => {
            info!("刷新令牌成功");
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "令牌刷新成功",
                    "tokens": tokens
                }))
            )
        }
        Err(e) => {
            warn!("刷新令牌失败: {}", e);
            handle_auth_error(e)
        }
    }
}

/// 获取当前用户信息
async fn get_current_user(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 暂时返回一个简单的响应，等待添加JWT中间件
    info!("收到获取当前用户请求");
    
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "success": false,
            "message": "JWT认证中间件尚未实现"
        }))
    )
}

/// 用户登出
async fn logout(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 暂时返回一个简单的响应
    info!("收到登出请求");
    
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "登出成功"
        }))
    )
}

/// 处理认证错误，转换为HTTP响应
fn handle_auth_error(error: AuthError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, message) = match error {
        AuthError::InvalidEmail => (StatusCode::BAD_REQUEST, "邮箱地址格式无效"),
        AuthError::InvalidPassword => (StatusCode::BAD_REQUEST, "密码不符合要求（至少6位）"),
        AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, "邮箱已被注册"),
        AuthError::AccountNotFound => (StatusCode::NOT_FOUND, "账户不存在"),
        AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "邮箱或密码错误"),
        AuthError::InvalidVerificationCode => (StatusCode::BAD_REQUEST, "验证码无效或已过期"),
        AuthError::AccountNotVerified => (StatusCode::FORBIDDEN, "账户邮箱未验证"),
        AuthError::AccountSuspended => (StatusCode::FORBIDDEN, "账户已被暂停"),
        AuthError::AccountDeleted => (StatusCode::FORBIDDEN, "账户已被删除"),
        AuthError::VerificationSendFailed => (StatusCode::SERVICE_UNAVAILABLE, "验证码发送失败"),
        AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "令牌已过期"),
        AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "令牌无效"),
        AuthError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误"),
        AuthError::PasswordHashError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "密码处理错误"),
        AuthError::EmailSendError(_) => (StatusCode::SERVICE_UNAVAILABLE, "邮件发送失败"),
    };

    (
        status,
        Json(json!({
            "success": false,
            "message": message,
            "error_type": format!("{:?}", error)
        }))
    )
}
