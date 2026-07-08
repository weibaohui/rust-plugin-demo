import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import qiankun from 'vite-plugin-qiankun'

export default defineConfig({
  base: './',
  plugins: [
    react(),
    qiankun('afp-plugin', { useDevMode: true }),
  ],
})
