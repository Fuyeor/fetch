// @/router/index.ts
import { createRouter, type RouteRecord } from '@fuyeor/vue-router';
import { useTransitionBar } from '@fuyeor/interactify';
import { useAuthStore } from '@/stores/auth';
import { useUserStore } from '@/stores/user';

const { start, done } = useTransitionBar();

const routes: Array<RouteRecord> = [
  {
    // fetch.fuyeor.com/auth/callback
    path: 'auth/callback',
    name: 'AuthCallback',
    component: () => import('@/views/AuthCallback.vue'),
    meta: {
      titleKey: 'signin',
      public: true,
    },
  },
  {
    // fetch.fuyeor.com/
    path: '',
    name: 'Home',
    component: () => import('@/views/Home.vue'),
    meta: {
      public: true,
      overrideTitle: ['site.name', '—', 'site.title'],
    },
  },
  {
    // fetch.fuyeor.com/search
    path: 'search',
    name: 'Search',
    component: () => import('@/views/Search.vue'),
    props: (route) => ({ q: route.query.q || '' }),
    meta: { public: true },
  },
  /*
  {
    // fetch.fuyeor.com/options
    path: 'options',
    name: 'Option',
    component: () => import('@/views/Options/Index.vue'), // 作为布局/容器组件
    meta: {
      areaKey: 'settings',
      titleKey: 'settings',
    },
    children: [
      {
        // fetch.fuyeor.com/options/preferences
        path: 'preferences',
        name: 'Option.Preference',
        component: () => import('@/views/Options/Preferences/Index.vue'),
        meta: {
          titleKey: 'settings.preferences',
        },
        children: [
          {
            // fetch.fuyeor.com/options/preferences/theme
            path: 'theme',
            name: 'Option.Preference.Theme',
            component: () => import('@/views/Options/Preferences/Theme.vue'),
            meta: {
              titleKey: 'settings.prefer.theme',
              public: true,
            },
          },
          {
            // fetch.fuyeor.com/options/preferences/languages
            path: 'languages',
            name: 'Option.Preference.Locale',
            component: () => import('@/views/Options/Preferences/Locale.vue'),
            meta: {
              titleKey: 'settings.prefer.locale',
            },
          },
          {
            // fetch.fuyeor.com/options/preferences/conversations
            path: 'conversations',
            name: 'Option.Preference.Conversations',
            component: () =>
              import('@/views/Options/Preferences/Conversations.vue'),
            meta: {
              titleKey: 'settings.prefer.chat',
            },
          },
        ],
      },
    ],
  },
  */
  {
    // 404 NotFound
    path: '/*',
    name: 'NotFound',
    component: () =>
      import('@fuyeor/interactify/views').then((m) => m.NotFoundView),
    meta: {
      public: true,
      titleKey: 'notFound.title',
    },
  },
];

// 创建路由实例
const router = createRouter({ routes });

// 路由守卫
router.beforeEach(async (to, from) => {
  // 启动顶部进度条
  start();

  const authStore = useAuthStore();
  const userStore = useUserStore();

  // 如果用户已认证但信息未加载，则加载信息。
  // 这是整个守卫中唯一需要加载数据的地方。
  if (authStore.isAuthenticated && !userStore.isUserLoaded) {
    try {
      await userStore.loadCurrentUser();
    } catch {
      // 如果加载失败，store 内部的 onSignOut 会处理状态，
      // authStore.isAuthenticated 将变为 false，后续逻辑会自动处理。
      console.warn(
        '[RouterGuard] User info load failed, session is now invalid.',
      );
    }
  }

  // 检查路由是否需要认证
  const requiresAuth = to.meta.public !== true;

  if (requiresAuth && !authStore.isAuthenticated) {
    // 存业务跳转目标
    window.sessionStorage.setItem('redirect_target', to.fullPath);

    // 如果需要认证但用户未认证，重定向到登录页
    // 保持当前的 locale 参数，如果用户在 /en/ 登录，登录后应该还在 /en/
    return {
      name: 'Auth',
      params: to.params, // 传递 params 以保留 locale
      state: {
        showAuthHint: true,
      },
    };
  }

  // 如果代码能执行到这里，说明所有检查都通过了，允许导航
  return true;
});

router.afterEach(() => {
  done();
});

router.onError(() => {
  done();
});

export default router;
