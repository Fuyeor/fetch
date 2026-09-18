// @/auth/type.ts
import type { EmbeddedUser } from '@/user/type';

/**
 * Request payload for signing in (code exchange)
 */
export interface SigninRequest {
  code: string;
}

/**
 * Response for a successful signin
 */
export interface SigninResponse {
  token: string;
  user: EmbeddedUser;
}

export interface UserToken {
  id: string;
  name: string;
  createdAt: string;
}

export interface UserSummary {
  id: string;
  username: string;
  nickname: string;
  avatar: string | null;
}
