/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri dev server 约定：固定端口 + 不清屏，便于 tauri dev 挂载
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: 'es2022',
  },
  test: {
    environment: 'jsdom',
    // 组件库测试集中在 src/components/ui 下；快照内联于测试文件
    include: ['src/**/*.test.ts'],
  },
})
