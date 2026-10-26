#!/usr/bin/env node
// Explicit, copy-on-migration conversion: the original Property is never changed.
import { createHash } from 'node:crypto'
import { pathToFileURL } from 'node:url'

export function normalizeDecimal(text, { precision = 28, scale = 12 } = {}) {
  if (!Number.isInteger(precision) || precision < 1 || precision > 28 || !Number.isInteger(scale) || scale < 0 || scale > precision) throw new Error('Invalid precision/scale')
  if (typeof text !== 'string' || text.length > 64 || !/^-?\d+(?:\.\d+)?$/.test(text)) throw new Error('Expected a plain decimal string')
  const [integer, fraction = ''] = text.replace(/^-/, '').split('.')
  const whole = integer.replace(/^0+/, '') || '0'
  if ((whole === '0' ? 0 : whole.length) > precision - scale || fraction.length > scale) throw new Error('Value exceeds precision/scale; rounding is never implicit')
  const tail = fraction.replace(/0+$/, '')
  return `${text.startsWith('-') && (whole !== '0' || tail) ? '-' : ''}${whole}${tail ? `.${tail}` : ''}`
}

export function planMigration(records, sourceId, config) {
  const failures = [], values = []
  for (const record of records) {
    const value = record.items.find((item) => item.property_id === sourceId)?.value
    if (value == null) continue
    const source = value.string ?? (Number.isSafeInteger(value.integer) ? String(value.integer) : undefined)
    try { values.push({ id: record.id, version: record.recordVersion, source, decimal: normalizeDecimal(source, config) }) }
    catch (error) { failures.push({ id: record.id, error: error.message }) }
  }
  const digest = createHash('sha256').update(JSON.stringify({ sourceId, config, values, failures })).digest('hex')
  return { digest, values, failures }
}

async function main() {
  const args = Object.fromEntries(process.argv.slice(2).map((arg) => { const [key, ...value] = arg.replace(/^--/, '').split('='); return [key, value.join('=') || true] }))
  for (const key of ['base-url', 'org', 'repo', 'source', 'target']) if (typeof args[key] !== 'string') throw new Error(`Missing --${key}=...`)
  if (!/^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(args.target) || args.target.startsWith('ext_')) throw new Error('Invalid destination property key')
  const config = { precision: Number(args.precision ?? 28), scale: Number(args.scale ?? 12) }
  normalizeDecimal('0', config)
  const base = new URL(args['base-url'])
  if (!['https:', 'http:'].includes(base.protocol) || base.username || base.password) throw new Error('Use an HTTP(S) API URL without embedded credentials')
  const api = `${base.href.replace(/\/$/, '')}/v1beta/repos/${encodeURIComponent(args.org)}/${encodeURIComponent(args.repo)}`
  const request = async (method, path, body) => {
    const response = await fetch(`${api}${path}`, { method, headers: { 'content-type': 'application/json', ...(process.env.LIBRARY_TOKEN ? { authorization: `Bearer ${process.env.LIBRARY_TOKEN}` } : {}) }, ...(body ? { body: JSON.stringify(body) } : {}) })
    if (!response.ok) throw new Error(`${method} ${path}: HTTP ${response.status}`)
    return response.status === 204 ? null : response.json()
  }
  const properties = await request('GET', '/properties')
  const source = properties.find((p) => p.id === args.source)
  if (!source || !['STRING', 'INTEGER'].includes(source.property_type)) throw new Error('Source must be an existing String or Integer property')
  if (properties.some((p) => p.name === args.target)) throw new Error('Destination already exists; use a new key')
  const records = []
  for (let page = 1; ; page++) {
    const response = await request('GET', `/data-list?page=${page}&page_size=100`)
    records.push(...response.data)
    if (response.data.length < 100) break
    if (page >= 10000) throw new Error('Migration exceeds record limit')
  }
  const plan = planMigration(records, args.source, config)
  if (!args.apply) { console.log(JSON.stringify({ digest: plan.digest, records: plan.values.length, failures: plan.failures, config }, null, 2)); return }
  if (args['expected-digest'] !== plan.digest || plan.failures.length) throw new Error('Preview digest changed or preview has failures; no writes performed')
  const target = await request('POST', '/properties', { name: args.target, property_type: 'decimal', config })
  try {
    for (const item of plan.values) {
      const current = await request('GET', `/data/${encodeURIComponent(item.id)}`)
      const check = planMigration([current], args.source, config).values[0]
      if (!check || check.source !== item.source || check.version !== item.version) throw new Error('Source changed during migration')
      await request('PUT', `/data/${encodeURIComponent(item.id)}`, { name: current.name, property_data: [{ property_id: target.id, value: { decimal: item.decimal } }] })
    }
  } catch (error) {
    try { await request('DELETE', `/properties/${encodeURIComponent(target.id)}`) }
    catch { throw new Error(`Migration interrupted; source is intact. Remove the new destination ${target.id} to roll back.`) }
    throw error
  }
  console.log(JSON.stringify({ property_id: target.id, copied: plan.values.length, source_preserved: true }))
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main().catch((error) => { console.error(error.message); process.exitCode = 1 })
