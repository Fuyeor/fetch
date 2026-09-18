// @/user/state.ts
import { ref, computed } from 'vue';
import { defineStore, getCookie } from '@fuyeor/commons';
import { getMyUserDetails } from '@/user/api';
import type { OAuthUser } from '@fuyeor/types';

export const useUserState = defineStore('user', () => {
  // 防并发请求的 Promise 锁
  let userInfoPromise: Promise<void> | null = null;

  const isLoading = ref<boolean>(false);

  const currentUser = ref<OAuthUser | null>(null);
  const isUserLoaded = computed(() => !!currentUser.value);

  function setUser(user: OAuthUser | null) {
    currentUser.value = user;
  }

  function clearUser() {
    currentUser.value = null;
  }

  // 加载当前用户信息（支持并发复用）
  async function loadCurrentUser() {
    if (userInfoPromise) return userInfoPromise;

    if (!getCookie('session_payload')) return;

    isLoading.value = true;
    try {
      userInfoPromise = (async () => {
        const userInfo = await getMyUserDetails();
        setUser(userInfo);
      })();
      await userInfoPromise;
    } catch (error) {
      // 认证失败(401)由 apiClient 全局拦截器处理，断网/500等错误不该强制登出。
      console.error('[userState] Failed to load user info:', error);
      throw error;
    } finally {
      isLoading.value = false;
      // 无论成功或失败，加载结束后都清空 Promise，以便下次可以重新加载
      userInfoPromise = null;
    }
  }

  // 返回 User 信息
  async function fetchCurrentUser(): Promise<OAuthUser> {
    // 如果 state 中已有用户信息，直接返回，避免重复请求
    if (currentUser.value) return currentUser.value;

    // 否则，调用加载函数
    await loadCurrentUser();

    // 加载后再次检查，如果仍然没有，说明加载失败，抛出错误
    if (!currentUser.value) {
      throw new Error('Failed to fetch current user after loading.');
    }

    return currentUser.value;
  }

  return {
    currentUser,
    isUserLoaded,
    setUser,
    clearUser,
    loadCurrentUser,
    fetchCurrentUser,
  };
});
