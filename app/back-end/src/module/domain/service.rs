// src/module/domain/service.rs
use axum::http::StatusCode;
use hickory_resolver::TokioAsyncResolver;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::{
    config::AppConfig,
    entity::{domain, prelude::Domain, sea_orm_active_enums::DomainStatus},
    module::domain::{
        dto::{AddDomainDto, DomainDto, VerifyDomainResultDto},
        util::{canonicalize_and_validate_domain, derive_verification_token},
    },
    util::dns::check_txt_contains,
};

pub async fn add(
    db: &DatabaseConnection,
    config: &AppConfig,
    user_id: &str,
    payload: AddDomainDto,
) -> Result<DomainDto, (StatusCode, String)> {
    // 严格使用 PSL 与 Punycode 校验
    let canonical_domain = canonicalize_and_validate_domain(&payload.domain)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid user UUID: {}", e)))?;

    let existing = Domain::find_by_id(&canonical_domain)
        .one(db)
        .await
        .map_err(internal_database_error)?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "This domain is already registered.".to_string(),
        ));
    }

    let now = chrono::Utc::now().into();
    let new_domain = domain::ActiveModel {
        domain: Set(canonical_domain.clone()),
        user_id: Set(user_uuid),
        status: Set(DomainStatus::Pending),
        created_at: Set(now),
        updated_at: Set(now),
        verified_at: Set(None),
    };

    new_domain
        .insert(db)
        .await
        .map_err(internal_database_error)?;

    let token = derive_verification_token(user_id, &canonical_domain, &config.verification_salt);

    Ok(DomainDto {
        domain: canonical_domain.clone(),
        status: "pending".to_string(),
        verification_token: token.clone(),
        dns_record_name: format!("_fetch-challenge.{}", canonical_domain),
        dns_record_value: format!("fetch-verification={}", token),
        created_at: chrono::Utc::now().to_rfc3339(),
        verified_at: None,
    })
}

pub async fn verify(
    db: &DatabaseConnection,
    resolver: &TokioAsyncResolver,
    http_client: &reqwest::Client,
    config: &AppConfig,
    user_id: &str,
    domain_name: &str,
) -> Result<VerifyDomainResultDto, (StatusCode, String)> {
    let canonical_domain = canonicalize_and_validate_domain(domain_name)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;

    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid user UUID: {}", e)))?;

    let domain_model = Domain::find_by_id(&canonical_domain)
        .filter(domain::Column::UserId.eq(user_uuid))
        .one(db)
        .await
        .map_err(internal_database_error)?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Domain not found under your account.".to_string(),
        ))?;

    let expected_token =
        derive_verification_token(user_id, &canonical_domain, &config.verification_salt);

    // 途径 1: DNS TXT 记录查询
    let challenge_host = format!("_fetch-challenge.{}", canonical_domain);
    let mut matched: bool = check_txt_contains(resolver, &challenge_host, &expected_token).await;

    // 途径 2: HTTP 文件验证 (/.well-known/fetch-challenge.txt)
    if !matched {
        let file_url = format!(
            "https://{}/.well-known/fetch-challenge.txt",
            canonical_domain
        );
        if let Ok(resp) = http_client.get(&file_url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.text().await {
                    if body.trim() == expected_token {
                        matched = true;
                    }
                }
            }
        }
    }

    if matched {
        let mut active: domain::ActiveModel = domain_model.into();
        active.status = Set(DomainStatus::Verified);
        active.verified_at = Set(Some(chrono::Utc::now().into()));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(internal_database_error)?;

        Ok(VerifyDomainResultDto {
            domain: canonical_domain,
            verified: true,
            message: "Domain ownership verified successfully via DNS or well-known file!"
                .to_string(),
        })
    } else {
        let message = format!(
            "Verification failed: DNS TXT record matching '{}' not found on '{}', and https://{}/.well-known/fetch-challenge.txt is unreachable.",
            expected_token, challenge_host, canonical_domain
        );

        Ok(VerifyDomainResultDto {
            domain: canonical_domain,
            verified: false,
            message,
        })
    }
}

pub async fn list(
    db: &DatabaseConnection,
    config: &AppConfig,
    user_id: &str,
) -> Result<Vec<DomainDto>, (StatusCode, String)> {
    let user_uuid = Uuid::parse_str(user_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid user UUID: {}", e)))?;

    let domains = Domain::find()
        .filter(domain::Column::UserId.eq(user_uuid))
        .all(db)
        .await
        .map_err(internal_database_error)?;

    let result = domains
        .into_iter()
        .map(|d| {
            let token = derive_verification_token(user_id, &d.domain, &config.verification_salt);
            DomainDto {
                domain: d.domain.clone(),
                status: match d.status {
                    DomainStatus::Pending => "pending".to_string(),
                    DomainStatus::Verified => "verified".to_string(),
                    DomainStatus::Failed => "failed".to_string(),
                },
                verification_token: token.clone(),
                dns_record_name: format!("_fetch-challenge.{}", d.domain),
                dns_record_value: token,
                created_at: d.created_at.to_rfc3339(),
                verified_at: d.verified_at.map(|t| t.to_rfc3339()),
            }
        })
        .collect();

    Ok(result)
}

fn internal_database_error(error: sea_orm::DbErr) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
