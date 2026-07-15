import { expect, test, type Page } from '@playwright/test'
import { e2eAuthState } from './auth-state'

async function mockLibraryRepository(page: Page) {
  const repository = {
    id: 'repo-1',
    username: 'photon-core',
    name: 'Photon Core',
    description: 'Library E2E repository',
  }

  await page.route('**/v1beta/repos', async (route) => {
    await route.fulfill({
      json: [
        {
          ...repository,
          organization_id: 'org-1',
          org_username: 'quantum-box',
        },
      ],
    })
  })

  await page.route('**/v1/graphql', async (route) => {
    const body = route.request().postDataJSON() as { query?: string }
    const query = body.query ?? ''

    if (query.includes('LibraryClientMeOrganizations')) {
      await route.fulfill({
        json: {
          data: {
            me: {
              id: 'library-e2e-user',
              email: 'library-e2e@local.test',
              tenantIdList: ['org-1'],
              organizations: [
                {
                  id: 'org-1',
                  operatorName: 'quantum-box',
                  platformTenantId: 'tn_01j702qf86pc2j35s0kv0gv3gy',
                  repos: [repository],
                },
              ],
            },
          },
        },
      })
      return
    }

    if (query.includes('LibraryClientOrganizationRepos')) {
      await route.fulfill({
        json: {
          data: {
            organization: {
              id: 'org-1',
              name: 'Quantum Box',
              username: 'quantum-box',
              repos: [repository],
            },
          },
        },
      })
      return
    }

    await route.fulfill({
      json: {
        data: {
          repo: {
            id: repository.id,
            name: repository.name,
            dataList: { items: [] },
            properties: [],
          },
        },
      },
    })
  })
}

test.describe('Library shell', () => {
  test('opens the database table and creates a new record', async ({ page }) => {
    const title = `E2E smoke record ${Date.now()}`

    await page.goto('/')

    await expect(page).toHaveURL(/\/home/)
    await expect(page.getByRole('heading', { name: 'Home', exact: true })).toBeVisible()
    await page.getByRole('link', { name: 'New data' }).click()
    await expect(page).toHaveURL(/\/databases/)
    await expect(page.getByRole('heading', { name: 'All repository data' })).toBeVisible()
    await expect(page.getByText(/\d+ records/)).toBeVisible()

    await page.getByTestId('open-create-record').click()
    await expect(page.getByTestId('create-record-modal')).toBeVisible()

    await page.getByLabel(/Record title/i).fill(title)
    await page.getByLabel('Description').fill('Created from Playwright')
    await page.getByTestId('create-record-submit').click()

    await expect(page.getByTestId('create-record-modal')).toBeHidden()
    await page.getByPlaceholder('Filter records...').fill(title)
    await expect(page.getByText(title)).toBeVisible()
  })

  test('switches between table, board, workflow, docs, and chat views', async ({ page }) => {
    await page.goto('/databases')
    await page.getByTestId('view-kanban').click()

    await expect(page).toHaveURL(/view=.*board/)
    await expect(page.getByRole('heading', { name: 'All repository data' })).toBeVisible()
    await expect(page.getByText('drag to move')).toBeVisible()

    await page.getByTestId('view-workflow').click()

    await expect(page).toHaveURL(/view=.*workflow/)
    await expect(page.getByRole('heading', { name: 'All repository data' })).toBeVisible()
    await expect(page.getByTestId('workflow-canvas')).toBeVisible()
    await expect(page.getByTestId('workflow-elements-panel')).toBeVisible()
    await expect(page.getByTestId('workflow-template-business-flow')).toBeVisible()
    await expect(page.getByTestId('workflow-template-kpi-tree')).toBeVisible()
    await expect(page.getByText('Workflow Canvas')).toBeVisible()
    await expect(page.locator('.react-flow__controls')).toBeVisible()

    await page.getByTestId('view-docs').click()

    await expect(page).toHaveURL(/\/docs$/)
    await expect(page.getByRole('heading', { name: 'Docs' })).toBeVisible()

    await page.getByTestId('view-chat').click()

    await expect(page).toHaveURL(/\/chat$/)
    await expect(page.getByRole('heading', { name: 'Chat', exact: true })).toBeVisible()
    await expect(page.getByText('Library Chat')).toBeVisible()
  })

  test('supports global keyboard shortcuts for fast navigation and creation', async ({ page }) => {
    await page.goto('/databases')

    await expect(page.getByTestId('open-create-record').locator('kbd').filter({ hasText: 'C' })).toBeVisible()
    await expect(page.locator('kbd').filter({ hasText: '/' }).first()).toBeVisible()

    await page.keyboard.press('ControlOrMeta+F')
    await expect(page.getByTestId('records-global-filter')).toBeFocused()

    await page.keyboard.press('Escape')
    await page.keyboard.press('ControlOrMeta+B')
    await expect(page).toHaveURL(/view=.*board/)
    await expect(page.getByText('drag to move')).toBeVisible()

    await page.keyboard.press('g')
    await page.keyboard.press('c')
    await expect(page).toHaveURL(/\/chat$/)
    await expect(page.getByRole('heading', { name: 'Chat', exact: true })).toBeVisible()

    await page.keyboard.press('c')
    await expect(page).toHaveURL(/\/databases/)
    await expect(page.getByTestId('create-record-modal')).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.getByTestId('create-record-modal')).toHaveCount(0)

    await page.keyboard.press('ControlOrMeta+K')
    await expect(page.getByTestId('keyboard-shortcuts-panel')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Command Menu' })).toBeVisible()
    await expect(page.getByTestId('keyboard-shortcuts-panel').locator('kbd').filter({ hasText: 'G' }).first()).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.getByTestId('keyboard-shortcuts-panel')).toHaveCount(0)

    await page.keyboard.press('Shift+/')
    await expect(page.getByTestId('keyboard-shortcuts-panel')).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.getByTestId('keyboard-shortcuts-panel')).toHaveCount(0)
  })

  test('adds database items to the workflow canvas', async ({ page }) => {
    test.setTimeout(90_000)

    const canvasDatabase = `workflow-e2e-${Date.now()}`

    await page.goto(`/databases/workflow?database=${canvasDatabase}`)

    await page.getByTestId('workflow-add-record').first().click()
    await page.getByTestId('workflow-template-kpi-tree').click()
    await page.getByTestId('workflow-add-record').nth(1).click()

    await expect(page).toHaveURL(/database=workflow-e2e-\d+/)
    await expect(page).toHaveURL(/view=.*workflow/)
    await expect(page.getByRole('heading', { name: 'All repository data' })).toBeVisible()
    await expect(page.getByTestId('workflow-node-record')).toHaveCount(2)
    await expect(page.getByText('KPI tree item')).toBeVisible()

    const sourceHandle = page.getByTestId('workflow-handle-source').first()
    const targetHandle = page.getByTestId('workflow-handle-target').nth(1)
    const sourceBox = await sourceHandle.boundingBox()
    const targetBox = await targetHandle.boundingBox()
    expect(sourceBox).toBeTruthy()
    expect(targetBox).toBeTruthy()
    if (sourceBox && targetBox) {
      await sourceHandle.click()
      await targetHandle.click()
      await expect(page.locator('.react-flow__edge')).toHaveCount(1)
    }
    await expect(page.getByTestId('workflow-persistence-status')).toHaveAttribute(
      'data-saved-signature',
      'kpi-tree:2:1'
    )
    await page.waitForTimeout(500)

    await page.reload()
    await expect(page.getByTestId('workflow-node-record')).toHaveCount(2)
    await expect(page.locator('.react-flow__edge')).toHaveCount(1)
    await expect(page.getByText('KPI tree item')).toBeVisible()

    await page.locator('.react-flow__node').first().dblclick()
    await expect(page.getByTestId('detail-panel')).toBeVisible()
    await expect(page.getByTestId('detail-panel').locator('.font-mono').filter({ hasText: /PLT-/ })).toBeVisible()
    await page.getByTestId('detail-panel').locator('h2').click()
    await expect(page.getByTestId('detail-panel').locator('input').first()).toBeVisible()
    await page.keyboard.press('Escape')
    await page.getByTestId('detail-panel-close').click()
    await expect(page.getByTestId('detail-panel')).toHaveCount(0)

    const firstNode = page.locator('.react-flow__node').first()
    const beforeDrag = await firstNode.boundingBox()
    expect(beforeDrag).toBeTruthy()
    if (beforeDrag) {
      await page.mouse.move(beforeDrag.x + beforeDrag.width / 2, beforeDrag.y + beforeDrag.height / 2)
      await page.mouse.down()
      await page.mouse.move(beforeDrag.x + beforeDrag.width / 2 + 220, beforeDrag.y + beforeDrag.height / 2 + 100, {
        steps: 8,
      })
      await page.mouse.up()
      const afterDrag = await firstNode.boundingBox()
      expect(Math.abs((afterDrag?.x ?? beforeDrag.x) - beforeDrag.x)).toBeGreaterThan(10)
    }

    await page.getByTestId('toggle-workflow-items').click()
    await expect(page.getByTestId('workflow-elements-panel')).toHaveCount(0)
    await expect(page.getByRole('button', { name: 'Items' })).toBeVisible()
    await page.getByTestId('toggle-workflow-items').click()
    await expect(page.getByTestId('workflow-elements-panel')).toBeVisible()

    await page.getByTestId('toggle-side-nav').click()
    await expect(page.getByTestId('view-home').getByText('Home')).toBeHidden()
    await expect(page.getByRole('button', { name: 'Expand sidebar' })).toBeVisible()
    await page.getByTestId('toggle-side-nav').click()
    await expect(page.getByTestId('view-home').getByText('Home')).toBeVisible()
  })

  test('syncs workflow canvas changes between browser tabs', async ({ page, context }) => {
    const canvasDatabase = `workflow-sync-${Date.now()}`
    const secondPage = await context.newPage()

    await page.goto(`/databases/workflow?database=${canvasDatabase}`)
    await secondPage.goto(`/databases/workflow?database=${canvasDatabase}`)
    await expect(page.getByTestId('sync-presence-status')).toHaveText(/([2-9]|\d{2,}) online/, {
      timeout: 15_000,
    })
    await expect(secondPage.getByTestId('sync-presence-status')).toHaveText(/([2-9]|\d{2,}) online/, {
      timeout: 15_000,
    })

    await page.getByTestId('workflow-add-record').first().click()
    await expect(page.getByTestId('workflow-persistence-status')).toHaveAttribute(
      'data-saved-signature',
      'business-flow:1:0',
      { timeout: 15_000 }
    )

    await expect(secondPage.locator('.react-flow__node')).toHaveCount(1, {
      timeout: 15_000,
    })

    const secondNode = secondPage.locator('.react-flow__node').first()
    const beforeTransform = await secondNode.getAttribute('style')
    const firstNode = page.locator('.react-flow__node').first()
    const beforeDrag = await firstNode.boundingBox()
    expect(beforeDrag).toBeTruthy()

    if (beforeDrag) {
      await page.mouse.move(beforeDrag.x + beforeDrag.width / 2, beforeDrag.y + beforeDrag.height / 2)
      await page.mouse.down()
      await page.mouse.move(beforeDrag.x + beforeDrag.width / 2 + 180, beforeDrag.y + beforeDrag.height / 2 + 80, {
        steps: 8,
      })
      await page.mouse.up()
    }

    await expect
      .poll(async () => secondNode.getAttribute('style'), { timeout: 15_000 })
      .not.toBe(beforeTransform)

    await secondPage.close()
  })

  test('preserves the selected database when switching database views', async ({ page }) => {
    await mockLibraryRepository(page)
    await page.goto('/databases')

    await page.getByTestId('database-quantum-box/photon-core').click()

    await expect(page).toHaveURL(/\/repositories\/quantum-box\/photon-core$/)
    await expect(page.getByTestId('repository-page')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'photon-core', exact: true })).toBeVisible()
    await expect(page.getByText('Library E2E repository')).toBeVisible()
    await expect(page.getByTestId('database-quantum-box/photon-core')).toHaveAttribute(
      'aria-current',
      'page',
    )

    await page.getByTestId('repository-open-data').click()

    await expect(page).toHaveURL(/database=quantum-box%2Fphoton-core/)
    await expect(page).toHaveURL(/view=.*table/)
    await expect(page.getByTestId('selected-database-pill')).toHaveText('Repository data')

    await page.getByTestId('view-kanban').click()

    await expect(page).toHaveURL(/database=quantum-box%2Fphoton-core/)
    await expect(page).toHaveURL(/view=.*board/)
    await expect(page.getByText('drag to move')).toBeVisible()
    await expect(page.getByTestId('selected-database-pill')).toHaveText('Repository data')

    await page.getByTestId('view-table').click()

    await expect(page).toHaveURL(/database=quantum-box%2Fphoton-core/)
    await expect(page).toHaveURL(/view=.*table/)
    await expect(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible()

    await page.getByTestId('view-workflow').click()

    await expect(page).toHaveURL(/database=quantum-box%2Fphoton-core/)
    await expect(page).toHaveURL(/view=.*workflow/)
    await expect(page.getByTestId('workflow-canvas')).toBeVisible()
    await expect(page.getByTestId('selected-database-pill')).toHaveText('Repository data')
  })

  test('opens an organization overview and continues into its repository', async ({ page }) => {
    await mockLibraryRepository(page)
    await page.goto('/organizations/quantum-box')

    await expect(page.getByTestId('organization-page')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'quantum-box', exact: true })).toBeVisible()
    await expect(page.getByText('tn_01j702qf86pc2j35s0kv0gv3gy')).toBeVisible()
    await expect(page.getByTestId('organization-repository-quantum-box/photon-core')).toBeVisible()

    await page.getByTestId('organization-repository-quantum-box/photon-core').click()
    await expect(page).toHaveURL(/\/repositories\/quantum-box\/photon-core$/)
    await expect(page.getByTestId('repository-page')).toBeVisible()
  })

  test('creates, renames, duplicates, and deletes named database views', async ({ page }) => {
    await page.goto('/databases')

    await page.getByTestId('new-board-view').click()
    await expect(page).toHaveURL(/view=.*board/)
    await expect(page.getByRole('button', { name: /New Board/ })).toBeVisible()

    page.once('dialog', async (dialog) => {
      await dialog.accept('Saved Board')
    })
    await page.getByTestId('view-options').click()
    await page.getByTestId('rename-view').click()
    await expect(page.getByRole('button', { name: /Saved Board/ })).toBeVisible()

    await page.getByTestId('duplicate-view').click()
    await expect(page.getByRole('button', { name: /Saved Board Copy/ })).toBeVisible()

    await page.getByTestId('delete-view').click()
    await expect(page.getByRole('button', { name: /Saved Board Copy/ })).toHaveCount(0)
    await expect(page.getByRole('button', { name: /Saved Board/ })).toBeVisible()
  })

  test('uses repository status filters from the right panel', async ({ page }) => {
    await page.goto('/databases')

    await page.getByTestId('toggle-database-filters').click()
    await expect(page.getByTestId('database-filter-panel')).toBeVisible()
    await page.getByRole('button', { name: /Todo/ }).click()
    await expect(page.getByTestId('status-filter-pill')).toHaveText(/Todo/)
    await expect(page.getByTestId('save-view')).toBeVisible()
    await page.getByTestId('save-view').click()
    await expect(page.getByTestId('save-view')).toHaveCount(0)
  })

  test('opens a database context menu from the sidebar', async ({ page }) => {
    await mockLibraryRepository(page)
    await page.goto('/databases')

    await expect(page.getByTestId('nav-databases')).toHaveCount(0)
    await page.getByTestId('database-quantum-box/photon-core').hover()
    await page.getByTestId('database-actions-quantum-box/photon-core').click()

    await expect(page.getByTestId('database-context-menu')).toBeVisible()
    await expect(page.getByTestId('database-context-menu').getByText('quantum-box/photon-core')).toBeVisible()

    await page.getByTestId('database-context-open').click()
    await expect(page).toHaveURL(/\/repositories\/quantum-box\/photon-core$/)
    await expect(page.getByTestId('repository-page')).toBeVisible()
    await expect(page.getByTestId('database-context-menu')).toHaveCount(0)
  })

  test('shows a repository not-found state for an unavailable path', async ({ page }) => {
    await mockLibraryRepository(page)
    await page.goto('/repositories/quantum-box/missing-repository')

    await expect(page.getByRole('heading', { name: 'Repository not found' })).toBeVisible()
    await expect(page.getByText('quantum-box/missing-repository is not available')).toBeVisible()
  })

  test('shows sync presence as clients connect', async ({ page, context }) => {
    await page.goto('/databases')

    await expect(page.getByTestId('sync-presence-status')).toHaveText(/\d+ online/)
    const initialOnlineText = await page.getByTestId('sync-presence-status').innerText()
    const initialOnlineCount = Number(initialOnlineText.split(' ')[0])

    const secondPage = await context.newPage()
    await secondPage.goto('/databases')

    await expect
      .poll(
        async () => Number((await secondPage.getByTestId('sync-presence-status').innerText()).split(' ')[0]),
        { timeout: 15_000 }
      )
      .toBeGreaterThanOrEqual(initialOnlineCount + 1)
    await expect
      .poll(
        async () => Number((await page.getByTestId('sync-presence-status').innerText()).split(' ')[0]),
        { timeout: 15_000 }
      )
      .toBeGreaterThanOrEqual(initialOnlineCount + 1)

    await secondPage.close()
  })

  test('syncs record creation between browser tabs', async ({ page, context }) => {
    const title = `E2E synced record ${Date.now()}`

    await page.goto('/databases')
    const secondPage = await context.newPage()
    await secondPage.goto('/databases')

    await page.getByTestId('open-create-record').click()
    await page.getByLabel(/Record title/i).fill(title)
    await page.getByLabel('Description').fill('Created in the first tab and observed in the second tab')
    await page.getByTestId('create-record-submit').click()
    await expect(page.getByTestId('create-record-modal')).toBeHidden()

    await expect(async () => {
      await secondPage.reload()
      await secondPage.getByPlaceholder('Filter records...').fill(title)
      await expect(secondPage.getByText(title).first()).toBeVisible({ timeout: 15_000 })
    }).toPass({ timeout: 60_000 })

    await secondPage.close()
  })

  test('sends a chat prompt and streams an assistant response', async ({ page }) => {
    await page.goto('/chat')

    await page.getByTestId('chat-message-input').fill('search for React 19')
    await page.getByTestId('chat-send').click()

    await expect(page.getByText('You', { exact: true })).toBeVisible()
    await expect(page.getByText('search for React 19')).toBeVisible()
    await expect(page.getByText('Assistant')).toBeVisible()
    await expect(page.getByText('Web Search')).toBeVisible()
    await expect(page.getByText(/Based on the search results|Here's a summary/)).toBeVisible({
      timeout: 15_000,
    })
    await expect(page.getByText(/Key Findings|Recommendations/)).toBeVisible()
  })

  test('syncs chat attachment metadata back into the workspace view', async ({ page }) => {
    const filename = `chat-attachment-${Date.now()}.pdf`

    await page.goto('/chat')
    const chooserPromise = page.waitForEvent('filechooser')
    await page.getByTestId('chat-attach-file').click()
    const chooser = await chooserPromise
    await chooser.setFiles({
      name: filename,
      mimeType: 'application/pdf',
      buffer: Buffer.from('%PDF-1.4\n% Photon attachment metadata smoke\n'),
    })
    await expect(page.getByText(filename)).toBeVisible()

    await page.getByTestId('chat-send').click()
    await expect(page.getByText(filename)).toBeVisible({ timeout: 15_000 })

    await page.getByTestId('side-nav').getByRole('button', { name: /All repositories/ }).click()
    await expect(page).toHaveURL(/\/databases/)
    await page.getByTestId('view-chat').click()

    await expect(page.getByTestId('chat-workspace-attachments').getByText(filename)).toBeVisible({
      timeout: 15_000,
    })
  })

  test('creates and searches records from chat tools', async ({ page }) => {
    const title = `Chat command record ${Date.now()}`

    await page.goto('/chat')
    await page.getByTestId('chat-message-input').fill(`create record "${title}"`)
    await page.getByTestId('chat-send').click()

    await expect(page.getByTestId('record-tool-result').getByText('Create Record')).toBeVisible()
    await expect(page.getByTestId('record-tool-result').getByText(title)).toBeVisible({
      timeout: 15_000,
    })
    await expect(page.getByText('Created PLT-')).toBeVisible()
    const createResultText = await page.getByTestId('record-tool-result').innerText()
    const identifier = createResultText.match(/PLT-\d+/)?.[0]
    expect(identifier).toBeTruthy()
    const recordIdentifier = identifier ?? ''

    await page.getByTestId('chat-message-input').fill(`move ${recordIdentifier} to done`)
    await page.getByTestId('chat-send').click()

    await expect(page.getByTestId('record-tool-result').last().getByText('Move Record')).toBeVisible()
    await expect(page.getByTestId('record-tool-result').last().getByText('Done')).toBeVisible({
      timeout: 15_000,
    })

    await page.getByTestId('side-nav').getByRole('button', { name: /All repositories/ }).click()
    await page.getByPlaceholder('Filter records...').fill(title)
    await expect(page.getByText(title)).toBeVisible()

    await page.getByTestId('view-chat').click()
    await page.getByTestId('chat-message-input').fill(`search record "${title}"`)
    await page.getByTestId('chat-send').click()

    await expect(page.getByTestId('record-tool-result').last().getByText('Database Search')).toBeVisible()
    await expect(page.getByTestId('record-tool-result').last().getByText(title)).toBeVisible({
      timeout: 15_000,
    })
  })

  test('shows chat-created records in the database table', async ({ page }) => {
    const title = `Detail command record ${Date.now()}`

    await page.goto('/chat')
    await page.getByTestId('chat-message-input').fill(`create record "${title}"`)
    await page.getByTestId('chat-send').click()

    await expect(page.getByTestId('record-tool-result').getByText('Create Record')).toBeVisible()
    await expect(page.getByTestId('record-tool-result').getByText(title)).toBeVisible({
      timeout: 15_000,
    })

    const createResultText = await page.getByTestId('record-tool-result').innerText()
    const identifier = createResultText.match(/PLT-\d+/)?.[0]
    expect(identifier).toBeTruthy()
    const recordIdentifier = identifier ?? ''

    await page.getByTestId('chat-message-input').fill(`move ${recordIdentifier} to done`)
    await page.getByTestId('chat-send').click()

    await expect(page.getByTestId('record-tool-result').last().getByText('Move Record')).toBeVisible()
    await expect(page.getByTestId('record-tool-result').last().getByText('Done')).toBeVisible({
      timeout: 15_000,
    })

    await page.getByTestId('side-nav').getByRole('button', { name: /All repositories/ }).click()
    await page.getByPlaceholder('Filter records...').fill(title)
    await expect(page.getByText(title).first()).toBeVisible({ timeout: 15_000 })
    await expect(page.locator('tbody tr', { hasText: recordIdentifier }).first()).toBeVisible()
  })

  test('creates a doc and syncs Yjs blocks from a shared document URL', async ({ page, browser }) => {
    const title = `E2E local doc ${Date.now()}`

    await page.goto('/docs')
    await page.getByTestId('create-doc').click()

    await expect(page).toHaveURL(/\/documents\/[^/]+$/, { timeout: 20_000 })
    await expect(page.getByText('Server connected')).toBeVisible()
    await page.getByLabel('Document title').fill(title)
    await page.keyboard.press('Tab')
    await expect(page.getByRole('link', { name: new RegExp(title) })).toBeVisible()
    const editor = page.locator('.bn-editor[contenteditable="true"]')
    await editor.click()
    await page.keyboard.type('Reload proof body')
    await page.waitForTimeout(500)

    const documentUrl = page.url()
    const sharedContext = await browser.newContext({ storageState: e2eAuthState })
    const sharedPage = await sharedContext.newPage()
    await sharedPage.goto(documentUrl)

    await expect(sharedPage.getByText('Server connected')).toBeVisible({ timeout: 20_000 })
    await expect(sharedPage.getByText('Reload proof body')).toBeVisible({ timeout: 20_000 })

    await editor.click()
    await page.keyboard.press(process.platform === 'darwin' ? 'Meta+A' : 'Control+A')
    await page.keyboard.type('Synced from first browser')
    await expect(sharedPage.getByText('Synced from first browser')).toBeVisible()
    await sharedContext.close()

    await page.reload()

    await expect(page.getByLabel('Document title')).toHaveValue(title)
    await expect(page.getByText('Synced from first browser')).toBeVisible()
  })

  test('reconnects a document after an offline edit and syncs it to another client', async ({ browser }) => {
    test.setTimeout(180_000)

    const title = `E2E reconnect doc ${Date.now()}`
    const initialText = `Online baseline ${Date.now()}`
    const offlineText = `Offline reconnect proof ${Date.now()}`

    const editingContext = await browser.newContext({ storageState: e2eAuthState })
    const editingPage = await editingContext.newPage()
    await editingPage.goto('/docs')
    await editingPage.getByTestId('create-doc').click()

    await expect(editingPage).toHaveURL(/\/documents\/[^/]+$/, { timeout: 20_000 })
    await expect(editingPage.getByText('Server connected')).toBeVisible()
    await editingPage.getByLabel('Document title').fill(title)
    await editingPage.keyboard.press('Tab')

    const editor = editingPage.locator('.bn-editor[contenteditable="true"]')
    await editor.fill(initialText)
    await expect(editingPage.getByText(initialText)).toBeVisible({ timeout: 10_000 })

    const documentUrl = editingPage.url()
    const verifierContext = await browser.newContext({ storageState: e2eAuthState })
    const verifierPage = await verifierContext.newPage()
    await verifierPage.goto(documentUrl)
    await expect(verifierPage.getByText('Server connected')).toBeVisible({ timeout: 20_000 })
    await expect(verifierPage.getByText(initialText)).toBeVisible({ timeout: 20_000 })

    await editingContext.setOffline(true)
    await editingPage.evaluate(() => window.__photonTestHooks?.closeDocumentSockets?.())
    await expect(editingPage.getByText(/Server connecting|Local only/)).toBeVisible({ timeout: 15_000 })

    await editor.click()
    await editingPage.keyboard.press(process.platform === 'darwin' ? 'Meta+End' : 'Control+End')
    await editingPage.keyboard.type(` ${offlineText}`)
    await expect(editingPage.getByText(offlineText)).toBeVisible({ timeout: 10_000 })

    await editingContext.setOffline(false)
    await editingPage.evaluate(() => window.dispatchEvent(new Event('online')))
    await expect(editingPage.getByText('Server connected')).toBeVisible({ timeout: 20_000 })
    await expect(async () => {
      await verifierPage.goto(documentUrl)
      await expect(verifierPage.getByText('Server connected')).toBeVisible({ timeout: 20_000 })
      await expect(verifierPage.getByText(offlineText)).toBeVisible({ timeout: 20_000 })
    }).toPass({ timeout: 120_000 })

    await verifierContext.close()
    await editingContext.close()
  })

  test('links docs and records from selected editor text', async ({ page }) => {
    const title = `E2E linked doc ${Date.now()}`
    const selectedText = `Selected follow-up ${Date.now()}`

    await page.goto('/docs')
    await page.getByTestId('create-doc').click()
    await expect(page).toHaveURL(/\/documents\/[^/]+$/, { timeout: 20_000 })
    await page.getByLabel('Document title').fill(title)
    await page.keyboard.press('Tab')

    const editor = page.locator('.bn-editor[contenteditable="true"]')
    await editor.click()
    await page.keyboard.type(selectedText)
    await editor.evaluate((element, text) => {
      const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT)
      let target: Text | null = null
      let startOffset = 0
      while (walker.nextNode()) {
        const node = walker.currentNode as Text
        const index = node.data.indexOf(text)
        if (index >= 0) {
          target = node
          startOffset = index
          break
        }
      }
      if (!target) throw new Error(`Unable to find editor text: ${text}`)
      const range = document.createRange()
      range.setStart(target, startOffset)
      range.setEnd(target, startOffset + text.length)
      const selection = window.getSelection()
      selection?.removeAllRanges()
      selection?.addRange(range)
      document.dispatchEvent(new Event('selectionchange', { bubbles: true }))
    }, selectedText)

    await expect(page.getByTestId('doc-selected-text').getByText(selectedText)).toBeVisible()
    await page.getByTestId('doc-create-record-from-selection').click()

    const relatedDatabases = page.getByTestId('doc-related-records')
    await expect(relatedDatabases.getByText(/PLT-\d+/)).toBeVisible({ timeout: 15_000 })
    const recordIdentifier = (await relatedDatabases.innerText()).match(/PLT-\d+/)?.[0]
    expect(recordIdentifier).toBeTruthy()

    await page.goto(`/databases/${recordIdentifier}`)
    await expect(page.getByTestId('record-related-docs').getByText(title)).toBeVisible({
      timeout: 15_000,
    })

    await page.getByTestId('view-chat').click()
    await expect(page.getByTestId('chat-document-context').getByText(title)).toBeVisible()
    await expect(page.getByTestId('chat-document-context').getByText('1 related records')).toBeVisible()
  })
})
