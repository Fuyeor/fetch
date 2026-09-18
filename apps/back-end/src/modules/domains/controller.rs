// src/modules/domains/controller.rs
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use hickory_resolver::TokioAsyncResolver;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    config::AppConfig,
    entities::domain,
    modules::{
        auth::extractor::CurrentUser,
        domains::{
            dto::{AddDomainDto, DomainDto, VerifyDomainResultDto},
            service,
        },
    },
};

/// 获取当前用户的站点列表
pub async fn list_domains(
    State(db): State<DatabaseConnection>,
    State(config): State<AppConfig>,
    user: CurrentUser,
) -> Result<Json<Vec<DomainDto>>, (StatusCode, String)> {
    service::list_domains(&db, &config, &user.id.to_string())
        .await
        .map(Json)
}

/// 添加新站点
pub async fn add_domain(
    State(db): State<DatabaseConnection>,
    State(config): State<AppConfig>,
    user: CurrentUser,
    Json(payload): Json<AddDomainDto>,
) -> Result<Json<DomainDto>, (StatusCode, String)> {
    service::add_domain(&db, &config, &user.id.to_string(), payload)
        .await
        .map(Json)
}

/// 触发 DNS TXT 记录所有权验证
pub async fn verify_domain(
    State(db): State<DatabaseConnection>,
    State(config): State<AppConfig>,
    State(resolver): State<TokioAsyncResolver>,
    State(http_client): State<reqwest::Client>,
    user: CurrentUser,
    Path(domain): Path<String>,
) -> Result<Json<VerifyDomainResultDto>, (StatusCode, String)> {
    service::verify_domain(
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
pub async fn delete_domain(
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
