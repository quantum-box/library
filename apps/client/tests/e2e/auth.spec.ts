import { expect, test } from '@playwright/test'

test.use({ storageState: { cookies: [], origins: [] } })

test('keeps the authentication provider out of the sign-in copy', async ({ page }) => {
  await page.goto('/')

  await expect(page.getByLabel('Email or username')).toBeVisible()
  await expect(page.getByText(/Cognito/i)).toHaveCount(0)
})

test('opens canonical public repository URLs without signing in', async ({ page, request }) => {
  await request.post('http://127.0.0.1:50063/v1/graphql', {
    data: {
      query: 'mutation LibraryClientUpdateRepository { updateRepo { id } }',
      variables: { input: { orgUsername: 'quantum-box', repoUsername: 'photon-core', isPublic: true } },
    },
  })
  for (const path of ['/quantum-box/photon-core', '/quantum-box/photon-core/data']) {
    await page.goto(path)
    await expect(page.getByRole('main').getByRole('link', { name: 'Prepare release notes', exact: true }).first()).toBeVisible()
    await expect(page.getByLabel('Email or username')).toHaveCount(0)
  }
  await page.goto('/quantum-box/photon-core/data/seed-data-201')
  await expect(page.getByRole('heading', { name: 'Prepare release notes', exact: true })).toBeVisible()
  await expect(page.locator('[contenteditable="true"]')).toHaveCount(0)

  await request.post('http://127.0.0.1:50063/v1/graphql', {
    data: {
      query: 'mutation LibraryClientUpdateRepository { updateRepo { id } }',
      variables: { input: { orgUsername: 'quantum-box', repoUsername: 'photon-core', isPublic: false } },
    },
  })
  await page.goto('/quantum-box/photon-core')
  await expect(page.getByTestId('public-repository-private')).toBeVisible()
  await expect(page.getByText('Prepare release notes', { exact: true })).toHaveCount(0)
  await page.goto('/quantum-box/photon-core/settings')
  await expect(page.getByLabel('Email or username')).toBeVisible()
})
