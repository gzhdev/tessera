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
})
