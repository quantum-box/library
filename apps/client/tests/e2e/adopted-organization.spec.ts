import { expect, test } from './test-fixtures'

const api = 'http://127.0.0.1:50063'
const platform = 'tn_01adoptedplatform0000000000'

test.beforeEach(async ({ request }) => {
  expect((await request.post(`${api}/__e2e/adopted-organization`)).ok()).toBe(true)
})

test('rejects a mismatched platform as the upstream authorization service does', async ({ request }) => {
  for (const data of [
    { query: 'query LibraryClientRepoData { repo { id } }', variables: { org: 'adopted', repo: 'handbook' } },
    { query: 'mutation LibraryClientCreateRepository { createRepo { id } }', variables: { input: { orgUsername: 'adopted', repoUsername: 'rejected' } } },
  ]) {
    const response = await request.post(`${api}/v1/graphql`, {
      headers: { 'x-operator-id': 'org-adopted', 'x-platform-id': 'tn_01j702qf86pc2j35s0kv0gv3gy' },
      data,
    })
    expect(await response.json()).toMatchObject({ errors: [{ message: 'Forbidden: Upstream authorization rejected' }] })
  }
})

test('opens a repository in an adopted organization', async ({ page }) => {
  const requests: string[] = []
  page.on('request', (request) => {
    if (request.url().endsWith('/v1/graphql') && request.postData()?.includes('LibraryClientRepoData') && request.postData()?.includes('adopted')) {
      requests.push(request.headers()['x-platform-id'] ?? '')
    }
  })
  await page.goto('/organizations/adopted')
  await expect(page.getByTestId('organization-repository-adopted/handbook')).toBeVisible()
  await page.getByTestId('organization-repository-adopted/handbook').click()
  await expect(page).toHaveURL(/\/adopted\/handbook$/)
  await expect(page.getByTestId('repository-page')).toBeVisible()
  await expect.poll(() => requests).toContain(platform)
  await expect(page.getByText('Imported organization guide').first()).toBeVisible()
})

test('creates a repository in an adopted organization', async ({ page }) => {
  let createPlatform = ''
  page.on('request', (request) => {
    if (request.url().endsWith('/v1/graphql') && request.postData()?.includes('LibraryClientCreateRepository')) {
      createPlatform = request.headers()['x-platform-id'] ?? ''
    }
  })
  await page.goto('/organizations/adopted')
  await page.getByRole('button', { name: 'New repository' }).click()
  await page.getByLabel('Repository name').fill('Imported Research')
  await page.getByRole('button', { name: 'Create repository' }).click()
  await expect(page).toHaveURL(/\/adopted\/imported-research$/)
  await expect(page.getByTestId('repository-page')).toBeVisible()
  expect(createPlatform).toBe(platform)
  const state = await (await page.request.get(`${api}/__e2e/state`)).json()
  expect(state.adoptedRepositories).toEqual(expect.arrayContaining([expect.objectContaining({ username: 'imported-research' })]))
})
