import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const host = process.env.TAURI_DEV_HOST
const tauriPlatform = process.env.TAURI_ENV_PLATFORM
const isTauriDebug = Boolean(process.env.TAURI_ENV_DEBUG)

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: tauriPlatform
      ? tauriPlatform === 'windows'
        ? 'chrome105'
        : 'safari13'
      : undefined,
    minify: isTauriDebug ? false : undefined,
    sourcemap: isTauriDebug,
  },
  server: {
    host: host || false,
    port: 5173,
    strictPort: true,
    allowedHosts: true,
    hmr: host ? { protocol: 'ws', host, port: 5174 } : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
    proxy: {
      '/api': {
        target: 'http://localhost:3001',
      },
      '/ws': {
        target: 'http://localhost:3001',
        ws: true,
      },
    },
  },
})
