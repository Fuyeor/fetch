// @/api/index.ts
import { HttpClient } from '@fuyeor/commons';

const apiClient = new HttpClient({
  baseURL: '/v1',
  timeout: 10000,
  credentials: 'include',
});

export default apiClient;
