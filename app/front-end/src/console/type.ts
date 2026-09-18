// @/types/domain.ts

export type DomainStatus = 'pending' | 'verified' | 'failed';

// 站点核心实体
export interface Domain {
  domain: string;
  status: DomainStatus;
  verificationToken: string;
  dnsRecordName: string;
  dnsRecordValue: string;
  createdAt: string;
  verifiedAt: string;
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
