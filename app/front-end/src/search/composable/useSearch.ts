// @/view/search/composable/useSearch.ts
import { computed, MaybeRefOrGetter, toValue } from 'vue';
import { useQuery } from '@fuyeor/vue-query';
import { fetchSearchResults } from '@/search/api';
import type { SearchQuery, SearchResults } from '@/search/type';

/**
 * Query Keys
 */
export const searchKeys = {
  all: ['search'] as const,
  results: (params: SearchQuery) =>
    [...searchKeys.all, 'results', params] as const,
};

/**
 * excute Composable
 */
export function useSearchResultsQuery(params: MaybeRefOrGetter<SearchQuery>) {
  const paramsRef = computed(() => toValue(params));

  return useQuery<SearchResults, Error>({
    queryKey: computed(() => searchKeys.results(paramsRef.value)),
    queryFn: () => fetchSearchResults(paramsRef.value),
    enabled: computed(() => Boolean(paramsRef.value.q?.trim())),
    staleTime: 1000 * 60 * 2,
    retry: 1,
  });
}
