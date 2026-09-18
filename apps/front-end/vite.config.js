// @fuyeor/fetch-front-end/vite.config.js
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { createViteConfig } from '@fuyeor/config/vite.config.js';

export default defineConfig(() => {
  return createViteConfig(
    {
      resolve: {
        alias: {
          '@app': resolve(__dirname, './src/@app'),
        },
      },
      server: {
        host: '0.0.0.0',
        port: 6040,
        allowedHosts: ['fetch.localhost'],
        proxy: {
          '/v1': {
            target: 'http://localhost:6041',
            changeOrigin: true,
            rewrite: (path) => path.replace(/^\/v1/, ''),
          },
        },
      },
    },
    import.meta.dirname,
  );
});
