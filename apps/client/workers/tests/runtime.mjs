// Test-only subclasses expose storage fixtures. Production imports build/index.js
// directly and never includes these routes or the fixture codec.
import worker, {
  PhotonSyncRoom as Sync,
  PhotonLiveRoom as Live,
  PhotonLiveTicketStore as Tickets,
  ExternalSyncDispatcher as ExternalDispatch,
} from '../sync/build/index.js'
export default worker
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
      return super.fetch(request)
    }
  }
}
export class PhotonSyncRoom extends fixture(Sync) {}
export class PhotonLiveRoom extends fixture(Live) {}
export class PhotonLiveTicketStore extends fixture(Tickets) {}
export class ExternalSyncDispatcher extends fixture(ExternalDispatch) {}
