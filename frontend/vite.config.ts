import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  plugins: [
    sveltekit({
      adapter: adapter({ fallback: 'index.html', precompress: true }),
      paths: { relative: false },
      preprocess: vitePreprocess()
    })
  ],
  resolve: { alias: { '$lib': fileURLToPath(new URL('./src/lib', import.meta.url)) } },
  server: { proxy: { '/api': 'http://127.0.0.1:3000', '/fonts': 'http://127.0.0.1:3000' } }
});
