// @/auth/state.ts
import { ref } from 'vue';
import { defineStore, getCookie } from '@fuyeor/commons';
import { useUserState } from '@/user/state';
import type { OAuthUser } from '@fuyeor/types';

export const useAuthState = defineStore('auth', () => {
  // 通过检查 session_payload cookie 来初始化登录状态。
  const isAuthenticated = ref<boolean>(!!getCookie('session_payload'));

  // 登录成功后的统一处理逻辑
  const onSigninSuccess = (user: OAuthUser) => {
    isAuthenticated.value = true;

    const userState = useUserState();
    userState.setUser(user);
  };

  // 内部函数：在登出或会话失效后，统一清理所有与用户相关的状态。
  const onSignOut = () => {
    isAuthenticated.value = false;
    // 清理 User Store 中的用户数据
    useUserState().clearUser();
    // 在所有状态清理完毕后，强制跳转并刷新到首页
    window.location.href = '/';
  };

  return {
    // State
    isAuthenticated,
    // Actions
    onSigninSuccess,
    onSignOut,
  };
});
