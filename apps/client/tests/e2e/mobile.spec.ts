import { expect, test } from './test-fixtures'

test.describe('Library mobile shell', () => {
  test('supports core workspace flows on a phone viewport', async ({ page }) => {
    const title = `Mobile smoke data ${Date.now()}`

    await page.goto('/databases')

    await expect(page.getByTestId('sync-presence-status-mobile')).toBeVisible()
    await expect(page.getByTestId('side-nav')).toBeHidden()
    await expect(page.getByRole('heading', { name: 'All repository data' })).toBeVisible()
    await expect(page.getByRole('button', { name: 'photon-core' })).toBeVisible()

    await page.getByTestId('open-create-record').click()
    await expect(page.getByTestId('create-record-repository')).toHaveValue('quantum-box/photon-core')
    await page.getByLabel(/Data name/i).fill(title)
    await page.getByTestId('create-record-submit').click()

    await page.getByPlaceholder('Filter data...').fill(title)
    await expect(page.getByTestId('mobile-record-card')).toHaveCount(1)
    await expect(page.getByTestId('mobile-record-card').getByText(title)).toBeVisible()

    await page.getByTestId('mobile-record-card').click()
    await expect(page.getByTestId('detail-panel')).toBeVisible()
    await expect(page.getByTestId('detail-panel')).toHaveCSS('position', 'fixed')
    await page.getByTestId('detail-panel-close').click()
    await expect(page.getByTestId('detail-panel')).toHaveCount(0)

    await page.getByTestId('view-docs-mobile').click()
    await expect(page).toHaveURL(/\/docs/)
    await expect(page.getByRole('heading', { name: 'Docs' })).toBeVisible()

    await page.getByTestId('view-chat-mobile').click()
    await expect(page).toHaveURL(/\/chat/)
    await expect(page.getByRole('heading', { name: 'Chat', exact: true })).toBeVisible()

    await page.getByTestId('view-sync-mobile').click()
    await expect(page).toHaveURL(/\/sync/)
    await expect(page.getByRole('heading', { name: 'Engine diagnostics' })).toBeVisible()
  })
})
