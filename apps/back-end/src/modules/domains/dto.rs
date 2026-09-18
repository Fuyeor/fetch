// src/modules/domains/dto.rs
use serde::{Deserialize, Serialize};

/// 添加站点请求入参
#[derive(Deserialize)]
pub struct AddDomainDto {
    pub domain: String,
}

/// 站点详细信息响应体（包含 DNS 验证指引）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainDto {
    pub domain: String,
    pub status: String,
    pub verification_token: String,
    pub dns_record_name: String,
    pub dns_record_value: String,
    pub created_at: String,
    pub verified_at: Option<String>,
}

/// DNS 验证结果响应体
#[derive(Serialize)]
pub struct VerifyDomainResultDto {
    pub domain: String,
    pub verified: bool,
    pub message: String,
}
