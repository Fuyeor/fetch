// @/composables/auth/useAuth.ts
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { useAuthStore } from '@/stores/auth';
import { useUserStore } from '@/stores/user';

export function useAuth() {
  const authStore = useAuthStore();
  const userStore = useUserStore();
  const isAuthenticated = computed(() => authStore.isAuthenticated);
  const currentUser = computed(() => userStore.currentUser);
  const isCurrentUser = (usernameToCheck: MaybeRefOrGetter<string>) =>
    computed(() => currentUser.value?.username === toValue(usernameToCheck));

  return {
    isAuthenticated,
    currentUser,
    isCurrentUser,
  };
}
