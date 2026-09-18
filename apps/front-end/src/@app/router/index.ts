// @app/router/index.ts
import { createRouter, type RouteRecord } from '@fuyeor/vue-router';
import { useTransitionBar } from '@fuyeor/interactify';
import { useAuthState } from '@/auth/state';
import { useUserState } from '@/user/state';

const { start, done } = useTransitionBar();

const routes: Array<RouteRecord> = [
  {
    // fetch.fuyeor.com/
    path: '',
    name: 'Home',
    component: () => import('@/home/index.vue'),
    meta: {
      public: true,
      overrideTitle: ['site.name', '—', 'site.title'],
    },
  },
  {
    // fetch.fuyeor.com/auth/callback
    path: 'auth/callback',
    name: 'AuthCallback',
    component: () => import('@/auth/callback.vue'),
    meta: {
      titleKey: 'signin',
      public: true,
    },
  },
  {
    // fetch.fuyeor.com/search
    path: 'search',
    name: 'Search',
    component: () => import('@/search/index.vue'),
    props: (route) => ({ q: route.query.q || '' }),
    meta: { public: true },
  },
  {
    path: 'console',
    children: [
      // fetch.fuyeor.com/console
      {
        path: '',
        name: 'Console',
        component: () => import('@/console/index.vue'),
        meta: { titleKey: 'console' },
      },
      {
        // fetch.fuyeor.com/console/fuyeor.com
        path: ':domain',
        redirect: (to) => ({
          name: 'Console.Domain.Overview',
          params: to.params,
        }),
        meta: { titleKey: 'console.domain' },
        children: [
          {
            // fetch.fuyeor.com/console/fuyeor.com/overview
            path: 'overview',
            name: 'Console.Domain.Overview',
            component: () => import('@/console/domain/overview.vue'),
          },
        ],
      },
    ],
  },
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

  const authState = useAuthState();
  const userState = useUserState();

  // 如果用户已认证但信息未加载，则加载信息。
  // 这是整个守卫中唯一需要加载数据的地方。
  if (authState.isAuthenticated && !userState.isUserLoaded) {
    try {
      await userState.loadCurrentUser();
    } catch {
      // 如果加载失败，store 内部的 onSignOut 会处理状态，
      // authState.isAuthenticated 将变为 false，后续逻辑会自动处理。
      console.warn(
        '[RouterGuard] User info load failed, session is now invalid.',
      );
    }
  }

  // 检查路由是否需要认证
  const requiresAuth = to.meta.public !== true;

  if (requiresAuth && !authState.isAuthenticated) {
    // 如果需要认证但用户未认证，重定向到登录页
    // 保持当前的 locale 参数，如果用户在 /en/ 登录，登录后应该还在 /en/
    return {
      name: 'Home',
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
