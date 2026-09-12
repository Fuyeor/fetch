// src/modules/auth/service.rs
use crate::entities::prelude::User;
use crate::entities::user;
use axum::http::StatusCode;
use fplatform_oauth::OAuthUser;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

/// 同步主站 OAuth 用户到 Fetch 本地数据库
pub async fn sync_oauth_user(
    db: &DatabaseConnection,
    oauth_user: &OAuthUser,
) -> Result<(), (StatusCode, String)> {
    let user_uuid = uuid::Uuid::parse_str(&oauth_user.id).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid user UUID format: {}", e),
        )
    })?;

    let existing_user = User::find_by_id(user_uuid)
        .one(db)
        .await
        .map_err(internal_database_error)?;

    match existing_user {
        Some(usr) => {
            let mut active_user: user::ActiveModel = usr.into();
            active_user.username = Set(oauth_user.username.clone());
            active_user.nickname = Set(oauth_user.nickname.clone());
            active_user.avatar = Set(oauth_user.avatar.clone());
            active_user.updated_at = Set(chrono::Utc::now().into());

            active_user.update(db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update user: {}", e),
                )
            })?;
        }
        None => {
            let mut new_user: user::ActiveModel = Default::default();
            new_user.id = Set(user_uuid);
            new_user.username = Set(oauth_user.username.clone());
            new_user.nickname = Set(oauth_user.nickname.clone());
            new_user.avatar = Set(oauth_user.avatar.clone());
            new_user.created_at = Set(chrono::Utc::now().into());
            new_user.updated_at = Set(chrono::Utc::now().into());

            new_user.insert(db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create user: {}", e),
                )
            })?;
        }
    }

    Ok(())
}

fn internal_database_error(error: sea_orm::DbErr) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
