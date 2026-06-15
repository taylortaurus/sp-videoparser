import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// 产物输出到 ../templates/dist，由 Rust 后端经 rust-embed 编译期内嵌。
// base '/' 与 assetsDir 'assets' 对应后端的 / 与 /assets/*path 路由。
export default defineConfig({
  plugins: [vue()],
  base: '/',
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) }
  },
  server: {
    port: 3000,
    // 开发期把 /api 透传到本地 Rust 服务（保留 /api 前缀，后端路由就在 /api 下）。
    proxy: {
      '/api': { target: 'http://localhost:8080', changeOrigin: true }
    }
  },
  build: {
    outDir: '../templates/dist',
    emptyOutDir: true,
    assetsDir: 'assets'
  }
})
