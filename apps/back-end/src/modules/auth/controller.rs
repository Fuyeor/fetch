// src/modules/auth/controller.rs
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use fplatform_oauth::{OAuthClient, create_jwt, verify_jwt};
use sea_orm::{DatabaseConnection, EntityTrait};
use time::Duration;

use crate::{
    AppState,
    config::AppConfig,
    entities::prelude::User,
    modules::auth::dto::{AuthResponseDto, OAuthCallbackDto, UserInfoDto},
    modules::auth::service,
};

/// 交换 OAuth 授权码并写入 3 Cookie 会话
pub async fn oauth_callback(
    State(db): State<DatabaseConnection>,
    State(oauth): State<OAuthClient>,
    State(config): State<AppConfig>,
    jar: CookieJar,
    Json(payload): Json<OAuthCallbackDto>,
) -> Result<(CookieJar, Json<AuthResponseDto>), (StatusCode, String)> {
    let token_resp = oauth
        .exchange_code(&payload.code, payload.state.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                format!("OAuth exchange failed: {}", e),
            )
        })?;

    let oauth_user = token_resp.user;

    service::sync_oauth_user(&db, &oauth_user).await?;

    // 签发 30 分钟短期 access_token
    let token = create_jwt(
        &oauth_user.id,
        &oauth_user.username,
        chrono::Duration::minutes(30),
        &config.jwt_key,
        &config.jwt_issuer,
        &config.jwt_audience,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to issue Access JWT: {}", e),
        )
    })?;

    // 签发 7 天长期 refresh_token
    let refresh_token = create_jwt(
        &oauth_user.id,
        &oauth_user.username,
        chrono::Duration::days(7),
        &config.jwt_key,
        &config.jwt_issuer,
        &config.jwt_audience,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to issue Refresh JWT: {}", e),
        )
    })?;

    let response = AuthResponseDto {
        token: token.clone(),
        user: UserInfoDto {
            id: oauth_user.id,
            username: oauth_user.username,
            nickname: oauth_user.nickname,
            avatar: oauth_user.avatar,
        },
    };

    // 注入 3-Cookie 会话
    let updated_jar = jar
        .add(make_cookie(
            "access_token",
            token,
            Duration::minutes(30),
            true,
        ))
        .add(make_cookie(
            "refresh_token",
            refresh_token,
            Duration::days(7),
            true,
        ))
        .add(make_cookie(
            "session_payload",
            "true".to_string(),
            Duration::days(7),
            false,
        ));

    Ok((updated_jar, Json(response)))
}

/// 刷新 access_token
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), (StatusCode, String)> {
    let refresh_token = match jar.get("refresh_token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Missing refresh token.".to_string(),
            ));
        }
    };

    let claims = match verify_jwt(
        &refresh_token,
        &state.config.jwt_key,
        &state.config.jwt_issuer,
        &state.config.jwt_audience,
    ) {
        Ok(claims) => claims,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid refresh token.".to_string(),
            ));
        }
    };

    let new_access_token = create_jwt(
        &claims.sub,
        &claims.username,
        chrono::Duration::minutes(30),
        &state.config.jwt_key,
        &state.config.jwt_issuer,
        &state.config.jwt_audience,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated_jar = jar.add(make_cookie(
        "access_token",
        new_access_token,
        Duration::minutes(30),
        true,
    ));

    Ok((updated_jar, StatusCode::OK))
}

/// 获取当前登录用户信息
pub async fn get_me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<UserInfoDto>, (StatusCode, String)> {
    let token = match jar.get("access_token") {
        Some(cookie) => cookie.value().to_string(),
        None => return Err(unauthorized()),
    };

    let claims = match verify_jwt(
        &token,
        &state.config.jwt_key,
        &state.config.jwt_issuer,
        &state.config.jwt_audience,
    ) {
        Ok(claims) => claims,
        Err(_) => return Err(unauthorized()),
    };

    let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| unauthorized())?;

    let user = User::find_by_id(user_id)
        .one(&state.db)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or_else(unauthorized)?;

    Ok(Json(UserInfoDto {
        id: user.id.to_string(),
        username: user.username,
        nickname: user.nickname,
        avatar: user.avatar,
    }))
}

fn unauthorized() -> (StatusCode, String) {
    (StatusCode::UNAUTHORIZED, "Unauthorized.".to_string())
}

fn make_cookie(
    name: &'static str,
    value: String,
    age: Duration,
    http_only: bool,
) -> Cookie<'static> {
    let secure = !cfg!(debug_assertions);

    Cookie::build((name, value))
        .http_only(http_only)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(age)
        .build()
}
