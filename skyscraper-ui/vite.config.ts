import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'
import { resolve } from 'path'

const root = import.meta.dirname

export default defineConfig({
  plugins: [tailwindcss(), svelte()],

  resolve: {
    alias: {
      '@': resolve(root, 'src'),
      '@lib': resolve(root, 'src/lib'),
      '@components': resolve(root, 'src/components'),
      '@pages': resolve(root, 'src/pages'),
    },
  },

  server: {
    port: 5173,
    host: true,
    proxy: {
      '/api/v1': {
        target: 'http://localhost:3002',
        changeOrigin: true,
      },
      '/cosmium': {
        target: 'http://localhost:3000',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/cosmium/, '/api'),
      },
      '/parallax': {
        target: 'http://localhost:3001',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/parallax/, '/api'),
      },
    },
  },

  build: {
    target: 'es2023',
    sourcemap: true,
  },

  preview: {
    port: 4173,
    host: true,
  },
})
