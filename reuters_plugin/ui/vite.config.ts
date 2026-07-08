import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import qiankun from 'vite-plugin-qiankun'

export default defineConfig({
  base: '/plugin-files/reuters_plugin/ui/dist/',
  plugins: [
    react(),
    qiankun('reuters-plugin', { useDevMode: false }),
  ],
})
