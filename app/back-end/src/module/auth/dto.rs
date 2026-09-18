// src/module/auth/dto.rs
use serde::{Deserialize, Serialize};

/// Request payload for OAuth authorization code exchange.
#[derive(Deserialize)]
pub struct OAuthCallbackDto {
    pub code: String,
    pub state: Option<String>,
}

/// Authentication response containing JWT and user profile.
#[derive(Serialize)]
pub struct AuthResponseDto {
    pub token: String,
    pub user: UserInfoDto,
}

/// Sanitized user profile object.
#[derive(Serialize)]
pub struct UserInfoDto {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub avatar: Option<String>,
}
