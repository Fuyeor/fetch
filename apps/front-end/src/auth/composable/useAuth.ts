// @/auth/composable/useAuth.ts
import { computed, toValue, type MaybeRefOrGetter } from 'vue';
import { useAuthState } from '@/auth/state';
import { useUserState } from '@/user/state';

export function useAuth() {
  const authState = useAuthState();
  const userState = useUserState();
  const isAuthenticated = computed(() => authState.isAuthenticated);
  const currentUser = computed(() => userState.currentUser);
  const isCurrentUser = (usernameToCheck: MaybeRefOrGetter<string>) =>
    computed(() => currentUser.value?.username === toValue(usernameToCheck));

  return {
    isAuthenticated,
    currentUser,
    isCurrentUser,
  };
}
