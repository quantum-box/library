import { expect, test } from '@playwright/test'

test.use({ storageState: { cookies: [], origins: [] } })

test('keeps the authentication provider out of the sign-in copy', async ({ page }) => {
  await page.goto('/')

  await expect(page.getByLabel('Email or username')).toBeVisible()
  await expect(page.getByText(/Cognito/i)).toHaveCount(0)
})
