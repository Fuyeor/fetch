<!-- @/layout/Left.vue -->
<template>
  <LeftSidebar>
    <template #nav>
      <router-link :to="{ name: 'Home' }" class="nav-item">
        <img :src="getIconUrl('home')" class="nav-icon" alt="" />
        <p class="nav-text">{{ t('home') }}</p>
      </router-link>

      <a class="nav-item" v-if="!isAuthenticated" @click.prevent="handleSignin">
        <img :src="getIconUrl('person')" class="nav-icon" alt="" />
        <p class="nav-text">{{ t('signin') }}</p>
      </a>
    </template>

    <template v-if="!isMobile && currentUser" #footer>
      <SidebarUserCard
        :avatar="getAvatarUrl(currentUser.avatar)"
        :nickname="currentUser.nickname"
        :username="currentUser.username"
      />
    </template>
  </LeftSidebar>
</template>

<script setup lang="ts">
import { useLocale } from '@fuyeor/locale';
import { getIconUrl, getAvatarUrl } from '@fuyeor/commons';
import {
  LeftSidebar,
  Foldable,
  SidebarUserCard,
  useMobileDetection,
} from '@fuyeor/interactify';
import { useAuth } from '@/composables/auth/useAuth';

const { t } = useLocale();
const { isMobile } = useMobileDetection();
const { isAuthenticated, currentUser } = useAuth();

// 处理 Ф 账号的 SSO 单点登录
const handleSignin = () => {
  // 从环境变量中读取配置
  const clientId = import.meta.env.VITE_OAUTH_CLIENT_ID;
  const redirectUri = import.meta.env.VITE_REDIRECT_URI;
  const authBaseUrl = import.meta.env.VITE_AUTH_BASE_URL;

  // 生成 state 防 CSRF
  const state = Math.random().toString(36).substring(2);
  window.localStorage.setItem('oauth_state', state); // 将 state 存入 localStorage，用于后续验证

  // 构建完整的 OAuth 授权 URL
  const authUrl = new URL(authBaseUrl);
  authUrl.searchParams.set('response_type', 'code');
  authUrl.searchParams.set('client_id', clientId);
  authUrl.searchParams.set('redirect_uri', redirectUri);
  authUrl.searchParams.set('state', state);

  // 跳转到 www 主站进行 OAuth 授权
  window.location.href = authUrl.toString();
};
</script>
