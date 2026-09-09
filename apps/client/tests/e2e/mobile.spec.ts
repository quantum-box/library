import { expect, test } from './test-fixtures'

test.describe('Library mobile shell', () => {
  test('supports core workspace flows on a phone viewport', async ({ page }) => {
    const title = `Mobile smoke data ${Date.now()}`

    await page.goto('/databases')

    await expect(page.getByTestId('sync-presence-status-mobile')).toBeVisible()
    await expect(page.getByTestId('side-nav')).toBeHidden()
    await expect(page.getByRole('heading', { name: 'All repository data' })).toBeVisible()

    // The workspace nav lives behind the app bar's drawer on a phone.
    await expect(page.getByTestId('mobile-nav')).toHaveCount(0)
    await page.getByTestId('open-mobile-nav').click()
    await expect(page.getByTestId('mobile-nav')).toBeVisible()
    await expect(
      page.getByTestId('mobile-nav').getByText('quantum-box/photon-core')
    ).toBeVisible()
    await page.getByTestId('close-mobile-nav').click()
    await expect(page.getByTestId('mobile-nav')).toHaveCount(0)

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

    await page.getByTestId('open-mobile-nav').click()
    await page.getByTestId('view-chat-mobile').click()
    await expect(page).toHaveURL(/\/chat/)
    await expect(page.getByRole('heading', { name: 'Chat', exact: true })).toBeVisible()

    await page.getByTestId('open-mobile-nav').click()
    await page.getByTestId('view-sync-mobile').click()
    await expect(page).toHaveURL(/\/sync/)
    await expect(page.getByRole('heading', { name: 'Engine diagnostics' })).toBeVisible()
  })

  test('shows repository data as cards instead of a wide table', async ({ page }) => {
    await page.goto('/quantum-box/photon-core/data')

    const cards = page.getByTestId('library-table-card')
    await expect(cards.first()).toBeVisible()
    await expect(page.getByTestId('library-table-view').locator('table')).toHaveCount(0)

    // The shell never pans sideways: every overflow is owned by a pane.
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    )
    expect(overflow).toBeLessThanOrEqual(0)

    await cards.first().click()
    await expect(page).toHaveURL(/\/quantum-box\/photon-core\/data\//)
  })

  /**
   * A data name is free text and nothing stops it being one 200-character
   * token -- a pasted URL, an exported identifier. Wrapping is what keeps that
   * inside the card instead of widening the pane and letting the phone pan.
   */
  test('keeps a long unbroken title inside the screen', async ({ page, request }) => {
    const title = `long-${'x'.repeat(180)}`

    // Renamed through the API rather than the UI: the point is what the list
    // does with a name it cannot break, not how the name got there.
    const renamed = await request.post('http://127.0.0.1:50063/v1/graphql', {
      data: {
        query: 'mutation LibraryClientUpdateData { updateData { id } }',
        variables: {
          input: {
            orgUsername: 'quantum-box',
            repoUsername: 'photon-core',
            dataId: 'seed-data-201',
            dataName: title,
          },
        },
      },
    })
    expect(renamed.ok(), await renamed.text()).toBe(true)

    await page.goto('/quantum-box/photon-core/data')
    const card = page.getByTestId('library-table-card').filter({ hasText: 'long-xxx' })
    await expect(card).toHaveCount(1)

    const pagePan = () =>
      page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth
      )
    const spill = (locator: ReturnType<typeof page.getByTestId>) =>
      locator.evaluate((element) => element.scrollWidth - element.clientWidth)

    expect(await pagePan()).toBeLessThanOrEqual(0)
    expect(await spill(card)).toBeLessThanOrEqual(0)

    // The data page shows the same name as a heading, at three times the size.
    await card.click()
    await expect(page).toHaveURL(/\/quantum-box\/photon-core\/data\/.+/)
    await expect(page.getByText('long-xxx', { exact: false }).first()).toBeVisible()

    expect(await pagePan()).toBeLessThanOrEqual(0)
    expect(
      await page.evaluate(() => {
        const main = document.querySelector('main') ?? document.body
        return main.scrollWidth - main.clientWidth
      })
    ).toBeLessThanOrEqual(0)
  })

  test('reads public documentation without signing in on mobile', async ({ page, request }) => {
    // The public route is what a shared link opens, so exercise it the way a
    // visitor arrives: no session, phone viewport.
    await request.post('http://127.0.0.1:50063/v1/graphql', {
      data: {
        query: 'mutation LibraryClientUpdateRepository { updateRepo { id } }',
        variables: {
          input: {
            orgUsername: 'quantum-box',
            repoUsername: 'photon-core',
            isPublic: true,
          },
        },
      },
    })
    await page.context().clearCookies()
    await page.goto('/public/quantum-box/photon-core')
    await page.evaluate(() => window.localStorage.clear())
    await page.reload()

    const cards = page.getByRole('main').getByRole('link', { name: 'Prepare release notes', exact: true })
    await expect(cards.first()).toBeVisible()
    await expect(page.locator('table')).toHaveCount(0)

    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    )
    expect(overflow).toBeLessThanOrEqual(0)

    await cards.first().click()
    await expect(page).toHaveURL(/\/public\/quantum-box\/photon-core\/.+/)
    await expect(page.getByRole('heading', { name: 'Prepare release notes', exact: true })).toBeVisible()
    await expect(page.locator('[contenteditable="true"]')).toHaveCount(0)
    await page.getByRole('button', { name: 'Open article menu' }).click()
    await page.getByRole('navigation', { name: 'Articles', exact: true }).getByRole('link', { name: 'Review content schema', exact: true }).click()
    await expect(page.getByRole('heading', { name: 'Review content schema', exact: true })).toBeVisible()
  })
})
