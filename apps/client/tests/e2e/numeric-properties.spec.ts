import { expect, test } from './test-fixtures'
const api = 'http://127.0.0.1:50063'

test('decimal and quantity editing preserve exact values after reload', async ({ page }, testInfo) => {
  const create = async (name: string, type: string, config: object) => {
    const response = await page.request.post(`${api}/v1/graphql`, { data: { query: 'mutation LibraryClientAddRepositoryProperty { addProperty { id } }', variables: { input: { orgUsername: 'quantum-box', repoUsername: 'photon-core', propertyName: name, propertyType: type, meta: { json: JSON.stringify(config) } } } } })
    const result = await response.json()
    return result.data.addProperty.id as string
  }
  const decimal = await create('unit_cost', 'DECIMAL', { precision: 28, scale: 12 })
  const quantity = await create('amount', 'QUANTITY', { decimal: { precision: 28, scale: 12 }, allowed_units: ['g','kg'], default_unit: 'g', unit_version: 1 })
  await page.request.post(`${api}/v1/graphql`, { data: { query: 'mutation LibraryClientUpdateData { updateData { id } }', variables: { input: { orgUsername: 'quantum-box', repoUsername: 'photon-core', dataId: 'seed-data-201', dataName: 'Prepare release notes', propertyData: [{ propertyId: decimal, value: { decimal: '0.100000000001' } }, { propertyId: quantity, value: { quantity: '{"value":"1.5","unit":"kg"}' } }] } } } })
  await page.goto('/quantum-box/photon-core/data/seed-data-201')
  const decimalCell = page.getByTestId(`library-editable-cell-${decimal}`)
  await expect(decimalCell).toHaveText('0.100000000001')
  await decimalCell.click()
  const input = page.getByTestId(`library-editable-input-${decimal}`)
  await input.fill('0.200000000002')
  await input.press('Enter')
  await expect(decimalCell).toHaveText('0.200000000002')
  const quantityCell = page.getByTestId(`library-editable-cell-${quantity}`)
  await expect(quantityCell).toHaveText('1.5 kg')
  await quantityCell.click()
  await page.getByLabel('Decimal', { exact: true }).fill('1500')
  await page.getByLabel('Default unit', { exact: true }).selectOption('g')
  await page.getByLabel('Save', { exact: true }).click()
  await expect(quantityCell).toHaveText('1500 g')
  await page.reload()
  await expect(page.getByTestId(`library-editable-cell-${decimal}`)).toHaveText('0.200000000002')
  await expect(page.getByTestId(`library-editable-cell-${quantity}`)).toHaveText('1500 g')
  await page.screenshot({ path: testInfo.outputPath('numeric-properties.png'), fullPage: true })
})
