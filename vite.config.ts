import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({
  plugins: [react()], clearScreen: false,
  server: {
    host: '127.0.0.1', port: 1420, strictPort: true,
    proxy: { '/api': { target: 'http://127.0.0.1:1421' } },
    watch: { ignored: ['**/target/**', '**/src-tauri/**', '**/.tools/**'] },
    headers: { 'X-Frame-Options': 'DENY', 'Referrer-Policy': 'no-referrer' },
  },
  build: { target: 'es2022' },
});
