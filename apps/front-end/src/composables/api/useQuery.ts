// @/composables/api/useQuery.ts
import { toValue, type MaybeRefOrGetter } from 'vue';
import { useQuery as useVueQuery } from '@fuyeor/vue-query';
import { fetchSearchResults } from '@/api/search';
import type { SearchQuery } from '@/types/search';

/** Composable for executing search queries with @fuyeor/vue-query. */
export function useQuery(params: MaybeRefOrGetter<SearchQuery>) {
  return useVueQuery({
    queryKey: ['search', () => toValue(params)],
    queryFn: () => fetchSearchResults(toValue(params)),
  });
}
