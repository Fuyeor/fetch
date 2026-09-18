// @/composables/api/useDomain.ts
import { useMutation, useQuery, useQueryClient } from '@fuyeor/vue-query';
import {
  createDomain,
  fetchDomains,
  removeDomain,
  verifyDomainOwnership,
} from '@/console/api';
import type {
  AddDomainPayload,
  Domain,
  VerifyDomainResult,
} from '@/console/type';

/**
 * Query Keys
 */
export const domainKeys = {
  all: ['domains'] as const,
};

/**
 * Query Composable
 */
export function useDomainsQuery(): ReturnType<
  typeof useQuery<Domain[], Error>
> {
  return useQuery<Domain[], Error>({
    queryKey: domainKeys.all,
    queryFn: fetchDomains,
  });
}

/**
 * Mutation: Add Domain
 */
export function useAddDomainMutation(): ReturnType<
  typeof useMutation<Domain, Error, AddDomainPayload>
> {
  const queryClient = useQueryClient();

  return useMutation<Domain, Error, AddDomainPayload>({
    mutationFn: (payload: AddDomainPayload) => createDomain(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: domainKeys.all });
    },
  });
}

/**
 * Mutation: Verify Domain Ownership
 */
export function useVerifyDomainMutation(): ReturnType<
  typeof useMutation<VerifyDomainResult, Error, string>
> {
  const queryClient = useQueryClient();

  return useMutation<VerifyDomainResult, Error, string>({
    mutationFn: (domain: string) => verifyDomainOwnership(domain),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: domainKeys.all });
    },
  });
}

/**
 * Mutation: Delete Domain
 */
export function useDeleteDomainMutation(): ReturnType<
  typeof useMutation<void, Error, string>
> {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: (domain: string) => removeDomain(domain),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: domainKeys.all });
    },
  });
}
