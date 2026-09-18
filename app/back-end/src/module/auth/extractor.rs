// src/module/auth/extractor.rs
use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::CookieJar;
use fplatform_oauth::verify_jwt;
use std::ops::Deref;
use uuid::Uuid;

use crate::config::AppConfig;

/// 当前登录用户的强类型上下文
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
}

impl Deref for CurrentUser {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    AppConfig: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let config = AppConfig::from_ref(state);

        // 从 HttpOnly Cookie 中提取 access_token
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to parse cookie jar",
                )
            })?;

        let token = jar
            .get("access_token")
            .map(|c| c.value())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing access token."))?;

        // 校验 JWT 签名与过期时间
        let claims = verify_jwt(
            token,
            &config.jwt_key,
            &config.jwt_issuer,
            &config.jwt_audience,
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token."))?;

        // 校验 UUID 格式
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid user identifier format."))?;

        // 这里直接返回构建好的当前用户
        Ok(CurrentUser {
            id: user_id,
            username: claims.username,
        })
    }
}
