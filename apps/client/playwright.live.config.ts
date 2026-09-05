import { defineConfig, devices } from '@playwright/test'
import { e2eAuthState } from './tests/e2e/auth-state'

/** Real Photon relay and Durable Object storage; fixture Library API. */
export default defineConfig({
  testDir: './tests/e2e',
  testMatch: /data-editor-live\.spec\.ts/,
  fullyParallel: false,
  workers: 1,
  timeout: 180_000,
  expect: { timeout: 30_000 },
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'playwright-report/live' }]],
  use: {
    baseURL: 'http://127.0.0.1:5187',
    storageState: e2eAuthState,
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: [
    {
      command: 'LIBRARY_E2E_API_PORT=50063 node tests/e2e/library-api-fixture.mjs',
      url: 'http://127.0.0.1:50063/__e2e/health',
      reuseExistingServer: false,
    },
    {
      command: 'wrangler dev --config wrangler.live-test.jsonc --port 8788 --persist-to /tmp/library-live-e2e-state',
      url: 'http://127.0.0.1:8788/api/health',
      reuseExistingServer: false,
      timeout: 120_000,
      env: { WRANGLER_SEND_METRICS: 'false' },
    },
    {
      command: 'LIBRARY_DEV_SERVER_PORT=5187 LIBRARY_APP_SERVER_URL=http://127.0.0.1:50063 VITE_ENABLE_DEV_TOKEN_AUTH=true VITE_LIBRARY_API_BASE_URL=http://127.0.0.1:50063 VITE_LIBRARY_SYNC_WS_URL=ws://127.0.0.1:50063/ws VITE_LIBRARY_DATA_LIVE_URL=http://127.0.0.1:8788 VITE_LIBRARY_CHAT_STREAM_MODE=mock npm run dev -- --host 127.0.0.1',
      url: 'http://127.0.0.1:5187',
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
})
