import assert from 'node:assert/strict'
import { test } from 'node:test'
import { mkdtemp, rm, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve, join } from 'node:path'
import { createHash } from 'node:crypto'
import { Miniflare } from 'miniflare'
import * as Y from 'yjs'

const origin = 'https://reader.example.test'
const edge = 'https://sync.example.test'
const productionOrigins = (await readFile(new URL('../wrangler.jsonc', import.meta.url), 'utf8')).match(/"PHOTON_LIVE_ALLOWED_ORIGINS":\s*"([^"]+)"/)[1]
const api = 'https://api.example.test'
const identity = { roomId: 'live:test-room', tenant: 'tenant-test', database: 'db-test', data: 'data-test', property: 'body', format: 'markdown' }
const metaKey = 'live:room:meta:v1'
const pendingKey = 'live:checkpoint:pending:v1'
const sha = (s) => createHash('sha256').update(s).digest('base64url')
const bodyHash = (body, format = 'markdown') => sha(`${format}\0${body}`)
const resultKey = (id) => `live:checkpoint:result:${Buffer.from(id).toString('base64url')}`
const asBytes = (bytes) => ({ $bytes: Buffer.from(bytes).toString('base64') })
const operationBody = (id, version, body) => JSON.stringify({ type: 'live-checkpoint', operation_id: id, version, body })
async function until(predicate, message = 'condition', timeout = 8000) {
  const deadline = Date.now() + timeout
  while (Date.now() < deadline) { const value = await predicate(); if (value) return value; await new Promise((r) => setTimeout(r, 5)) }
  throw new Error(`Timed out: ${message}`)
}
function socket(response) {
  assert.equal(response.status, 101)
  const ws = response.webSocket
  const doc = new Y.Doc()
  const messages = []
  let failure
  ws.addEventListener('message', ({ data }) => {
    try { if (typeof data === 'string') messages.push(JSON.parse(data)); else Y.applyUpdate(doc, new Uint8Array(data)) } catch (e) { failure = e }
  })
  ws.accept()
  return {
    ws, doc, messages,
    take: async (type, check = () => true) => until(() => {
      if (failure) throw failure
      const i = messages.findIndex((m) => m.type === type && check(m))
      return i < 0 ? null : messages.splice(i, 1)[0]
    }, type),
    edit(text) {
      const updates = []
      const listener = (u) => updates.push(u)
      doc.on('update', listener)
      doc.getText('body').insert(doc.getText('body').length, text)
      doc.off('update', listener)
      for (const update of updates) ws.send(update)
    },
    seed(text) { const seed = new Y.Doc(); seed.getText('body').insert(0, text); ws.send(JSON.stringify({ type: 'live-initialize', update: Buffer.from(Y.encodeStateAsUpdate(seed)).toString('base64url') })); seed.destroy() },
    checkpoint(id, version, body) { ws.send(operationBody(id, version, body)) },
    close() { try { ws.close(1000) } catch {} doc.destroy() },
  }
}
async function scenario(t, overrides = {}) {
  const persist = await mkdtemp(join(tmpdir(), 'library-rust-sync-'))
  const state = { body: 'Seed', recordVersion: '1', authorizationStatus: 200, checkpointStatus: 200, calls: [], ...overrides }
  const sockets = []
  let mf
  const start = () => new Miniflare({
    name: 'sync',
    modules: true,
    scriptPath: resolve('workers/tests/runtime.mjs'),
    modulesRoot: resolve('workers'),
    modulesRules: [{ type: 'ESModule', include: ['**/*.js'], fallthrough: true }, { type: 'CompiledWasm', include: ['**/*.wasm'], fallthrough: true }],
    compatibilityDate: '2026-05-05',
    durableObjects: Object.fromEntries(['PhotonSyncRoom', 'PhotonLiveRoom', 'PhotonLiveTicketStore'].map((className, i) => [ ['PHOTON_SYNC_ROOMS', 'PHOTON_LIVE_ROOMS', 'PHOTON_LIVE_TICKETS'][i], { className, useSQLite: true } ])),
    durableObjectsPersist: persist,
    bindings: { PHOTON_LIVE_ENABLED: 'true', PHOTON_LIVE_ALLOWED_ORIGINS: origin, PHOTON_LIVE_API_BASE_URL: api, PHOTON_CLOUD_ENGINE_BASE_URL: api, ...overrides.bindings },
    outboundService: async (request) => {
      assert.equal(new URL(request.url).origin, api)
      const body = request.method === 'GET' ? undefined : await request.json()
      state.calls.push({ path: new URL(request.url).pathname, body, headers: Object.fromEntries(request.headers) })
      if (request.url.endsWith('/live/authorize')) {
        if (state.authorizeGate) await state.authorizeGate
        return Response.json({ tenant: identity.tenant, database: identity.database, data: identity.data, property: identity.property, format: state.format ?? identity.format, actor_id: 'actor-test', room_id: identity.roomId, record_version: state.recordVersion, body: state.body }, { status: state.authorizationStatus })
      }
      if (request.url.endsWith('/live/checkpoint')) {
        if (state.checkpointGate) await state.checkpointGate
        if (state.checkpointStatus !== 200) return Response.json({ error: 'test failure' }, { status: state.checkpointStatus })
        if (state.operations?.has(body.operation_id)) return Response.json({ record_version: state.operations.get(body.operation_id) })
        if (body.expected_record_version !== state.recordVersion) return Response.json({ error: 'conflict' }, { status: 409 })
        state.recordVersion = (BigInt(state.recordVersion) + 1n).toString(); state.body = body.body
        state.operations ??= new Map(); state.operations.set(body.operation_id, state.recordVersion)
        return Response.json({ record_version: state.recordVersion })
      }
      return Response.json({ ok: true })
    },
  })
  mf = start()
  t.after(async () => { for (const ws of sockets) ws.close(); await mf.dispose(); await rm(persist, { recursive: true, force: true }) })
  const fetch = (path, init) => mf.dispatchFetch(edge + path, init)
  const issue = (extra = {}) => fetch('/live/session', { method: 'POST', headers: { origin, authorization: 'Bearer test-only', 'content-type': 'application/json', ...extra }, body: JSON.stringify({ org: 'acme', repo: 'guide', data_id: identity.data, property_id: identity.property }) })
  const connect = async (ticket) => {
    ticket ??= (await (await issue()).json()).ticket
    return fetch(`/live/ws?ticket=${ticket}`, { headers: { origin, upgrade: 'websocket' } })
  }
  const live = async () => { const ws = socket(await connect()); sockets.push(ws); await ws.take('live-ready'); return ws }
  const sync = async (room = 'records') => { const ws = socket(await fetch(`/ws?room=${room}`, { headers: { upgrade: 'websocket' } })); sockets.push(ws); await ws.take('presence'); return ws }
  const stub = async (binding = 'PHOTON_LIVE_ROOMS', name = identity.roomId) => { const ns = await mf.getDurableObjectNamespace(binding); return ns.get(ns.idFromName(name)) }
  const storage = async (binding, name) => (await (await stub(binding, name)).fetch('https://fixture/__test/state')).json()
  const seed = async (values, binding, name) => { const r = await (await stub(binding, name)).fetch('https://fixture/__test/seed', { method: 'POST', body: JSON.stringify(values) }); assert.equal(r.status, 200) }
  return { state, fetch, issue, connect, live, sync, storage, seed, stub,
    restart: async () => { for (const ws of sockets) ws.close(); await mf.dispose(); mf = start() },
    initialize: async () => { const ws = await live(); ws.seed(state.body); await ws.take('live-ready', (m) => m.initialized); return ws },
    checkpoints: () => state.calls.filter((c) => c.path.endsWith('/live/checkpoint')),
  }
}

test('root, health, feature gate, exact origins and protected internal routes', async (t) => {
  const s = await scenario(t)
  assert.equal((await s.fetch('/')).status, 200)
  assert.equal((await (await s.fetch('/api/health')).json()).backend, 'cloudflare-durable-object')
  for (const bad of [undefined, 'https://reader.example.test.evil.test', 'null']) {
    const r = await s.issue({ origin: bad ?? '' }); assert.equal(r.status, 403); assert.equal(r.headers.get('access-control-allow-origin'), null)
  }
  assert.equal((await s.fetch('/live/internal-pointer')).status, 404)
  const off = await scenario(t, { bindings: { PHOTON_LIVE_ENABLED: 'false' } })
  assert.equal((await (await off.issue()).json()).code, 'LIVE_DISABLED')
})
test('authorization forwards only caller identity and issues one-time tickets without secrets', async (t) => {
  const s = await scenario(t)
  const response = await s.issue({ 'x-platform-id': 'platform-test', 'x-operator-id': 'operator-test' })
  assert.equal(response.status, 200)
  assert.equal(response.headers.get('access-control-allow-origin'), origin)
  const session = await response.json()
  assert.equal(session.body, 'Seed'); assert.doesNotMatch(JSON.stringify(session), /test-only|authorization|sessionId/)
  assert.equal(s.state.calls[0].headers.authorization, 'Bearer test-only')
  assert.equal(s.state.calls[0].headers['x-platform-id'], 'platform-test')
  const ws = socket(await s.connect(session.ticket)); t.after(() => ws.close()); await ws.take('live-ready')
  assert.equal((await s.connect(session.ticket)).status, 401)
  s.state.authorizationStatus = 403
  const denied = await s.issue(); assert.equal(denied.status, 403); assert.doesNotMatch(await denied.text(), /Seed|tenant-test/)
})
test('generic Yjs relay exchanges edits, presence and engine-changed; recovers after restart', async (t) => {
  const s = await scenario(t); const a = await s.sync(); const b = await s.sync()
  await a.take('presence', (v) => v.onlineCount === 2)
  a.edit('hello'); await until(() => b.doc.getText('body').toString() === 'hello')
  b.edit(' world'); await until(() => a.doc.getText('body').toString() === 'hello world')
  await s.fetch('/api/engine/push', { method: 'POST', body: JSON.stringify({ ops: [] }), headers: { 'content-type': 'application/json', authorization: 'Bearer user-token' } })
  await a.take('engine-changed'); assert.equal(s.state.calls.at(-1).headers.authorization, 'Bearer user-token')
  await s.restart(); const c = await s.sync(); await until(() => c.doc.getText('body').toString() === 'hello world')
})
test('two Live clients initialize once, exchange updates, save and reload Yjs state', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); const b = await s.live()
  await until(() => b.doc.getText('body').toString() === 'Seed')
  b.seed('losing seed'); await b.take('live-ready', (m) => m.initialized)
  assert.equal(b.doc.getText('body').toString(), 'Seed')
  a.edit(' edited'); const version = (await a.take('live-version')).version
  await until(() => b.doc.getText('body').toString() === 'Seed edited')
  a.checkpoint('op-save', version, 'Seed edited'); const saved = await b.take('live-saved')
  assert.equal(saved.record_version, '2'); assert.equal(s.state.body, 'Seed edited')
  await s.restart(); const c = await s.live(); await until(() => c.doc.getText('body').toString() === 'Seed edited')
})
test('unchanged body avoids CAS, changed body uses fresh version, replay is idempotent', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  s.state.recordVersion = '5'
  a.checkpoint('op-noop', 0, 'Seed'); assert.equal((await a.take('live-saved')).record_version, '5'); assert.equal(s.checkpoints().length, 0)
  a.edit('!'); await a.take('live-version')
  a.checkpoint('op-change', 1, 'Seed!'); await a.take('live-saved')
  assert.equal(s.checkpoints()[0].body.expected_record_version, '5')
  a.checkpoint('op-change', 1, 'Seed!'); await a.take('live-saved'); assert.equal(s.checkpoints().length, 1)
  a.checkpoint('op-change', 1, 'different'); assert.equal((await a.take('live-error')).message, 'Checkpoint operation was reused')
})
test('external body changes conflict without checkpoint mutation', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  a.edit('!'); await a.take('live-version'); s.state.body = 'external'; s.state.recordVersion = '2'
  a.checkpoint('op-conflict', 1, 'Seed!'); await a.take('live-conflict'); assert.equal(s.checkpoints().length, 0)
  assert.equal((await s.connect()).status, 409)
  assert.equal((await s.storage())[metaKey].version, 1)
})
test('Yjs updates remain responsive during authorization and checkpoint network waits', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); const b = await s.live()
  let release; s.state.authorizeGate = new Promise((r) => { release = r })
  a.checkpoint('stale', 0, 'proposed'); await until(() => s.state.calls.filter((c) => c.path.endsWith('/authorize')).length === 3)
  b.edit(' edit'); await a.take('live-version'); s.state.authorizeGate = undefined; release()
  const stale = await a.take('live-error'); assert.equal(stale.code, 'CHECKPOINT_STALE'); assert.equal(s.checkpoints().length, 0)
  let releaseCheckpoint; s.state.checkpointGate = new Promise((r) => { releaseCheckpoint = r })
  a.checkpoint('save-version-1', 1, 'Seed edit'); await until(() => s.checkpoints().length === 1)
  b.edit(' more'); await a.take('live-version', (v) => v.version === 2)
  s.state.checkpointGate = undefined; releaseCheckpoint(); await a.take('live-saved')
  const meta = (await s.storage())[metaKey]; assert.equal(meta.version, 2); assert.equal(meta.savedVersion, 1)
})

test('transient failed saves retain a journal and a new session safely replaces it after restart', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  a.edit(' pending'); await a.take('live-version'); s.state.checkpointStatus = 503
  a.checkpoint('old-pending', 1, 'Seed pending'); await a.take('live-error')
  const before = await s.storage(); assert.equal(before[pendingKey].operationId, 'old-pending')
  assert.equal(before['live:checkpoint:pending:body:000000'].$bytes, Buffer.from('Seed pending').toString('base64'))
  await s.restart(); s.state.checkpointStatus = 200; const b = await s.live()
  b.checkpoint('replacement', 1, 'Seed pending'); await b.take('live-saved')
  assert.equal(s.state.body, 'Seed pending'); assert.equal((await s.storage())[pendingKey], undefined)
})
test('exact pending retry recovers a committed request after an ACK loss', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); a.edit(' committed'); await a.take('live-version')
  const pending = { version: 1, operationId: 'lost-ack', expectedRecordVersion: '1', bodyHash: bodyHash('Seed committed'), fingerprint: sha('Seed committed'), bodyByteLength: 14, chunkCount: 1 }
  pending.bodyByteLength = Buffer.byteLength('Seed committed')
  await s.seed({ [pendingKey]: pending, 'live:checkpoint:pending:body:000000': asBytes(Buffer.from('Seed committed')) })
  s.state.recordVersion = '2'; s.state.body = 'Seed committed'; s.state.operations = new Map([['lost-ack', '2']])
  // Existing session remains usable even though a new join would see the body conflict.
  a.checkpoint('lost-ack', 1, 'Seed committed'); assert.equal((await a.take('live-saved')).record_version, '2')
  assert.equal(s.state.recordVersion, '2'); assert.equal((await s.storage())[pendingKey], undefined)
})
test('old acknowledged retries preserve newer metadata and pending saves; stale replacements do not erase the journal', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  const current = (await s.storage())[metaKey]
  const pending = { version: 4, operationId: 'newer', expectedRecordVersion: '8', bodyHash: bodyHash('new proposal'), fingerprint: sha('new proposal'), bodyByteLength: 12, chunkCount: 1 }
  await s.seed({
    [metaKey]: { ...current, version: 4, savedVersion: 3, recordVersion: '8' },
    [pendingKey]: pending,
    'live:checkpoint:pending:body:000000': asBytes(Buffer.from('new proposal')),
    [resultKey('old')]: { version: 1, operationId: 'old', recordVersion: '2', bodyHash: bodyHash('old body'), fingerprint: sha('old body'), expiresAt: Date.now() + 100000 },
  })
  a.checkpoint('old', 1, 'old body'); await a.take('live-saved')
  assert.deepEqual((await s.storage())[pendingKey], pending); assert.equal((await s.storage())[metaKey].recordVersion, '8')
  a.checkpoint('stale-new-operation', 1, 'body'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_STALE')
  assert.deepEqual((await s.storage())[pendingKey], pending); assert.equal(s.checkpoints().length, 0)
})
test('concurrent checkpoint callers are serialized without replacing an active CAS', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); const b = await s.live()
  a.edit(' edit'); await b.take('live-version')
  let release; s.state.checkpointGate = new Promise((r) => { release = r })
  a.checkpoint('first', 1, 'Seed edit'); await until(() => s.checkpoints().length === 1)
  b.checkpoint('second', 1, 'Seed edit')
  assert.equal((await s.storage())[pendingKey].operationId, 'first')
  s.state.checkpointGate = undefined; release()
  await b.take('live-saved', (v) => v.operation_id === 'second')
  assert.equal(s.checkpoints().length, 1); assert.equal(s.state.recordVersion, '2')
})
test('clean rooms rotate by canonical version, including returning to an older body', async (t) => {
  const s = await scenario(t); await s.initialize()
  s.state.body = 'External B'; s.state.recordVersion = '2'
  const b = await s.live(); b.seed('External B'); await b.take('live-ready', (m) => m.initialized)
  const firstPointer = (await s.storage())['live:room:current-generation:v1']
  assert.match(firstPointer.roomId, /^live-generation:/)
  s.state.body = 'Seed'; s.state.recordVersion = '3'
  const c = await s.live(); c.seed('Seed'); await c.take('live-ready', (m) => m.initialized)
  const secondPointer = (await s.storage())['live:room:current-generation:v1']
  assert.notEqual(secondPointer.roomId, firstPointer.roomId)
  await s.restart(); const d = await s.live(); await until(() => d.doc.getText('body').toString() === 'Seed')
})
test('uninitialized rooms retain versions, reject stale tickets, and legacy rooms can rotate', async (t) => {
  const s = await scenario(t)
  const stale = (await (await s.issue()).json()).ticket
  s.state.body = 'new body'; s.state.recordVersion = '2'; await s.live()
  assert.equal((await s.storage())[metaKey].recordVersion, '2')
  assert.equal((await s.connect(stale)).status, 409)
  const legacy = { ...(await s.storage())[metaKey] }; delete legacy.recordVersion
  await s.seed({ [metaKey]: legacy }); s.state.body = 'newest'; s.state.recordVersion = '3'
  const c = await s.live(); c.seed('newest'); await c.take('live-ready', (m) => m.initialized)
})
test('RichText key order and numeric formatting do not rotate equivalent canonical bodies', async (t) => {
  const s = await scenario(t, { format: 'richText', body: '[{"z":1.0,"a":{"10":2,"2":3},"type":"paragraph","content":[{"text":"日本語 😀"}]}]' })
  await s.live()
  s.state.body = '[{"content":[{"text":"日本語 😀"}],"type":"paragraph","a":{"2":3,"10":2},"z":1}]'; s.state.recordVersion = '2'
  await s.live(); const stored = await s.storage()
  assert.equal(stored['live:room:current-generation:v1'], undefined); assert.equal(stored[metaKey].recordVersion, '2')
})
test('legacy ArrayBuffer snapshots and pending initialization survive the Rust migration', async (t) => {
  const s = await scenario(t)
  const seed = new Y.Doc(); seed.getText('body').insert(0, 'legacy 日本語 😀')
  const update = Y.encodeStateAsUpdate(seed)
  await s.seed({ 'yjs:snapshot:bytes': asBytes(update), 'yjs:snapshot:meta': { seq: 4, byteLength: update.length, updatedAt: new Date().toISOString() }, 'yjs:update:meta': { nextSeq: 5, oldestSeq: 5 } }, 'PHOTON_SYNC_ROOMS', 'legacy')
  const a = await s.sync('legacy'); await until(() => a.doc.getText('body').toString() === 'legacy 日本語 😀')
  await s.live()
  const meta = (await s.storage())[metaKey]
  const initial = new Y.Doc(); initial.getText('body').insert(0, 'Seed')
  await s.seed({ [metaKey]: meta, 'live:room:init-pending:v1': { update: asBytes(Y.encodeStateAsUpdate(initial)), recordVersion: '1' } })
  await s.restart(); const b = await s.live(); await until(() => b.doc.getText('body').toString() === 'Seed')
  assert.equal((await s.storage())['live:room:init-pending:v1'], undefined)
})
test('snapshot compaction spans multiple values and preserves out-of-order Yjs dependencies', async (t) => {
  const s = await scenario(t); const a = await s.sync('large')
  const source = new Y.Doc(); const updates = []; source.on('update', (u) => updates.push(u))
  for (let i = 0; i < 60; i++) source.getText('body').insert(source.getText('body').length, 'あ😀'.repeat(1024))
  // Persist a suffix first: compaction must retain updates whose dependency
  // has not arrived, then integrate them when the first update finally arrives.
  for (const u of updates.slice(1).reverse()) a.ws.send(u)
  await until(async () => (await s.storage('PHOTON_SYNC_ROOMS', 'large'))['yjs:update:meta']?.nextSeq === 60)
  a.ws.send(updates[0])
  await until(async () => (await s.storage('PHOTON_SYNC_ROOMS', 'large'))['yjs:update:meta']?.nextSeq === 61)
  const stored = await s.storage('PHOTON_SYNC_ROOMS', 'large'); assert.ok(stored['yjs:snapshot:meta'].chunks > 1)
  await s.restart(); const b = await s.sync('large'); await until(() => b.doc.getText('body').toString() === source.getText('body').toString())
})
test('legacy result keys migrate and expire, but pending ACK recovery stays pinned', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  const legacy = { version: 0, operationId: 'legacy-result', recordVersion: '1', bodyHash: bodyHash('Seed'), fingerprint: sha('Seed') }
  await s.seed({ [resultKey(legacy.operationId)]: legacy })
  a.checkpoint(legacy.operationId, 0, 'Seed'); await a.take('live-saved')
  assert.ok((await s.storage())[resultKey(legacy.operationId)].expiresAt > Date.now())
  const pinned = { ...legacy, operationId: 'pinned', expiresAt: Date.now() - 10000 }
  await s.seed({
    [resultKey('pinned')]: pinned,
    [resultKey('expired')]: { ...pinned, operationId: 'expired' },
    [pendingKey]: { version: 0, operationId: 'pinned', expectedRecordVersion: '1', bodyHash: bodyHash('Seed'), fingerprint: sha('Seed'), bodyByteLength: 4, chunkCount: 1 },
    'live:checkpoint:pending:body:000000': asBytes(Buffer.from('Seed')),
  })
  await (await s.stub()).fetch('https://fixture/__test/alarm')
  const stored = await s.storage(); assert.equal(stored[resultKey('expired')], undefined); assert.ok(stored[resultKey('pinned')].expiresAt > Date.now())
  a.checkpoint('pinned', 0, 'Seed'); await a.take('live-saved'); assert.equal((await s.storage())[pendingKey], undefined)
})
test('replay result index remains bounded to 128 operations', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  for (let i = 0; i < 132; i++) { a.checkpoint(`noop-${i}`, 0, 'Seed'); await a.take('live-saved') }
  const stored = await s.storage(); assert.equal(stored['live:checkpoint:result-index:v1'].length, 128)
  assert.equal(Object.keys(stored).filter((k) => k.startsWith('live:checkpoint:result:')).length, 128)
  assert.equal(stored[resultKey('noop-0')], undefined); assert.equal(s.checkpoints().length, 0)
})

test('production native origins pass preflight while authentication and exact matching remain required', async (t) => {
  const s = await scenario(t, { bindings: { PHOTON_LIVE_ALLOWED_ORIGINS: productionOrigins } })
  for (const nativeOrigin of ['tauri://localhost', 'http://tauri.localhost']) {
    const preflight = await s.fetch('/live/session', { method: 'OPTIONS', headers: { origin: nativeOrigin } })
    assert.equal(preflight.status, 204); assert.equal(preflight.headers.get('access-control-allow-origin'), nativeOrigin)
    const denied = await s.fetch('/live/session', { method: 'POST', headers: { origin: nativeOrigin }, body: JSON.stringify({ org: 'acme', repo: 'guide', data_id: 'data-test', property_id: 'body' }) })
    assert.equal(denied.status, 401); assert.equal(denied.headers.get('access-control-allow-origin'), nativeOrigin)
  }
  for (const untrusted of ['null', 'http://localhost', 'tauri://localhost.evil.test', 'http://tauri.localhost.evil.test']) {
    const response = await s.issue({ origin: untrusted }); assert.equal(response.status, 403); assert.equal(response.headers.get('access-control-allow-origin'), null)
  }
})
test('backlogged rooms resume over alarms without replaying the same prefix forever', async (t) => {
  const s = await scenario(t); const source = new Y.Doc(); const updates = []
  source.on('update', (u) => updates.push(u))
  for (let i = 0; i < 600; i++) source.getText('body').insert(source.getText('body').length, `${i};`)
  await s.seed({
    ...Object.fromEntries(updates.map((u, i) => [`yjs:update:${String(i + 1).padStart(12, '0')}`, asBytes(u)])),
    'yjs:update:meta': { oldestSeq: 1, nextSeq: 601 },
  }, 'PHOTON_SYNC_ROOMS', 'backlog')
  const a = await s.sync('backlog')
  const partial = await s.storage('PHOTON_SYNC_ROOMS', 'backlog')
  assert.equal(partial['yjs:snapshot:meta'].seq, 512)
  await until(() => a.doc.getText('body').toString() === source.getText('body').toString(), 'alarm catch-up')
  const done = await s.storage('PHOTON_SYNC_ROOMS', 'backlog')
  assert.equal(done['yjs:update:meta'].oldestSeq, 601)
  await s.restart(); const b = await s.sync('backlog'); await until(() => b.doc.getText('body').toString() === source.getText('body').toString())
})
test('malformed and oversized frames cannot change document versions or pending state', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  a.ws.send(new Uint8Array([255, 255, 255])); await a.take('live-error')
  a.ws.send(new Uint8Array(128 * 1024)); await a.take('live-error')
  a.ws.send(JSON.stringify({ type: 'awareness', update: '%not-base64' })); await a.take('live-error')
  a.ws.send(JSON.stringify({ type: 'live-checkpoint', operation_id: 'bad-version', version: 1.5, body: 'wrong' })); await a.take('live-error')
  const state = await s.storage(); assert.equal(state[metaKey].version, 0); assert.equal(state[pendingKey], undefined); assert.equal(s.checkpoints().length, 0)
})
test('expired ticket and session rows are cleaned in bounded pages', async (t) => {
  const s = await scenario(t); await s.issue()
  const binding = 'PHOTON_LIVE_TICKETS'; const name = 'live-ticket-store'
  const stored = await s.storage(binding, name)
  const template = Object.values(stored).find((s) => s && typeof s === 'object' && s.authorization)
  const seed = {}
  for (let i = 0; i < 140; i++) {
    seed[`live:ticket:${String(i).padStart(43, '0')}`] = { ...template, expiresAt: Date.now() - 1000 }
    seed[`live:session:${String(i).padStart(24, '0')}`] = { ...template, expiresAt: Date.now() - 1000 }
  }
  await s.seed(seed, binding, name)
  await (await s.stub(binding, name)).fetch('https://fixture/__test/alarm')
  const once = await s.storage(binding, name)
  assert.ok(Object.values(once).some((v) => v && typeof v === 'object' && v.expiresAt < Date.now()))
  await (await s.stub(binding, name)).fetch('https://fixture/__test/alarm')
  const twice = await s.storage(binding, name)
  assert.ok(!Object.values(twice).some((v) => v && typeof v === 'object' && v.expiresAt < Date.now()))
})

test('session-expiry alarm releases an idle connection at its absolute expiry', async (t) => {
  const s = await scenario(t); const issued = await (await s.issue()).json()
  const name = 'live-ticket-store'; const binding = 'PHOTON_LIVE_TICKETS'
  const key = `live:ticket:${issued.ticket}`
  const stored = await s.storage(binding, name)
  await s.seed({ [key]: { ...stored[key], expiresAt: Date.now() + 1000 } }, binding, name)
  const a = socket(await s.connect(issued.ticket)); t.after(() => a.close())
  await a.take('live-ready')
  const alarmState = async () => (await (await s.stub()).fetch('https://fixture/__test/alarm-state')).json()
  const active = await alarmState()
  assert.equal(active.sockets.length, 1)
  assert.ok(active.alarm > active.now && active.alarm < active.now + 2000)
  await until(async () => (await alarmState()).sockets.length === 0, 'idle session expires', 5000)
  assert.equal((await alarmState()).alarm, null)
})
test('checkpoint bodies split across storage rows recover with UTF-8 boundaries intact', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  const body = '日本語😀'.repeat(20000)
  s.state.checkpointStatus = 503; a.checkpoint('large-body', 0, body); await a.take('live-error')
  const journal = await s.storage(); assert.ok(journal[pendingKey].chunkCount > 1)
  s.state.checkpointStatus = 200
  a.checkpoint('large-body', 0, body); await a.take('live-saved')
  assert.equal(s.state.body, body); assert.equal((await s.storage())[pendingKey], undefined)
})

test('Engine diagnostics retain exact byte counts without buffering the response', async (t) => {
  const s = await scenario(t)
  const body = JSON.stringify({ ops: [], client: 'test' })
  const response = await s.fetch('/api/engine/push', { method: 'POST', headers: { 'content-type': 'application/json' }, body })
  const received = await response.text()
  const debug = await (await s.fetch('/__debug/sync')).json()
  assert.equal(debug.logs[0].requestBytes, Buffer.byteLength(body))
  assert.equal(debug.logs[0].responseBytes, Buffer.byteLength(received))
  assert.doesNotMatch(JSON.stringify(debug), /test-only|authorization/)
})
