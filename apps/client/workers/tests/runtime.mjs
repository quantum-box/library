// Test-only subclasses expose storage fixtures. Production imports build/index.js
// directly and never includes these routes or the fixture codec.
import worker, {
  PhotonSyncRoom as Sync,
  PhotonLiveRoom as Live,
  PhotonLiveTicketStore as Tickets,
  ExternalSyncDispatcher as ExternalDispatch,
} from '../sync/build/index.js'
export default worker
// Test-only transport faults. The Rust Worker calls the global fetch, so this
// wrapper can make one checkpoint request reject the way an abort timeout or a
// dropped connection does, without waiting out the Worker's 15s timeout.
// 'after-commit' delivers the request first: the API committed, the ACK is lost.
let transportFault = null
const upstreamFetch = globalThis.fetch
globalThis.fetch = async (input, init) => {
  const url = typeof input === 'string' ? input : input.url
  const fault = url.endsWith('/live/checkpoint') ? transportFault : null
  if (!fault) return upstreamFetch(input, init)
  transportFault = null
  if (fault === 'after-commit') await (await upstreamFetch(input, init)).arrayBuffer()
  throw new TypeError('Network connection lost')
}
function decode(value) {
  if (value && typeof value === 'object' && '$bytes' in value) {
    return Uint8Array.from(atob(value.$bytes), (c) => c.charCodeAt(0)).buffer
  }
  if (Array.isArray(value)) return value.map(decode)
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, decode(v)]))
  return value
}
function encode(value) {
  if (value instanceof ArrayBuffer) return { $bytes: btoa(String.fromCharCode(...new Uint8Array(value))) }
  if (Array.isArray(value)) return value.map(encode)
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, encode(v)]))
  return value
}
function fixture(Base) {
  return class extends Base {
    constructor(state, env) { super(state, env); this.fixtureState = state }
    async fetch(request) {
      const path = new URL(request.url).pathname
      if (path === '/__test/seed') {
        const values = decode(await request.json())
        const entries = Object.entries(values)
        for (let i = 0; i < entries.length; i += 96) await this.fixtureState.storage.put(Object.fromEntries(entries.slice(i, i + 96)))
        return Response.json({ ok: true })
      }
      if (path === '/__test/state') return Response.json(encode(Object.fromEntries(await this.fixtureState.storage.list())))
      if (path === '/__test/alarm-state') return Response.json({ alarm: await this.fixtureState.storage.getAlarm(), now: Date.now(), sockets: this.fixtureState.getWebSockets().map((s) => ({ state: s.readyState, attachment: s.deserializeAttachment() })) })
      if (path === '/__test/alarm') { await super.alarm(); return Response.json({ ok: true }) }
      if (path === '/__test/transport-fault') { transportFault = (await request.json()).mode; return Response.json({ ok: true }) }
      return super.fetch(request)
    }
  }
}
export class PhotonSyncRoom extends fixture(Sync) {}
export class PhotonLiveRoom extends fixture(Live) {}
export class PhotonLiveTicketStore extends fixture(Tickets) {}
export class ExternalSyncDispatcher extends fixture(ExternalDispatch) {}
