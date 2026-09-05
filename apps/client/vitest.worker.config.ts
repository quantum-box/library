import { defineConfig } from 'vitest/config'
import { fileURLToPath } from 'node:url'

export default defineConfig({
  resolve: {
    alias: {
      'cloudflare:workers': fileURLToPath(new URL('./workers/sync/test-cloudflare-workers.ts', import.meta.url)),
      '@quantum-box/photon/worker': fileURLToPath(new URL('./workers/sync/test-photon-worker.ts', import.meta.url)),
    },
  },
  test: {
    environment: 'node',
    include: ['workers/sync/**/*.test.ts'],
    testTimeout: 10_000,
  },
})
