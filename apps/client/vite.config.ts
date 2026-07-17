import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const host = process.env.TAURI_DEV_HOST
const tauriPlatform = process.env.TAURI_ENV_PLATFORM
const isTauriDebug = Boolean(process.env.TAURI_ENV_DEBUG)
const devServerPort = Number(process.env.LIBRARY_DEV_SERVER_PORT ?? 5173)
const appServerUrl = process.env.LIBRARY_APP_SERVER_URL ?? 'http://127.0.0.1:3001'

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
    port: devServerPort,
    strictPort: true,
    allowedHosts: true,
    hmr: host ? { protocol: 'ws', host, port: 5174 } : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
    proxy: {
      '/api': {
        target: appServerUrl,
      },
      '/ws': {
        target: appServerUrl,
        ws: true,
      },
    },
  },
})
