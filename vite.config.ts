import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

// Tauri 2 推荐配置 —— 固定端口 1420,frontendDist 路径对位 src-tauri/tauri.conf.json。
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: 'localhost',
    hmr: { port: 5174 },
    watch: {
      // 避免 vite 监听到 Tauri 构建产物
      ignored: ['**/src-tauri/**', '**/dist/**'],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'esnext',
  },
});
