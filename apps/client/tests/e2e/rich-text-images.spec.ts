import { expect, test } from './test-fixtures'

const api = 'http://127.0.0.1:50063'
const route = '/quantum-box/photon-core/data/seed-data-201'
const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=', 'base64')

for (const method of ['upload', 'url'] as const) {
  test(`inserts a rich text image by ${method} and retains it after reload`, async ({ page }) => {
    await page.request.post(`${api}/v1/graphql`, {
      data: {
        query: 'mutation LibraryClientUpdateRepositoryProperty { updateProperty { id } }',
        variables: {
          id: 'prop-description',
          input: { orgUsername: 'quantum-box', repoUsername: 'photon-core', propertyName: 'Body', propertyType: 'RichText' },
        },
      },
    })
    await page.request.post(`${api}/v1/graphql`, {
      data: {
        query: 'mutation LibraryClientUpdateData { updateData { id } }',
        variables: {
          input: {
            orgUsername: 'quantum-box', repoUsername: 'photon-core', dataId: 'seed-data-201',
            propertyData: [{ propertyId: 'prop-description', value: { richText: JSON.stringify([
              { type: 'paragraph', content: [{ type: 'text', text: 'Seed data for image verification.', styles: {} }] },
            ]) } }],
          },
        },
      },
    })
    const imageUrl = `${api}/v1beta/repos/quantum-box/photon-core/images/test.png`
    let uploaded = false
    await page.route('**/v1beta/repos/quantum-box/photon-core/images**', async (requestRoute) => {
      const request = requestRoute.request()
      if (request.method() === 'POST') {
        expect(request.headers()['content-type']).toBe('image/png')
        expect(request.postDataBuffer()).toEqual(png)
        uploaded = true
        await requestRoute.fulfill({ json: { url: imageUrl } })
      } else {
        await requestRoute.fulfill({ contentType: 'image/png', body: png })
      }
    })
    await page.goto(route)
    const editor = page.locator('.record-body-blocknote')
    await expect(editor.locator('[contenteditable="true"]').first()).toContainText('Seed data')
    await editor.getByRole('button', { name: 'Insert image', exact: true }).click()
    if (method === 'upload') {
      await page.locator('.bn-panel input[type="file"]').setInputFiles({ name: 'test.png', mimeType: 'image/png', buffer: png })
    } else {
      await page.getByRole('tab', { name: 'Embed', exact: true }).click()
      await page.getByRole('textbox', { name: 'Enter URL' }).fill(imageUrl)
      await page.getByRole('button', { name: 'Embed image', exact: true }).click()
    }
    await expect(editor.locator(`img[src="${imageUrl}"]`)).toBeVisible()
    expect(uploaded).toBe(method === 'upload')
    await expect.poll(async () => {
      const state = await (await page.request.get(`${api}/__e2e/state`)).json()
      return state.data.find((item: { id: string }) => item.id === 'seed-data-201')
        .propertyData.find((entry: { propertyId: string }) => entry.propertyId === 'prop-description').value.richText
    }).toContain(imageUrl)
    await page.reload()
    await expect(editor.locator(`img[src="${imageUrl}"]`)).toBeVisible()
    await expect(editor.locator('[contenteditable="true"]').first()).toContainText('Seed data')
  })
}

test('a failed upload restores the image chooser and can be retried', async ({ page }) => {
  let attempts = 0
  const imageUrl = `${api}/v1beta/repos/quantum-box/photon-core/images/retry.png`
  await page.route('**/v1beta/repos/quantum-box/photon-core/images**', async (route) => {
    if (route.request().method() === 'POST') {
      attempts += 1
      await route.fulfill(attempts === 1
        ? { status: 500, json: { error: 'Storage unavailable' } }
        : { json: { url: imageUrl } })
    } else {
      await route.fulfill({ contentType: 'image/png', body: png })
    }
  })
  await page.goto('/quantum-box/photon-core/data/seed-data-201')
  const editor = page.locator('.record-body-blocknote')
  await editor.getByRole('button', { name: 'Insert image', exact: true }).click()
  const upload = page.locator('.bn-panel input[type="file"]')
  const file = { name: 'retry.png', mimeType: 'image/png', buffer: png }
  await upload.setInputFiles(file)
  await expect(editor.getByRole('alert')).toHaveText('Image upload failed. Choose the image again to retry.')
  await expect(editor.locator('.bn-file-loading-preview')).toHaveCount(0)
  await expect(editor.getByText('Add image', { exact: true })).toBeVisible()
  await expect(page.locator('.bn-error-text')).toBeHidden({ timeout: 6_000 })
  await expect(editor.getByRole('alert')).toBeVisible()
  await upload.setInputFiles(file)
  await expect(editor.locator(`img[src="${imageUrl}"]`)).toBeVisible()
  await expect(editor.getByRole('alert')).toHaveCount(0)
  expect(attempts).toBe(2)
})
