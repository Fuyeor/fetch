// @/main.ts
// pnpm -F @fuyeor/fetch-front-end dev
import App from './App.vue';
import router from './router';

import { createApp } from 'vue';
import { VueQueryPlugin } from '@fuyeor/vue-query';
import { initializeLocale, createHead } from '@fuyeor/commons';
import { vRipple, vTooltip } from '@fuyeor/interactify';

async function bootstrap() {
  const app = createApp(App);
  const head = createHead();

  app.use(router);
  app.use(VueQueryPlugin);
  app.use(head);

  await initializeLocale({ app });

  app.directive('ripple', vRipple);
  app.directive('tooltip', vTooltip);

  app.mount('#app');
}

bootstrap();