import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import qiankun from 'vite-plugin-qiankun'

export default defineConfig({
  base: '/plugin-files/reuters_plugin/ui/dist/',
  plugins: [
    vue(),
    qiankun('reuters-plugin', { useDevMode: false }),
  ],
})
