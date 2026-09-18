// src/module/domain/controller.rs
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use hickory_resolver::TokioAsyncResolver;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    config::AppConfig,
    entity::domain,
    module::{
        auth::extractor::CurrentUser,
        domain::{
            dto::{AddDomainDto, DomainDto, VerifyDomainResultDto},
            service,
        },
    },
};

/// 获取当前用户的站点列表
pub async fn list(
    State(db): State<DatabaseConnection>,
    State(config): State<AppConfig>,
    user: CurrentUser,
) -> Result<Json<Vec<DomainDto>>, (StatusCode, String)> {
    service::list(&db, &config, &user.id.to_string())
        .await
        .map(Json)
}

/// 添加新站点
pub async fn add(
    State(db): State<DatabaseConnection>,
    State(config): State<AppConfig>,
    user: CurrentUser,
    Json(payload): Json<AddDomainDto>,
) -> Result<Json<DomainDto>, (StatusCode, String)> {
    service::add(&db, &config, &user.id.to_string(), payload)
        .await
        .map(Json)
}

/// 触发 DNS TXT 记录所有权验证
pub async fn verify(
    State(db): State<DatabaseConnection>,
    State(config): State<AppConfig>,
    State(resolver): State<TokioAsyncResolver>,
    State(http_client): State<reqwest::Client>,
    user: CurrentUser,
    Path(domain): Path<String>,
) -> Result<Json<VerifyDomainResultDto>, (StatusCode, String)> {
    service::verify(
        &db,
        &resolver,
        &http_client,
        &config,
        &user.id.to_string(),
        &domain,
    )
    .await
    .map(Json)
}

/// 解绑删除站点
pub async fn delete(
    State(db): State<DatabaseConnection>,
    user: CurrentUser,
    Path(domain_name): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let res = domain::Entity::delete_many()
        .filter(domain::Column::Domain.eq(&domain_name))
        .filter(domain::Column::UserId.eq(user.id))
        .exec(&db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if res.rows_affected > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            "Domain not found or unauthorized.".to_string(),
        ))
    }
}
