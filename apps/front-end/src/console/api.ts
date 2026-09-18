// @/console/type.ts
import apiClient from '@app/http';
import type { AddDomainPayload, Domain, VerifyDomainResult } from './type';

/** Fetches all domains belonging to the authenticated user. */
export async function fetchDomains(): Promise<Domain[]> {
  return await apiClient.get<Domain[]>('/domains');
}

/** Registers a new domain for ownership verification. */
export async function createDomain(payload: AddDomainPayload): Promise<Domain> {
  return await apiClient.post<Domain>('/domains', payload);
}

/** Triggers asynchronous DNS TXT record verification. */
export async function verifyDomainOwnership(
  domain: string,
): Promise<VerifyDomainResult> {
  return await apiClient.post<VerifyDomainResult>(
    `/domains/${encodeURIComponent(domain)}/verify`,
  );
}

/** Deletes and unbinds a domain. */
export async function removeDomain(domain: string): Promise<void> {
  await apiClient.delete(`/domains/${encodeURIComponent(domain)}`);
}
