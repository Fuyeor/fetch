// @fuyeor/fetch-front-end/vite.config.js
import { defineConfig } from 'vite';
import { createViteConfig } from '@fuyeor/config/vite.config.js';

export default defineConfig(() => {
  return createViteConfig(
    {
      server: {
        host: '0.0.0.0',
        port: 6040,
        allowedHosts: ['fetch.localhost'],
        proxy: {
          '/v1': {
            target: 'http://localhost:3000',
            changeOrigin: true,
            rewrite: (path) => path.replace(/^\/v1/, ''),
          },
          '/docs': {
            target: 'http://localhost:3000',
            changeOrigin: true,
          },
        },
      },
    },
    import.meta.dirname,
  );
});
