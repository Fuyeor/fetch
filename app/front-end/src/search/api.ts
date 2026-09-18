// @/api/search.ts
import apiClient from '@app/http';
import type { SearchQuery, SearchResults } from './type';

/** Fetches search results based on query parameters. */
export async function fetchSearchResults(
  params: SearchQuery,
): Promise<SearchResults> {
  return await apiClient.get<SearchResults>('/search', {
    params,
  });
}
