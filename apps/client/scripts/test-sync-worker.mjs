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
const pointerKey = 'live:room:current-generation:v1'
const generationName = (recordVersion, body) => `live-generation:${Buffer.from(`${identity.roomId}\0${recordVersion}\0${bodyHash(body)}`).toString('base64url')}`
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
  const closes = []
  let failure
  ws.addEventListener('message', ({ data }) => {
    try { if (typeof data === 'string') messages.push(JSON.parse(data)); else Y.applyUpdate(doc, new Uint8Array(data)) } catch (e) { failure = e }
  })
  ws.addEventListener('close', ({ code }) => closes.push(code))
  ws.accept()
  return {
    ws, doc, messages, closes,
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
    bindings: { PHOTON_LIVE_ENABLED: 'true', PHOTON_LIVE_ALLOWED_ORIGINS: origin, PHOTON_LIVE_API_BASE_URL: api, PHOTON_CLOUD_ENGINE_BASE_URL: api, EXTERNAL_SYNC_SCANNER_ENABLED: 'false', ...overrides.bindings },
    outboundService: async (request) => {
      assert.equal(new URL(request.url).origin, api)
      const body = request.method === 'GET' ? undefined : await request.json()
      state.calls.push({ path: new URL(request.url).pathname, body, headers: Object.fromEntries(request.headers) })
      if (request.url.endsWith('/live/authorize')) {
        if (state.authorizeGate) await state.authorizeGate
        return Response.json({ tenant: identity.tenant, database: identity.database, data: identity.data, property: identity.property, format: state.format ?? identity.format, actor_id: 'actor-test', room_id: state.roomId ?? identity.roomId, record_version: state.recordVersion, body: state.body }, { status: state.authorizationStatus })
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
  const stub = async (binding = 'PHOTON_LIVE_ROOMS', name = state.roomId ?? identity.roomId) => { const ns = await mf.getDurableObjectNamespace(binding); return ns.get(ns.idFromName(name)) }
  const storage = async (binding, name) => (await (await stub(binding, name)).fetch('https://fixture/__test/state')).json()
  const seed = async (values, binding, name) => { const r = await (await stub(binding, name)).fetch('https://fixture/__test/seed', { method: 'POST', body: JSON.stringify(values) }); assert.equal(r.status, 200) }
  const pointer = async () => (await storage())['live:room:current-generation:v1']
  // Delivers a stored session straight to one room, as the edge does after a hop.
  const joinDirect = async (room, recordVersion) => {
    const sessions = Object.values(await storage('PHOTON_LIVE_TICKETS', 'live-ticket-store'))
    const { authorization, ...reference } = sessions.find((v) => v?.sessionId && v.recordVersion === recordVersion)
    assert.ok(authorization)
    return (await stub('PHOTON_LIVE_ROOMS', room)).fetch('https://live.internal/live/internal-ws', {
      headers: { upgrade: 'websocket', 'x-photon-live-internal': '1', 'x-photon-live-target': room, 'x-photon-live-session': Buffer.from(JSON.stringify(reference)).toString('base64url') },
    })
  }
  // Rejects the next checkpoint request like a timeout; see workers/tests/runtime.mjs.
  const fault = async (mode) => { const r = await (await stub()).fetch('https://fixture/__test/transport-fault', { method: 'POST', body: JSON.stringify({ mode }) }); assert.equal(r.status, 200) }
  return { state, fetch, issue, connect, live, sync, storage, seed, stub, pointer, fault, joinDirect,
    scheduled: async () => (await mf.getWorker()).scheduled(),
    restart: async () => { for (const ws of sockets) ws.close(); await mf.dispose(); mf = start() },
    initialize: async () => { const ws = await live(); ws.seed(state.body); await ws.take('live-ready', (m) => m.initialized); return ws },
    checkpoints: () => state.calls.filter((c) => c.path.endsWith('/live/checkpoint')),
  }
}

test('scheduled external-sync scan is gated and carries only its secret bearer', async (t) => {
  const off = await scenario(t)
  await off.scheduled()
  assert.equal(off.state.calls.some((call) => call.path.endsWith('/internal/external-sync/outbound-scan')), false)

  const on = await scenario(t, { bindings: { EXTERNAL_SYNC_SCANNER_ENABLED: 'true', EXTERNAL_SYNC_SCANNER_TOKEN: 'scanner-test-secret-at-least-32-bytes' } })
  await on.scheduled()
  const scan = on.state.calls.find((call) => call.path.endsWith('/internal/external-sync/outbound-scan'))
  assert.ok(scan)
  assert.equal(scan.headers.authorization, 'Bearer scanner-test-secret-at-least-32-bytes')
  assert.deepEqual(scan.body, {})
})

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
test('generic Yjs relay serves its stored log to joiners and compacts it only in an alarm', async (t) => {
  const s = await scenario(t); const a = await s.sync()
  for (let i = 0; i < 60; i += 1) a.edit(`${i},`)
  const expected = Array.from({ length: 60 }, (_, i) => `${i},`).join('')
  const room = async () => s.storage('PHOTON_SYNC_ROOMS', 'records')
  await until(async () => Object.keys(await room()).filter((k) => k.startsWith('yjs:update:0')).length === 60)
  // Nothing was folded while relaying; a joiner is sent the log as stored.
  assert.equal((await room())['yjs:snapshot:meta'], undefined)
  const b = await s.sync(); await until(() => b.doc.getText('body').toString() === expected)
  const alarm = async () => (await (await s.stub('PHOTON_SYNC_ROOMS', 'records')).fetch('https://fixture/__test/alarm-state')).json()
  assert.ok((await alarm()).alarm, 'a long log schedules a compaction')
  await (await s.stub('PHOTON_SYNC_ROOMS', 'records')).fetch('https://fixture/__test/alarm')
  const compacted = await room()
  assert.equal(Object.keys(compacted).filter((k) => k.startsWith('yjs:update:0')).length, 0)
  assert.equal(compacted['yjs:snapshot:meta'].seq, 60)
  a.edit('after')
  await s.restart(); const c = await s.sync(); await until(() => c.doc.getText('body').toString() === `${expected}after`)
})
test('clients joining while another edits receive every edit', async (t) => {
  const s = await scenario(t); const a = await s.sync()
  let expected = ''
  const joins = []
  for (let i = 0; i < 40; i += 1) {
    a.edit(`${i},`); expected += `${i},`
    if (i % 8 === 0) joins.push(s.sync())
  }
  const joined = await Promise.all(joins)
  for (const b of joined) await until(() => b.doc.getText('body').toString() === expected, 'joiner has every edit')
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
test('external body changes conflict the checkpoint, then rotate a dirty room without merging its unsaved edits', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  a.edit('!'); await a.take('live-version'); s.state.body = 'external'; s.state.recordVersion = '2'
  a.checkpoint('op-conflict', 1, 'Seed!'); await a.take('live-conflict'); assert.equal(s.checkpoints().length, 0)
  // A dirty room used to refuse every join with a header-less 409, forever.
  const b = socket(await s.connect()); t.after(() => b.close())
  const ready = await b.take('live-ready'); assert.equal(ready.initialized, false); assert.equal(ready.record_version, '2')
  await until(() => a.closes.includes(4410), 'peer closed with 4410')
  const pointer = await s.pointer(); assert.match(pointer.roomId, /^live-generation:/); assert.equal(pointer.recordVersion, '2')
  b.seed('external'); await b.take('live-ready', (m) => m.initialized)
  assert.equal(b.doc.getText('body').toString(), 'external')
  // The unsaved edit stays in the old generation's storage and is never merged.
  const old = (await s.storage())[metaKey]; assert.equal(old.version, 1); assert.equal(old.savedVersion, 0)
  assert.equal((await s.storage('PHOTON_LIVE_ROOMS', pointer.roomId))[metaKey].version, 0)
  assert.equal(s.checkpoints().length, 0)
})
test('peers are closed with 4410 only after the successor pointer commits', async (t) => {
  // This room id is too long to name a generation, so rotation fails before
  // any pointer write. Peers must keep editing instead of being disconnected.
  const s = await scenario(t, { roomId: `live:${'x'.repeat(480)}` }); const a = await s.initialize(); const b = await s.live()
  a.edit('!'); await b.take('live-version'); s.state.body = 'external'; s.state.recordVersion = '2'
  assert.equal((await s.connect()).status, 409)
  a.edit('?'); await until(() => b.doc.getText('body').toString() === 'Seed!?')
  assert.deepEqual(a.closes, []); assert.deepEqual(b.closes, []); assert.equal(await s.pointer(), undefined)
})
test('a join settles a reservation that is already canonical instead of rotating', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); a.edit(' committed'); await a.take('live-version')
  const body = 'Seed committed'
  await s.seed({
    [pendingKey]: { version: 1, operationId: 'lost-ack', expectedRecordVersion: '1', bodyHash: bodyHash(body), fingerprint: sha(body), bodyByteLength: Buffer.byteLength(body), chunkCount: 1 },
    'live:checkpoint:pending:body:000000': asBytes(Buffer.from(body)),
  })
  // The CAS committed, but this room never saw its ACK.
  s.state.recordVersion = '2'; s.state.body = body
  const b = await s.live()
  const saved = await a.take('live-saved'); assert.equal(saved.operation_id, 'lost-ack'); assert.equal(saved.record_version, '2')
  await until(() => b.doc.getText('body').toString() === body)
  const stored = await s.storage(); const meta = stored[metaKey]
  assert.equal(stored['live:room:current-generation:v1'], undefined); assert.equal(stored[pendingKey], undefined)
  assert.equal(meta.recordVersion, '2'); assert.equal(meta.savedVersion, 1); assert.equal(meta.bodyHash, bodyHash(body))
  // The owner's retry of the same operation replays the result without a CAS.
  a.checkpoint('lost-ack', 1, body); assert.equal((await a.take('live-saved')).record_version, '2')
  assert.equal(s.checkpoints().length, 0); assert.deepEqual(a.closes, [])
})
test('a different body at the same record version cannot be ordered and is refused', async (t) => {
  const s = await scenario(t); const a = await s.initialize()
  s.state.body = 'same version, different body'
  assert.equal((await s.connect()).status, 409)
  assert.equal(await s.pointer(), undefined); assert.equal((await s.storage())[metaKey].recordVersion, '1'); assert.deepEqual(a.closes, [])
})
test('recoverable checkpoint failures keep the reservation and ask for an identical retry', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); a.edit(' retry'); await a.take('live-version')
  s.state.checkpointStatus = 500; a.checkpoint('op-500', 1, 'Seed retry')
  const failed = await a.take('live-error'); assert.equal(failed.code, 'CHECKPOINT_RETRY'); assert.equal(failed.operation_id, 'op-500')
  assert.equal((await s.storage())[pendingKey].operationId, 'op-500')
  s.state.checkpointStatus = 200; a.checkpoint('op-500', 1, 'Seed retry')
  assert.equal((await a.take('live-saved')).record_version, '2'); assert.equal(s.checkpoints().length, 2); assert.equal(s.state.body, 'Seed retry')
  // Authorization failures surface through the recovery catch-all.
  a.edit('!'); await a.take('live-version'); s.state.authorizationStatus = 500; a.checkpoint('op-auth', 2, 'Seed retry!')
  const recovery = await a.take('live-error'); assert.equal(recovery.code, 'CHECKPOINT_RETRY'); assert.equal(recovery.message, 'Live checkpoint recovery failed')
  s.state.authorizationStatus = 200; a.checkpoint('op-auth', 2, 'Seed retry!'); assert.equal((await a.take('live-saved')).record_version, '3')
  // A rejected patch repeats identically for the same operation, so it stays terminal.
  a.edit('?'); await a.take('live-version'); s.state.checkpointStatus = 422; a.checkpoint('op-422', 3, 'Seed retry!?')
  assert.equal((await a.take('live-error')).code, undefined)
})
test('a checkpoint response lost after commit is retried with the same operation and replayed by the API', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); a.edit(' lost'); await a.take('live-version')
  await s.fault('after-commit'); a.checkpoint('op-lost', 1, 'Seed lost')
  assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  assert.equal(s.state.recordVersion, '2'); assert.equal((await s.storage())[pendingKey].operationId, 'op-lost')
  a.checkpoint('op-lost', 1, 'Seed lost'); assert.equal((await a.take('live-saved')).record_version, '2')
  assert.equal(s.state.recordVersion, '2'); assert.equal(s.checkpoints().length, 2); assert.equal((await s.storage())[pendingKey], undefined)
  // A request that never left is retried the same way.
  a.edit('!'); await a.take('live-version'); await s.fault('before-send'); a.checkpoint('op-offline', 2, 'Seed lost!')
  assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  a.checkpoint('op-offline', 2, 'Seed lost!'); assert.equal((await a.take('live-saved')).record_version, '3')
  assert.equal(a.messages.some((m) => m.type === 'live-conflict'), false)
})
test('a conflicting retry whose reservation is already canonical settles instead of conflicting', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); a.edit(' same'); await a.take('live-version')
  s.state.checkpointStatus = 503; a.checkpoint('op-same', 1, 'Seed same'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  // Another writer stored the same body; the API has no record of this operation.
  s.state.checkpointStatus = 200; s.state.body = 'Seed same'; s.state.recordVersion = '2'
  a.checkpoint('op-same', 1, 'Seed same')
  const saved = await a.take('live-saved'); assert.equal(saved.operation_id, 'op-same'); assert.equal(saved.record_version, '2')
  assert.equal(a.messages.some((m) => m.type === 'live-conflict'), false)
  const stored = await s.storage(); assert.equal(stored[pendingKey], undefined); assert.equal(stored[metaKey].savedVersion, 1)
  // A different canonical body is a real conflict, but only once it is verified.
  a.edit('!'); await a.take('live-version'); s.state.checkpointStatus = 503; a.checkpoint('op-other', 2, 'Seed same!')
  assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  s.state.checkpointStatus = 200; s.state.body = 'someone else'; s.state.recordVersion = '3'; s.state.authorizationStatus = 500
  a.checkpoint('op-other', 2, 'Seed same!'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  assert.equal((await s.storage())[pendingKey].operationId, 'op-other')
  s.state.authorizationStatus = 200; a.checkpoint('op-other', 2, 'Seed same!'); await a.take('live-conflict', (m) => m.operation_id === 'op-other')
  assert.equal((await s.storage())[pendingKey], undefined)
})
test('a stale ticket never seeds a generation, so the canonical join still gets in', async (t) => {
  const s = await scenario(t); await s.initialize()
  const stale = (await (await s.issue()).json()).ticket
  s.state.body = 'external'; s.state.recordVersion = '2'
  const g = generationName('2', 'external')
  // The rotating join committed the pointer but has not reached its generation yet.
  await s.seed({ [pointerKey]: { roomId: g, bodyHash: bodyHash('external'), recordVersion: '2' } })
  const refused = await s.connect(stale)
  assert.equal(refused.status, 409); assert.equal(refused.headers.get('x-photon-live-generation'), null)
  // Even delivered straight to the unseeded generation it seeds nothing.
  const direct = await s.joinDirect(g, '1')
  assert.equal(direct.status, 409); assert.equal(direct.headers.get('x-photon-live-generation'), null)
  assert.equal((await s.storage('PHOTON_LIVE_ROOMS', g))[metaKey], undefined)
  await s.live()
  const meta = (await s.storage('PHOTON_LIVE_ROOMS', g))[metaKey]
  assert.equal(meta.recordVersion, '2'); assert.equal(meta.bodyHash, bodyHash('external'))
  // A newer session that reaches an unseeded generation rotates past it.
  const skipped = generationName('3', 'third')
  await s.seed({ [pointerKey]: { roomId: skipped, bodyHash: bodyHash('third'), recordVersion: '3' } })
  s.state.body = 'fourth'; s.state.recordVersion = '4'
  const d = socket(await s.connect()); t.after(() => d.close()); assert.equal((await d.take('live-ready')).record_version, '4')
  assert.equal((await s.pointer()).roomId, generationName('4', 'fourth'))
  const retired = await s.storage('PHOTON_LIVE_ROOMS', skipped)
  assert.equal(retired[metaKey], undefined); assert.equal(retired['live:room:retired-by:v1'], generationName('4', 'fourth'))
})
test('a record version bump that leaves the body alone rebases checkpoints instead of conflicting', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); const b = await s.live()
  a.edit(' x'); await b.take('live-version')
  // A title save lands while the CAS is out: the API answers 409.
  let release; s.state.checkpointGate = new Promise((r) => { release = r })
  a.checkpoint('op-1', 1, 'Seed x'); await until(() => s.checkpoints().length === 1)
  s.state.recordVersion = '2'; s.state.checkpointGate = undefined; release()
  const rebased = await a.take('live-error'); assert.equal(rebased.code, 'CHECKPOINT_STALE'); assert.equal(rebased.operation_id, 'op-1')
  let stored = await s.storage(); assert.equal(stored[pendingKey], undefined); assert.equal(stored[metaKey].recordVersion, '2')
  a.checkpoint('op-1b', 1, 'Seed x'); assert.equal((await a.take('live-saved')).record_version, '3')
  assert.equal(s.checkpoints().at(-1).body.expected_record_version, '2')
  // A title save lands between a CHECKPOINT_RETRY and its identical resend.
  a.edit('!'); await b.take('live-version'); s.state.checkpointStatus = 503
  a.checkpoint('op-2', 2, 'Seed x!'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  s.state.checkpointStatus = 200; s.state.recordVersion = '4'
  a.checkpoint('op-2', 2, 'Seed x!'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_STALE')
  a.checkpoint('op-2b', 2, 'Seed x!'); assert.equal((await a.take('live-saved')).record_version, '5')
  // A title save while a reservation is kept: another participant's checkpoint
  // replaces it, and its owner is told to resend under a new operation id.
  a.edit('?'); await b.take('live-version'); s.state.checkpointStatus = 503
  a.checkpoint('op-3', 3, 'Seed x!?'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  s.state.checkpointStatus = 200; s.state.recordVersion = '6'
  b.checkpoint('op-4', 3, 'Seed x!?'); assert.equal((await b.take('live-saved', (m) => m.operation_id === 'op-4')).record_version, '7')
  const owner = await a.take('live-error'); assert.equal(owner.code, 'CHECKPOINT_STALE'); assert.equal(owner.operation_id, 'op-3')
  stored = await s.storage(); assert.equal(stored[pendingKey], undefined); assert.equal(stored[metaKey].recordVersion, '7')
  assert.equal(s.state.body, 'Seed x!?')
  assert.equal([...a.messages, ...b.messages].some((m) => m.type === 'live-conflict'), false)
})
test('a checkpoint whose outcome is unknown is not replaced while it can still commit', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); const b = await s.live()
  a.edit(' x'); await b.take('live-version')
  await s.fault('before-send'); a.checkpoint('op-1', 1, 'Seed x')
  assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  b.edit('!'); await a.take('live-version', (m) => m.version === 2)
  b.checkpoint('op-2', 2, 'Seed x!')
  const held = await b.take('live-error'); assert.equal(held.code, 'CHECKPOINT_RETRY'); assert.equal(held.operation_id, 'op-2')
  assert.equal((await s.storage())[pendingKey].operationId, 'op-1'); assert.equal(s.checkpoints().length, 0)
  // The API commits op-1 after the room gave up on it.
  s.state.body = 'Seed x'; s.state.recordVersion = '2'; s.state.operations = new Map([['op-1', '2']])
  a.checkpoint('op-1', 1, 'Seed x'); assert.equal((await a.take('live-saved', (m) => m.operation_id === 'op-1')).record_version, '2')
  b.checkpoint('op-2', 2, 'Seed x!'); assert.equal((await b.take('live-saved', (m) => m.operation_id === 'op-2')).record_version, '3')
  assert.equal(s.state.body, 'Seed x!')
  // A body that is already canonical at a newer version is saved without a CAS,
  // even when no reservation for it is left (another writer stored it).
  a.edit('?'); await b.take('live-version', (m) => m.version === 3)
  s.state.body = 'Seed x!?'; s.state.recordVersion = '4'; const before = s.checkpoints().length
  a.checkpoint('op-5', 3, 'Seed x!?'); assert.equal((await a.take('live-saved', (m) => m.operation_id === 'op-5')).record_version, '4')
  assert.equal(s.checkpoints().length, before); assert.equal((await s.storage())[metaKey].savedVersion, 3)
  assert.equal([...a.messages, ...b.messages].some((m) => m.type === 'live-conflict'), false)
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
  a.checkpoint('old-pending', 1, 'Seed pending'); assert.equal((await a.take('live-error')).code, 'CHECKPOINT_RETRY')
  const before = await s.storage(); assert.equal(before[pendingKey].operationId, 'old-pending')
  assert.equal(before['live:checkpoint:pending:body:000000'].$bytes, Buffer.from('Seed pending').toString('base64'))
  await s.restart(); s.state.checkpointStatus = 200; const b = await s.live()
  // A gateway 5xx leaves the outcome unknown: the API may still commit it, so
  // an unchanged record version does not yet prove it never will.
  b.checkpoint('replacement', 1, 'Seed pending')
  const held = await b.take('live-error'); assert.equal(held.code, 'CHECKPOINT_RETRY'); assert.equal(held.operation_id, 'replacement')
  assert.equal((await s.storage())[pendingKey].operationId, 'old-pending'); assert.equal(s.checkpoints().length, 1)
  await s.seed({ [pendingKey]: { ...(await s.storage())[pendingKey], inDoubtUntil: Date.now() - 1 } })
  b.checkpoint('replacement', 1, 'Seed pending'); await b.take('live-saved')
  assert.equal(s.state.body, 'Seed pending'); assert.equal((await s.storage())[pendingKey], undefined)
})
test('exact pending retry recovers a committed request after an ACK loss', async (t) => {
  const s = await scenario(t); const a = await s.initialize(); a.edit(' committed'); await a.take('live-version')
  const pending = { version: 1, operationId: 'lost-ack', expectedRecordVersion: '1', bodyHash: bodyHash('Seed committed'), fingerprint: sha('Seed committed'), bodyByteLength: 14, chunkCount: 1 }
  pending.bodyByteLength = Buffer.byteLength('Seed committed')
  await s.seed({ [pendingKey]: pending, 'live:checkpoint:pending:body:000000': asBytes(Buffer.from('Seed committed')) })
  s.state.recordVersion = '2'; s.state.body = 'Seed committed'; s.state.operations = new Map([['lost-ack', '2']])
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
  // The superseded generation closed its peers and redirects a join that read
  // the old pointer just before it moved, instead of accepting it.
  assert.equal((await s.storage('PHOTON_LIVE_ROOMS', firstPointer.roomId))['live:room:retired-by:v1'], secondPointer.roomId)
  await until(() => b.closes.includes(4410), 'superseded peer closed')
  const late = await s.joinDirect(firstPointer.roomId, '3')
  assert.equal(late.status, 409); assert.equal(late.headers.get('x-photon-live-generation'), secondPointer.roomId)
  // A ticket older than the successor is refused rather than forwarded.
  const stale = await s.joinDirect(firstPointer.roomId, '2')
  assert.equal(stale.status, 409); assert.equal(stale.headers.get('x-photon-live-generation'), null)
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
  // Folded by the alarm the long log scheduled, not while relaying.
  await (await s.stub('PHOTON_SYNC_ROOMS', 'large')).fetch('https://fixture/__test/alarm')
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
  // A joiner is sent the whole log as stored, without waiting on a fold.
  const a = await s.sync('backlog')
  await until(() => a.doc.getText('body').toString() === source.getText('body').toString(), 'log sent on join')
  // Alarms fold it a bounded pass at a time, each one further than the last.
  await (await s.stub('PHOTON_SYNC_ROOMS', 'backlog')).fetch('https://fixture/__test/alarm')
  const partial = await s.storage('PHOTON_SYNC_ROOMS', 'backlog')
  assert.ok(partial['yjs:snapshot:meta'].seq >= 512)
  await until(async () => (await s.storage('PHOTON_SYNC_ROOMS', 'backlog'))['yjs:update:meta'].oldestSeq === 601, 'alarm catch-up')
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
