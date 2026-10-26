import { test } from 'node:test'
import assert from 'node:assert/strict'
import { normalizeDecimal, planMigration } from './library-decimal-migration.mjs'
test('decimal conversion is exact, explicitly bounded, and never rounds', () => {
  assert.equal(normalizeDecimal('-00012.3400'), '-12.34')
  assert.equal(normalizeDecimal('-0.00'), '0')
  for (const value of ['1e2', '0.1234567890123', 'NaN', 0.1]) assert.throws(() => normalizeDecimal(value))
})
test('migration preview detects stale source data and reports invalid rows', () => {
  const records = [{ id: 'a', recordVersion: '1', items: [{ property_id: 'source', value: { string: '0.1' } }] }]
  const initial = planMigration(records, 'source', { precision: 28, scale: 12 })
  assert.equal(initial.values[0].decimal, '0.1')
  records[0].items[0].value.string = '0.2'
  assert.notEqual(planMigration(records, 'source', { precision: 28, scale: 12 }).digest, initial.digest)
  records[0].items[0].value.string = 'invalid'
  assert.equal(planMigration(records, 'source', { precision: 28, scale: 12 }).failures.length, 1)
})
