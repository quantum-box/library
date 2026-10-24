import { expect, test } from './test-fixtures'

const reference = (id: string, kind: string) => ({ provider: 'linear', workspace_id: 'workspace', kind, external_id: id })
function record(id: string, kind: string, name: string) {
  const resource_ref = reference(id, kind)
  return { id, resource_ref, projection: { resource_ref, name, url: 'https://linear.app/quantum-box/project/example', archived: false, status: 'Active', owner: { external_id: 'owner', name: 'Project owner' }, target_date: '2026-12-31', source_updated_at: '2026-10-24T02:00:00Z' }, observed_at: '2026-10-24T02:05:00Z', observed_revision: 1, lifecycle: 'active', note: '', document_ids: [] }
}

test('repository navigation, shared relationships, context edits and revoked state', async ({ page }) => {
  const records = [record('a', 'initiative', 'Improve onboarding'), record('b', 'initiative', 'Grow adoption'), record('p', 'project', 'Simplify setup')]
  let revoked = false
  await page.route('**/v1beta/repos/quantum-box/photon-core/pm-projection**', async route => {
    const url = new URL(route.request().url())
    if (revoked) return route.fulfill({ status: 403, json: { message: 'Denied' } })
    if (url.pathname.endsWith('/context')) {
      const body = route.request().postDataJSON()
      expect(body).toEqual({ note: 'Reviewed together with sales.', document_ids: [] })
      records[2].note = body.note
      return route.fulfill({ json: { saved: true } })
    }
    if (url.pathname.endsWith('/records/p')) return route.fulfill({ json: records[2] })
    return route.fulfill({ json: { records, relations: [records[0], records[1]].map(r => ({ source: r.resource_ref, target: records[2].resource_ref, removed: false })), pending_relations: 0, sync_pending: false, next_after: null } })
  })
  await page.goto('/quantum-box/photon-core')
  await page.getByRole('link', { name: 'Strategy & execution', exact: true }).click()
  await expect(page).toHaveURL(/\/quantum-box\/photon-core\/strategy/)
  await page.getByRole('button', { name: 'Projects', exact: true }).click()
  await expect(page.getByRole('button', { name: /Simplify setup/ })).toHaveCount(1)
  await page.getByRole('button', { name: /Simplify setup/ }).click()
  const details = page.getByRole('complementary', { name: 'Details' })
  await expect(details.getByRole('button', { name: 'Improve onboarding', exact: true })).toBeVisible()
  await expect(details.getByRole('button', { name: 'Grow adoption', exact: true })).toBeVisible()
  await expect(details.getByText('Source fields are read-only.')).toBeVisible()
  await details.getByLabel('Note', { exact: true }).fill('Reviewed together with sales.')
  await details.getByRole('button', { name: 'Save', exact: true }).click()
  await expect(details.getByText('Context saved.')).toBeVisible()
  await expect(page.getByText('Loading Library data', { exact: true })).not.toBeVisible({ timeout: 15_000 })
  await details.evaluate(element => { element.scrollTop = 0 })
  await page.screenshot({ path: 'test-results/strategy-project-details.png', fullPage: true })
  const local = await page.evaluate(() => JSON.stringify(localStorage))
  expect(local).not.toContain('Simplify setup')
  expect(local).not.toContain('Reviewed together with sales.')
  revoked = true
  await page.getByRole('button', { name: 'Refresh', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('revoked')
  await expect(page.getByText('Simplify setup')).toHaveCount(0)
  await page.screenshot({ path: 'test-results/strategy-revoked.png', fullPage: true })
})

test('partial source data is labeled and pagination is explicit', async ({ page }) => {
  const first = record('a', 'initiative', 'Improve onboarding')
  const second = record('p', 'project', 'Simplify setup')
  await page.route('**/v1beta/repos/quantum-box/photon-core/pm-projection**', async route => {
    const next = new URL(route.request().url()).searchParams.has('after')
    await route.fulfill({ json: { records: next ? [second] : [first], relations: [], pending_relations: next ? 0 : 1, sync_pending: !next, next_after: next ? null : 'a' } })
  })
  await page.goto('/quantum-box/photon-core/strategy')
  await expect(page.getByText(/More source changes/)).toBeVisible()
  await page.getByRole('button', { name: 'Load more', exact: true }).click()
  await expect(page.getByRole('button', { name: /Simplify setup/ })).toBeVisible()
  await expect(page.getByRole('button', { name: /Improve onboarding/ })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Load more', exact: true })).toHaveCount(0)
})
