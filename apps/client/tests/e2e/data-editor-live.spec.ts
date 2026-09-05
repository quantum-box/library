import { test, expect } from './test-fixtures'
import { e2eAuthState } from './auth-state'
import * as Y from 'yjs'

const api = 'http://127.0.0.1:50063'
const route = '/quantum-box/photon-core/data/seed-data-201'
const seed = 'Seed data for deterministic E2E coverage.'
const live = 'http://127.0.0.1:8788'
const otherAuthState = structuredClone(e2eAuthState)
otherAuthState.origins[0].localStorage[0].value = JSON.stringify({
  ...JSON.parse(otherAuthState.origins[0].localStorage[0].value),
  accessToken: 'dev:ren',
  userId: 'library-e2e-ren',
  email: 'ren@local.test',
  username: 'Ren',
})

test('the Live edge rejects missing credentials, forbidden origins and non-body properties', async ({ request }) => {
  const data = { org: 'quantum-box', repo: 'photon-core', data_id: 'seed-data-201', property_id: 'prop-description' }
  const anonymous = await request.post(`${live}/live/session`, {
    headers: { Origin: 'http://127.0.0.1:5187' }, data,
  })
  expect([401, 403]).toContain(anonymous.status())
  const forbiddenOrigin = await request.post(`${live}/live/session`, {
    headers: { Origin: 'https://unrelated.example', Authorization: 'Bearer dev:local' }, data,
  })
  expect(forbiddenOrigin.status()).toBe(403)
  const wrongProperty = await request.post(`${live}/live/session`, {
    headers: { Origin: 'http://127.0.0.1:5187', Authorization: 'Bearer dev:local' },
    data: { ...data, property_id: 'prop-assignee' },
  })
  expect(wrongProperty.status()).toBe(404)
})

for (const format of ['markdown', 'richText'] as const) {
  test(`two independent browsers merge ${format} and checkpoint without duplicating the seed`, async ({ browser, page }) => {
    if (format === 'richText') {
      const property = await page.request.post(`${api}/v1/graphql`, {
        data: {
          query: 'mutation LibraryClientUpdateRepositoryProperty { updateProperty { id } }',
          variables: {
            id: 'prop-description',
            input: { orgUsername: 'quantum-box', repoUsername: 'photon-core', propertyName: 'Body', propertyType: 'RichText' },
          },
        },
      })
      expect(property.ok()).toBe(true)
      const body = await page.request.post(`${api}/v1/graphql`, {
        data: {
          query: 'mutation LibraryClientUpdateData { updateData { id } }',
          variables: {
            input: {
              orgUsername: 'quantum-box', repoUsername: 'photon-core', dataId: 'seed-data-201',
              propertyData: [{ propertyId: 'prop-description', value: {
                richText: JSON.stringify([{ type: 'paragraph', content: [{ type: 'text', text: seed, styles: {} }] }]),
              } }],
            },
          },
        },
      })
      expect(body.ok()).toBe(true)
    }
    const otherContext = await browser.newContext({ storageState: otherAuthState })
    const other = await otherContext.newPage()
    try {
      await Promise.all([page.goto(route), other.goto(route)])
      const firstEditor = page.locator('.record-body-blocknote [contenteditable="true"]').first()
      const secondEditor = other.locator('.record-body-blocknote [contenteditable="true"]').first()
      await expect(firstEditor).toContainText(seed)
      await expect(secondEditor).toContainText(seed)
      expect((await firstEditor.innerText()).split(seed)).toHaveLength(2)
      expect((await secondEditor.innerText()).split(seed)).toHaveLength(2)

      await firstEditor.click()
      await page.keyboard.press('ControlOrMeta+End')
      await secondEditor.click()
      await other.keyboard.press('ControlOrMeta+End')
      await Promise.all([
        page.keyboard.insertText(' Aoi contribution.'),
        other.keyboard.insertText(' Ren contribution.'),
      ])
      for (const editor of [firstEditor, secondEditor]) {
        await expect(editor).toContainText('Aoi contribution.')
        await expect(editor).toContainText('Ren contribution.')
        const text = await editor.innerText()
        expect(text.split('Aoi contribution.')).toHaveLength(2)
        expect(text.split('Ren contribution.')).toHaveLength(2)
      }
      await expect.poll(async () => {
        const state = await (await page.request.get(`${api}/__e2e/state`)).json()
        return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
          .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value[format] as string
      }).toContain('Aoi contribution.')
      await expect.poll(async () => {
        const state = await (await page.request.get(`${api}/__e2e/state`)).json()
        return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
          .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value[format] as string
      }).toContain('Ren contribution.')

      // Choosing the same room string on the legacy, unauthenticated endpoint
      // must not reach the data editor's dedicated Durable Object namespace.
      const exchange = await page.request.post(`${live}/live/session`, {
        headers: { Origin: 'http://127.0.0.1:5187', Authorization: 'Bearer dev:local' },
        data: { org: 'quantum-box', repo: 'photon-core', data_id: 'seed-data-201', property_id: 'prop-description' },
      })
      expect(exchange.ok()).toBe(true)
      const session = await exchange.json() as { room_id: string }
      const bytes = await page.evaluate((roomId) => new Promise<number[]>((resolve, reject) => {
        const socket = new WebSocket(`ws://127.0.0.1:8788/ws?room=${encodeURIComponent(roomId)}`)
        socket.binaryType = 'arraybuffer'
        socket.onerror = () => reject(new Error('Legacy socket failed'))
        socket.onmessage = (event) => {
          if (!(event.data instanceof ArrayBuffer)) return
          resolve(Array.from(new Uint8Array(event.data)))
          socket.close()
        }
      }), session.room_id)
      const unrelatedDoc = new Y.Doc()
      Y.applyUpdate(unrelatedDoc, new Uint8Array(bytes))
      expect(unrelatedDoc.getXmlFragment('prosemirror').length).toBe(0)
      unrelatedDoc.destroy()

      await other.reload()
      await expect(other.locator('.record-body-blocknote [contenteditable="true"]').first()).toContainText('Aoi contribution.')
      await expect(other.locator('.record-body-blocknote [contenteditable="true"]').first()).toContainText('Ren contribution.')
      await otherContext.setOffline(true)
      await expect(other.locator('.record-body-blocknote [contenteditable="true"]')).toHaveCount(0)
      await firstEditor.click()
      await page.keyboard.press('ControlOrMeta+End')
      await page.keyboard.insertText(' Reconnected contribution.')
      await otherContext.setOffline(false)
      await expect(other.locator('.record-body-blocknote [contenteditable="true"]').first()).toContainText('Reconnected contribution.')
      await expect.poll(async () => {
        const state = await (await page.request.get(`${api}/__e2e/state`)).json()
        return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
          .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value[format] as string
      }).toContain('Reconnected contribution.')
      await expect(page.getByTestId('data-editor-live-status')).toHaveText('Shared body saved')
      await page.screenshot({ path: `test-results/data-editor-live-${format}.png`, fullPage: true })
    } finally {
      try {
        const fixtureState = await page.request.get(`${api}/__e2e/state`, { timeout: 3_000 })
        await test.info().attach('live-fixture-state', {
          body: await fixtureState.body(),
          contentType: 'application/json',
        })
        if (test.info().status !== test.info().expectedStatus) {
          console.log('Live fixture diagnostics:', await fixtureState.text())
        }
      } catch {
        // A timed-out test may already have closed its API context. Preserve
        // the original failure rather than replacing it with diagnostics.
      }
      await otherContext.close()
    }
  })
}

test('a different data record is isolated from the live body', async ({ browser, page }) => {
  const context = await browser.newContext({ storageState: e2eAuthState })
  const other = await context.newPage()
  try {
    await Promise.all([
      page.goto(route),
      other.goto('/quantum-box/photon-core/data/seed-data-202'),
    ])
    const editor = page.locator('.record-body-blocknote [contenteditable="true"]').first()
    const otherEditor = other.locator('.record-body-blocknote [contenteditable="true"]').first()
    await expect(editor).toBeVisible()
    await expect(otherEditor).toBeVisible()
    await editor.click()
    await page.keyboard.press('ControlOrMeta+End')
    await page.keyboard.insertText(' Only the first record.')
    await expect(editor).toContainText('Only the first record.')
    await expect(otherEditor).not.toContainText('Only the first record.')
    await other.reload()
    await expect(other.locator('.record-body-blocknote [contenteditable="true"]').first()).not.toContainText('Only the first record.')
  } finally {
    await context.close()
  }
})

test('editing a normal property preserves the body and allows the next shared checkpoint', async ({ page }) => {
  await page.goto(route)
  const editor = page.locator('.record-body-blocknote [contenteditable="true"]').first()
  await expect(editor).toContainText(seed)
  await page.getByTestId('library-editable-cell-prop-assignee').click()
  const input = page.getByTestId('library-editable-input-prop-assignee')
  await input.fill('Aoi')
  await input.press('Enter')
  await expect.poll(async () => {
    const state = await (await page.request.get(`${api}/__e2e/state`)).json()
    return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
      .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-assignee')?.value.string
  }).toBe('Aoi')
  await editor.click()
  await page.keyboard.press('ControlOrMeta+End')
  await page.keyboard.insertText(' Body after property change.')
  await expect.poll(async () => {
    const state = await (await page.request.get(`${api}/__e2e/state`)).json()
    return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
      .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value.markdown
  }).toContain('Body after property change.')
  await page.reload()
  await expect(page.getByTestId('library-editable-cell-prop-assignee')).toHaveText('Aoi')
  await expect(page.locator('.record-body-blocknote [contenteditable="true"]').first()).toContainText('Body after property change.')
})

for (const format of ['markdown', 'richText'] as const) {
  test(`refreshes an idle ${format} room from an externally changed canonical body`, async ({ browser, page }) => {
    const externalBodyText = 'External canonical body after the room was idle.'
    const previousContribution = ' Saved before the room became idle.'
    const recoveryContribution = ' Recovery contribution after reopening.'
    const richTextValue = (text: string) => JSON.stringify([{
      type: 'paragraph',
      content: [{ type: 'text', text, styles: {} }],
    }])
    const canonicalBody = async (): Promise<string> => {
      const state = await (await page.request.get(`${api}/__e2e/state`)).json()
      return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
        .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value[format] as string
    }

    if (format === 'richText') {
      const property = await page.request.post(`${api}/v1/graphql`, {
        data: {
          query: 'mutation LibraryClientUpdateRepositoryProperty { updateProperty { id } }',
          variables: {
            id: 'prop-description',
            input: {
              orgUsername: 'quantum-box',
              repoUsername: 'photon-core',
              propertyName: 'Body',
              propertyType: 'RichText',
            },
          },
        },
      })
      expect(property.ok()).toBe(true)
      const initialBody = await page.request.post(`${api}/v1/graphql`, {
        data: {
          query: 'mutation LibraryClientUpdateData { updateData { id } }',
          variables: {
            input: {
              orgUsername: 'quantum-box',
              repoUsername: 'photon-core',
              dataId: 'seed-data-201',
              propertyData: [{ propertyId: 'prop-description', value: { richText: richTextValue(seed) } }],
            },
          },
        },
      })
      expect(initialBody.ok()).toBe(true)
    }

    const initialContext = await browser.newContext({ storageState: e2eAuthState })
    try {
      const initialPage = await initialContext.newPage()
      await initialPage.goto(route)
      const initialEditor = initialPage.locator('.record-body-blocknote [contenteditable="true"]').first()
      await expect(initialEditor).toContainText(seed)
      await initialEditor.click()
      await initialPage.keyboard.press('ControlOrMeta+End')
      await initialPage.keyboard.insertText(previousContribution)
      await expect.poll(canonicalBody).toContain(previousContribution.trim())
    } finally {
      // The external write below must happen after every participant has left
      // so the next authorization exercises the worker's idle-room recovery.
      await initialContext.close()
    }

    const externalUpdate = await page.request.post(`${api}/v1/graphql`, {
      data: {
        query: 'mutation LibraryClientUpdateData { updateData { id } }',
        variables: {
          input: {
            orgUsername: 'quantum-box',
            repoUsername: 'photon-core',
            dataId: 'seed-data-201',
            propertyData: [{
              propertyId: 'prop-description',
              value: format === 'markdown'
                ? { markdown: externalBodyText }
                : { richText: richTextValue(externalBodyText) },
            }],
          },
        },
      },
    })
    expect(externalUpdate.ok()).toBe(true)
    await expect.poll(canonicalBody).toContain(externalBodyText)

    const reopenedContext = await browser.newContext({ storageState: e2eAuthState })
    try {
      const reopenedPage = await reopenedContext.newPage()
      await reopenedPage.goto(route)
      const reopenedEditor = reopenedPage.locator('.record-body-blocknote [contenteditable="true"]').first()
      await expect(reopenedEditor).toContainText(externalBodyText)
      await expect(reopenedEditor).not.toContainText(previousContribution.trim())
      await reopenedEditor.click()
      await reopenedPage.keyboard.press('ControlOrMeta+End')
      await reopenedPage.keyboard.insertText(recoveryContribution)
      await expect.poll(canonicalBody).toContain(recoveryContribution.trim())
      await expect(reopenedPage.getByTestId('data-editor-live-status')).toHaveText('Shared body saved')

      const nextContext = await browser.newContext({ storageState: otherAuthState })
      try {
        const nextPage = await nextContext.newPage()
        await nextPage.goto(route)
        const nextEditor = nextPage.locator('.record-body-blocknote [contenteditable="true"]').first()
        await expect(nextEditor).toContainText(externalBodyText)
        await expect(nextEditor).toContainText(recoveryContribution.trim())

        const existingParticipantContribution = ' Existing participant next edit.'
        const newParticipantContribution = ' New participant next edit.'
        await reopenedEditor.click()
        await reopenedPage.keyboard.press('ControlOrMeta+End')
        await nextEditor.click()
        await nextPage.keyboard.press('ControlOrMeta+End')
        await Promise.all([
          reopenedPage.keyboard.insertText(existingParticipantContribution),
          nextPage.keyboard.insertText(newParticipantContribution),
        ])
        for (const editor of [reopenedEditor, nextEditor]) {
          await expect(editor).toContainText(existingParticipantContribution.trim())
          await expect(editor).toContainText(newParticipantContribution.trim())
        }
        await expect.poll(canonicalBody).toContain(existingParticipantContribution.trim())
        await expect.poll(canonicalBody).toContain(newParticipantContribution.trim())
      } finally {
        await nextContext.close()
      }
    } finally {
      await reopenedContext.close()
    }
  })
}

test('does not merge a retained offline document into a replacement room', async ({ browser, page }) => {
  const initialText = ' Saved before going offline.'
  const externalText = 'Replacement canonical body created while the first participant was offline.'
  const oldContext = await browser.newContext({ storageState: e2eAuthState })
  const freshContext = await browser.newContext({ storageState: otherAuthState })
  const canonicalBody = async (): Promise<string> => {
    const state = await (await page.request.get(`${api}/__e2e/state`)).json()
    return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
      .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value.markdown
  }
  try {
    const oldPage = await oldContext.newPage()
    await oldPage.goto(route)
    const oldEditor = oldPage.locator('.record-body-blocknote [contenteditable="true"]').first()
    await expect(oldEditor).toContainText(seed)
    await oldEditor.click()
    await oldPage.keyboard.press('ControlOrMeta+End')
    await oldPage.keyboard.insertText(initialText)
    await expect.poll(canonicalBody).toContain(initialText.trim())
    await expect(oldPage.getByTestId('data-editor-live-status')).toHaveText('Shared body saved')
    await oldContext.setOffline(true)

    const update = await page.request.post(`${api}/v1/graphql`, {
      data: {
        query: 'mutation LibraryClientUpdateData { updateData { id } }',
        variables: { input: {
          orgUsername: 'quantum-box', repoUsername: 'photon-core', dataId: 'seed-data-201',
          propertyData: [{ propertyId: 'prop-description', value: { markdown: externalText } }],
        } },
      },
    })
    expect(update.ok()).toBe(true)
    const freshPage = await freshContext.newPage()
    await freshPage.goto(route)
    const freshEditor = freshPage.locator('.record-body-blocknote [contenteditable="true"]').first()
    await expect(freshEditor).toContainText(externalText)
    await oldContext.setOffline(false)
    await expect(oldPage.getByTestId('data-editor-live-status')).toHaveText('Shared body conflicts with a newer saved version')
    await expect(oldPage.locator('.record-body-blocknote')).toContainText(initialText.trim())
    await expect(oldPage.locator('.record-body-blocknote')).not.toContainText(externalText)
    await expect(oldPage.locator('.record-body-blocknote [contenteditable="true"]')).toHaveCount(0)
    await freshEditor.click()
    await freshPage.keyboard.press('ControlOrMeta+End')
    await freshPage.keyboard.insertText(' Fresh room stays writable.')
    await expect.poll(canonicalBody).toContain('Fresh room stays writable.')
    expect(await canonicalBody()).not.toContain(initialText.trim())
  } finally {
    await oldContext.close()
    await freshContext.close()
  }
})

test('keeps recovering when an external write restores an earlier body', async ({ browser, page }) => {
  const initialContext = await browser.newContext({ storageState: e2eAuthState })
  const initialPage = await initialContext.newPage()
  await initialPage.goto(route)
  await expect(initialPage.locator('.record-body-blocknote [contenteditable="true"]').first()).toContainText(seed)
  await initialContext.close()
  for (const [index, body] of ['External body A.', 'External body B.', 'External body A.'].entries()) {
    const result = await page.request.post(`${api}/v1/graphql`, {
      data: {
        query: 'mutation LibraryClientUpdateData { updateData { id } }',
        variables: { input: {
          orgUsername: 'quantum-box', repoUsername: 'photon-core', dataId: 'seed-data-201',
          propertyData: [{ propertyId: 'prop-description', value: { markdown: body } }],
        } },
      },
    })
    expect(result.ok()).toBe(true)
    const context = await browser.newContext({ storageState: e2eAuthState })
    try {
      const participant = await context.newPage()
      await participant.goto(route)
      const editor = participant.locator('.record-body-blocknote [contenteditable="true"]').first()
      await expect(editor).toContainText(body)
      await expect(editor).not.toContainText('Cycle edit')
      await editor.click()
      await participant.keyboard.press('ControlOrMeta+End')
      await participant.keyboard.insertText(` Cycle edit ${index}.`)
      await expect(participant.getByTestId('data-editor-live-status')).toHaveText('Shared body saved')
    } finally {
      await context.close()
    }
  }
})
