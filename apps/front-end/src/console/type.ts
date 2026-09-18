// @/types/domain.ts

export type DomainStatus = 'pending' | 'verified' | 'failed';

// 站点核心实体
export interface Domain {
  domain: string;
  status: DomainStatus;
  verification_token: string;
  dns_record_name: string;
  dns_record_value: string;
  created_at: string;
  verified_at: string;
}

// 添加站点入参
export interface AddDomainPayload {
  domain: string;
}

// DNS TXT 验证响应
export interface VerifyDomainResult {
  domain: string;
  verified: boolean;
  message: string;
}
