// @/auth/api.ts
import apiClient from '@app/http';
import type { SigninRequest, SigninResponse, UserToken } from './type';
import type { ApiResponse, OAuthUser } from '@fuyeor/types';

/**
 * 处理 OAuth 回调，换取用户信息
 */
export const authCallback = async (code: string, state: string) => {
  // 后端返回的是 { success: true, data: { user: { ... } } }
  const response = await apiClient.post<ApiResponse<{ user: OAuthUser }>>(
    '/auth/callback',
    { code },
  );
  return response.data.user;
};

/**
 * @description 调用接口以刷新令牌。
 * 这个函数现在不接收参数，也不直接返回令牌。
 * 它只是触发一个网络请求，浏览器和服务器会自动处理 Cookie。
 * 现在它不再返回用户基本信息。
 */
export const refreshToken = async (): Promise<void> => {
  await apiClient.post('/auth/refresh-token');
};

/**
 * @description 用户登出。
 * 调用此接口会清除后端的 session 和前端的 HttpOnly Cookie。
 */
export const signOut = async (): Promise<void> => {
  // 登出接口通常不返回有意义的数据
  await apiClient.post('/auth/sign-out');
};
