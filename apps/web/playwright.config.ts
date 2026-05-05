import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: true,
	forbidOnly: Boolean(process.env.CI),
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined,
	reporter: process.env.CI ? [['html'], ['list']] : 'list',
	use: {
		baseURL: process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5010',
		locale: 'ja-JP',
		timezoneId: 'Asia/Tokyo',
		trace: 'on-first-retry',
	},
	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] },
		},
	],
	webServer: {
		command: 'yarn dev --host 127.0.0.1',
		env: {
			VITE_COGNITO_CLIENT_ID: 'playwright-client-id',
			VITE_COGNITO_REGION: 'ap-northeast-1',
		},
		url: 'http://127.0.0.1:5010',
		reuseExistingServer: !process.env.CI,
		timeout: 120 * 1000,
	},
})
