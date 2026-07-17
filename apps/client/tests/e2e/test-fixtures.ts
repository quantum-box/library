import { expect, test as base } from '@playwright/test'

const libraryApiUrl = 'http://127.0.0.1:50063'

export const test = base.extend<{ resetLibraryApi: void }>({
  resetLibraryApi: [async ({ request }, use) => {
    const response = await request.post(`${libraryApiUrl}/__e2e/reset`)
    expect(response.ok(), await response.text()).toBe(true)
    await use()
  }, { auto: true }],
})

export { expect }
