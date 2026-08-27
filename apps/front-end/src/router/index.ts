// @/router/index.ts
import {
  createRouter,
  RouteLocation,
  RouterView,
  type RouteRecord,
} from '@fuyeor/vue-router';
import { useTransitionBar } from '@fuyeor/interactify';
import { useLocaleStore } from '@fuyeor/commons';
import { LOCALE_REGEX } from '@/config/locales';

const { start, done } = useTransitionBar();

const appRoutes: Array<RouteRecord> = [
  {
    // fetch.fuyeor.com/
    path: '',
    name: 'Home',
    component: () => import('@/views/Home.vue'),
    meta: {
      public: true,
      overrideTitle: ['site.name', ':', 'site.title'],
    },
  },
  {
    // fetch.fuyeor.com/search
    path: 'search',
    name: 'Search',
    component: () => import('@/views/Search.vue'),
    props: true,
    meta: {
      public: true,
      areaKey: 'search',
    },
  },
];

// root router
const routes: Array<RouteRecord> = [
  {
    // optional locale prefix wrapper, only allows supported locales
    path: `{/:locale(${LOCALE_REGEX})}?`,
    component: RouterView,
    // 将所有应用路由放入 children
    // 注意：子路由的 path 如果不以 / 开头，会拼接在父路由后面
    // 例如：/en/thought/123 或 /thought/123
    children: appRoutes,
  },

  // 404 路由必须放在最外层，且在最后
  // 如果 URL 是 /xx/thought/123 (不支持的语言)，正则匹配失败，会落到这里
  {
    path: '/*', // 通配符路由，捕获所有未匹配的路径
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

  // 获取语言 store
  const localeStore = useLocaleStore();

  // 从路由参数中获取 locale (例如 'en' 或 undefined)
  const routeLocale = to.params.locale as string | undefined;

  // 如果路由中有语言参数，且与当前 store 中的不一致，强制切换
  // 这会触发 initializeLocale 中的 loadLocaleMessages，加载新语言包
  if (routeLocale && routeLocale !== localeStore.locale) {
    try {
      await localeStore.setLocale(routeLocale);
    } catch (e) {
      console.error('[RouterGuard] Failed to load locale:', e);
      // 加载语言失败不应该阻塞跳转，只不过界面可能显示默认语言
    }
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
