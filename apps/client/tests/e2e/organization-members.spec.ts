import { expect, test } from './test-fixtures'

test('shows organization members and roles and filters the list', async ({ page }, testInfo) => {
  await page.route('**/v1/graphql', async (route) => {
    const body = route.request().postDataJSON() as { query?: string; variables?: { username?: string } }
    if (!body.query?.includes('LibraryClientOrganizationMembers')) {
      await route.continue()
      return
    }
    expect(body.variables?.username).toBe('quantum-box')
    await route.fulfill({ json: { data: { organization: { users: [
      { id: 'member-alex', name: 'Alex Morgan', email: 'alex@example.com', image: null, role: 'OWNER' },
      { id: 'member-jamie', name: 'Jamie Chen', email: 'jamie@example.com', image: null, role: 'MANAGER' },
      { id: 'member-riley', name: 'Riley Park', email: 'riley@example.com', image: null, role: 'GENERAL' },
    ] } } } })
  })
  await page.goto('/organizations/quantum-box')
  await page.getByRole('link', { name: 'Members & permissions' }).click()
  const members = page.locator('#members')
  await expect(members.getByRole('cell', { name: 'Owner', exact: true })).toBeVisible()
  await expect(members.getByRole('cell', { name: 'Manager', exact: true })).toBeVisible()
  await expect(members.getByRole('cell', { name: 'General', exact: true })).toBeVisible()
  await expect(page.getByText('Loading Library data', { exact: true })).toBeHidden()
  await page.screenshot({ path: testInfo.outputPath('org-members-permissions.png'), fullPage: true })
  await members.getByRole('textbox').fill('jamie@')
  await expect(members.getByText('Jamie Chen')).toBeVisible()
  await expect(members.getByText('Alex Morgan')).toHaveCount(0)
  await members.getByRole('textbox').fill('missing-member')
  await expect(members.getByText('No matching members.')).toBeVisible()
})
