// @/api/users.ts
import apiClient from './index';
import type { OAuthUser } from '@fuyeor/types';
import type { EmbeddedUser } from '@/types/user';

type AuthUserResponse = Pick<OAuthUser, 'id' | 'username' | 'nickname'> & {
  avatar: string | null;
};

/** Loads the current user from the Answers JWT session. */
export const getMyUserDetails = async (): Promise<OAuthUser> => {
  return await apiClient.get<AuthUserResponse>('/auth/me');
};

/** Loads a public profile by username. */
export const getUser = (username: string) =>
  apiClient.get<EmbeddedUser>(`/users/${encodeURIComponent(username)}`);
