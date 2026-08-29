// @/api/search.ts
import apiClient from '@/api';
import type { SearchQuery, SearchResults } from '@/types/search';

/** Fetches search results based on query parameters. */
export async function fetchSearchResults(
  params: SearchQuery,
): Promise<SearchResults> {
  return await apiClient.get<SearchResults>('/search', {
    params,
  });
}
