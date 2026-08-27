// @/types/search.ts

export interface SearchQuery extends Record<
  string,
  string | number | boolean | undefined
> {
  q: string;
  offset?: number;
  limit?: number;
}

export interface SearchResults {
  query: string;
  offset: number;
  limit: number;
  total: number;
  results: SearchItem[];
}

export interface SearchItem {
  url: string;
  title: string;
  snippet: string;
  updated_at: string;
  images: string[];
  score: number;
}
