use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
    Extension,
};
use rustchat_types::UserId;

use crate::{AppState, auth::{AuthError, TokenType}};

/// 用户认证信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub account_id: String,
    pub email: String,
}

/// 认证中间件 - 验证JWT token并提取用户信息
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    tracing::info!("认证中间件: 验证请求 {}", request.uri());
    
    // 从Authorization header中提取token
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("认证中间件: 缺少 Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    if !auth_header.starts_with("Bearer ") {
        tracing::warn!("认证中间件: Authorization header 格式错误");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..]; // 移除 "Bearer " 前缀
    tracing::debug!("认证中间件: 验证token...");

    // 验证token并提取用户信息
    match state.auth_service.verify_token(token, TokenType::Access) {
        Ok(claims) => {
            tracing::debug!("认证中间件: token验证成功，用户: {}", claims.sub);
            
            // 从claims.sub解析AccountId
            let account_id = crate::auth::AccountId::parse(&claims.sub)
                .map_err(|e| {
                    tracing::error!("认证中间件: 解析账户ID失败: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            
            // 从数据库获取完整的用户信息
            match state.auth_service.get_account_by_id(&account_id).await {
                Ok(account) => {
                    let user_id = UserId::parse(&account.id.to_string())
                        .map_err(|e| {
                            tracing::error!("认证中间件: 解析用户ID失败: {}", e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                    
                    let auth_user = AuthenticatedUser {
                        user_id,
                        account_id: account.id.to_string(),
                        email: account.email.clone(),
                    };
                    
                    tracing::info!("认证中间件: 用户认证成功 {} ({})", auth_user.email, auth_user.user_id);
                    
                    // 将用户信息添加到请求扩展中
                    request.extensions_mut().insert(auth_user);
                    Ok(next.run(request).await)
                }
                Err(e) => {
                    tracing::error!("认证中间件: 获取账户信息失败: {}", e);
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
        Err(AuthError::TokenExpired) => {
            tracing::warn!("认证中间件: token已过期");
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            tracing::error!("认证中间件: token验证失败: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// 可选认证中间件 - 如果有token则验证，没有token也继续
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // 尝试从Authorization header中提取token
    if let Some(auth_header) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
    {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..]; // 移除 "Bearer " 前缀            // 验证token并提取用户信息
            if let Ok(claims) = state.auth_service.verify_token(token, TokenType::Access) {
                // 从claims.sub解析AccountId
                if let Ok(account_id) = crate::auth::AccountId::parse(&claims.sub) {
                    if let Ok(account) = state.auth_service.get_account_by_id(&account_id).await {
                        if let Ok(user_id) = UserId::parse(&account.id.to_string()) {
                            let auth_user = AuthenticatedUser {
                                user_id,
                                account_id: account.id.to_string(),
                                email: account.email,
                            };
                            
                            // 将用户信息添加到请求扩展中
                            request.extensions_mut().insert(auth_user);
                        }
                    }
                }
            }
        }
    }

    next.run(request).await
}
