import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import type { SyncTicketResult } from './syncTicket'

const room = 'tenant:library:workspace:library-default:user:us_01abc:records'
const roomUrl = `wss://sync.example.test/ws?room=${room}`

interface FakeSocketLike extends EventTarget {
  readyState: number
}

const mocks = vi.hoisted(() => ({
  requestSyncTicket: vi.fn<(url: string) => Promise<SyncTicketResult>>(),
  sockets: [] as string[],
  instances: [] as Array<FakeSocketLike>,
  sent: [] as unknown[],
  userId: 'us_01abc' as string | undefined,
  sync: {} as Record<string, unknown>,
}))

vi.mock('../auth', () => ({
  authStorageKey: 'library_auth',
  loadStoredAuthIdentity: () => (mocks.userId ? { userId: mocks.userId } : null),
}))

vi.mock('./syncTicket', () => ({ requestSyncTicket: mocks.requestSyncTicket }))
vi.mock('y-indexeddb', () => ({
  IndexeddbPersistence: class {
    once() {}
  },
}))
vi.mock('../../app/kitConfig.js', () => ({
  appKitConfig: {
    sync: mocks.sync,
    attachments: { yjsArrayName: 'attachments' },
  },
  buildRecordsRoomId: (scope: string, actor: string | undefined) =>
    actor ? `${scope}:user:${actor}:records` : undefined,
  resolveBrowserSyncWebsocketUrl: (configured: string | undefined) => configured,
}))

const baseSync = {
  yjsArrayName: 'records',
  databasesArrayName: 'databases',
  databaseViewsArrayName: 'databaseViews',
  workflowCanvasesMapName: 'workflowCanvases',
  persistenceKey: 'test',
  workspaceScope: 'tenant:library:workspace:library-default',
  recordsRoomId: room,
  websocketUrl: roomUrl,
  websocketPath: `/ws?room=${room}`,
  backend: 'cloudflare-durable-object',
}

class FakeWebSocket extends EventTarget {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3
  readyState = FakeWebSocket.CONNECTING
  binaryType = 'blob'
  constructor(url: string) {
    super()
    mocks.sockets.push(url)
    mocks.instances.push(this)
  }
  send(data: unknown) {
    mocks.sent.push(data)
  }
  /** What the relay does on join: open, its stored state, then presence. */
  join(stored: Uint8Array) {
    this.readyState = FakeWebSocket.OPEN
    this.dispatchEvent(new Event('open'))
    this.dispatchEvent(new MessageEvent('message', { data: stored.slice().buffer }))
    this.dispatchEvent(
      new MessageEvent('message', { data: JSON.stringify({ type: 'presence', onlineCount: 1 }) })
    )
  }
  close() {
    this.readyState = FakeWebSocket.CLOSED
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((r) => { resolve = r })
  return { promise, resolve }
}

const pageEvent = (type: 'pagehide' | 'pageshow', persisted: boolean) =>
  Object.assign(new Event(type), { persisted })

describe('records room connection', () => {
  beforeEach(() => {
    vi.resetModules()
    mocks.sockets.length = 0
    mocks.instances.length = 0
    mocks.sent.length = 0
    mocks.userId = 'us_01abc'
    for (const key of Object.keys(mocks.sync)) delete mocks.sync[key]
    Object.assign(mocks.sync, baseSync)
    mocks.requestSyncTicket.mockReset()
    vi.stubGlobal('WebSocket', FakeWebSocket)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('opens the room only with a ticket issued for it', async () => {
    mocks.requestSyncTicket.mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=one` })
    const provider = await import('./yjsProvider')

    expect(mocks.requestSyncTicket).toHaveBeenCalledWith(roomUrl)
    await vi.waitFor(() => expect(mocks.sockets).toEqual([`${roomUrl}&ticket=one`]))
    provider.disconnectWs()
  })

  it('connects a Rust server directly, since it has no ticket endpoint', async () => {
    mocks.sync.backend = 'rust-server'
    const provider = await import('./yjsProvider')

    expect(mocks.requestSyncTicket).not.toHaveBeenCalled()
    expect(mocks.sockets).toEqual([roomUrl])
    provider.disconnectWs()
  })

  it('stays disconnected and does not retry when signed out', async () => {
    vi.useFakeTimers()
    try {
      mocks.requestSyncTicket.mockResolvedValue({ kind: 'signed-out' })
      const provider = await import('./yjsProvider')
      await vi.waitFor(() => expect(provider.connectionStatus.value).toBe('disconnected'))
      await vi.advanceTimersByTimeAsync(60_000)
      expect(mocks.requestSyncTicket).toHaveBeenCalledTimes(1)
      expect(mocks.sockets).toEqual([])
      provider.disconnectWs()
    } finally {
      vi.useRealTimers()
    }
  })

  it('connects once a signed-out page gets its session back', async () => {
    mocks.requestSyncTicket
      .mockResolvedValueOnce({ kind: 'signed-out' })
      .mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=restored` })
    const provider = await import('./yjsProvider')
    await vi.waitFor(() => expect(provider.connectionStatus.value).toBe('disconnected'))
    expect(mocks.sockets).toEqual([])

    window.dispatchEvent(new Event('library-auth-change'))
    await vi.waitFor(() => expect(mocks.sockets).toEqual([`${roomUrl}&ticket=restored`]))
    provider.disconnectWs()
  })

  it('closes the room when the session ends, and does not reopen it for another account', async () => {
    mocks.requestSyncTicket.mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=one` })
    const provider = await import('./yjsProvider')
    await vi.waitFor(() => expect(mocks.instances).toHaveLength(1))

    mocks.userId = undefined
    window.dispatchEvent(new Event('library-auth-change'))
    expect(mocks.instances[0].readyState).toBe(FakeWebSocket.CLOSED)
    expect(provider.connectionStatus.value).toBe('disconnected')

    // Signed in as someone else without a reload: this page's room is not theirs.
    mocks.userId = 'us_someone_else'
    window.dispatchEvent(new Event('library-auth-change'))
    window.dispatchEvent(new Event('online'))
    expect(mocks.requestSyncTicket).toHaveBeenCalledTimes(1)
    expect(mocks.instances).toHaveLength(1)
    provider.disconnectWs()
  })

  it('closes the room when another tab signs out', async () => {
    mocks.requestSyncTicket.mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=one` })
    const provider = await import('./yjsProvider')
    await vi.waitFor(() => expect(mocks.instances).toHaveLength(1))

    // An unrelated key leaves the room alone.
    window.dispatchEvent(new StorageEvent('storage', { key: 'library-client-theme' }))
    mocks.userId = undefined
    window.dispatchEvent(new StorageEvent('storage', { key: 'library-client-theme' }))
    expect(mocks.instances[0].readyState).not.toBe(FakeWebSocket.CLOSED)

    window.dispatchEvent(new StorageEvent('storage', { key: 'library_auth' }))
    expect(mocks.instances[0].readyState).toBe(FakeWebSocket.CLOSED)
    expect(provider.connectionStatus.value).toBe('disconnected')
    provider.disconnectWs()
  })

  it('reconnects a page shown again while its ticket request was in flight', async () => {
    const first = deferred<SyncTicketResult>()
    mocks.requestSyncTicket
      .mockReturnValueOnce(first.promise)
      .mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=fresh` })
    const provider = await import('./yjsProvider')

    window.dispatchEvent(pageEvent('pagehide', true))
    window.dispatchEvent(pageEvent('pageshow', true))
    first.resolve({ kind: 'ticket', url: `${roomUrl}&ticket=stale` })

    await vi.waitFor(() => expect(mocks.sockets).toEqual([`${roomUrl}&ticket=fresh`]))
    expect(mocks.requestSyncTicket).toHaveBeenCalledTimes(2)
    provider.disconnectWs()
  })

  it('uploads nothing on join when the room already has the whole document', async () => {
    mocks.requestSyncTicket.mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=one` })
    const provider = await import('./yjsProvider')
    provider.recordsArray.push([new Y.Map<string>()])
    await vi.waitFor(() => expect(mocks.instances).toHaveLength(1))

    const socket = mocks.instances[0] as unknown as FakeWebSocket
    socket.join(Y.encodeStateAsUpdate(provider.ydoc))

    expect(mocks.sent).toEqual([])
    provider.disconnectWs()
  })

  it('uploads only what the room lacks, once its stored state has arrived', async () => {
    mocks.requestSyncTicket.mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=one` })
    const provider = await import('./yjsProvider')
    const known = new Y.Map<string>()
    known.set('id', 'known')
    provider.recordsArray.push([known])
    const roomState = Y.encodeStateAsUpdate(provider.ydoc)
    const offline = new Y.Map<string>()
    offline.set('id', 'offline')
    provider.recordsArray.push([offline])
    await vi.waitFor(() => expect(mocks.instances).toHaveLength(1))

    const socket = mocks.instances[0] as unknown as FakeWebSocket
    socket.join(roomState)

    expect(mocks.sent).toHaveLength(1)
    const sent = mocks.sent[0] as Uint8Array
    expect(sent.byteLength).toBeLessThan(Y.encodeStateAsUpdate(provider.ydoc).byteLength)
    const room = new Y.Doc()
    Y.applyUpdate(room, roomState)
    Y.applyUpdate(room, sent)
    expect(room.getArray<Y.Map<string>>('records').toArray().map((m) => m.get('id'))).toEqual([
      'known',
      'offline',
    ])
    provider.disconnectWs()
  })

  it('waits for the Cloudflare relay\'s presence frame, however long its stored state takes', async () => {
    vi.useFakeTimers()
    try {
      mocks.requestSyncTicket.mockResolvedValue({ kind: 'ticket', url: `${roomUrl}&ticket=one` })
      const provider = await import('./yjsProvider')
      const known = new Y.Map<string>()
      provider.recordsArray.push([known])
      const roomState = Y.encodeStateAsUpdate(provider.ydoc)
      await vi.waitFor(() => expect(mocks.instances).toHaveLength(1))
      const socket = mocks.instances[0] as unknown as FakeWebSocket
      socket.readyState = FakeWebSocket.OPEN
      socket.dispatchEvent(new Event('open'))

      // Another client's update arrives before the room has read its storage.
      const other = new Y.Doc()
      other.getArray<string>('records').push(['from another client'])
      socket.dispatchEvent(
        new MessageEvent('message', { data: Y.encodeStateAsUpdate(other).slice().buffer })
      )
      await vi.advanceTimersByTimeAsync(5_000)
      expect(mocks.sent).toEqual([])

      socket.dispatchEvent(new MessageEvent('message', { data: roomState.slice().buffer }))
      socket.dispatchEvent(
        new MessageEvent('message', { data: JSON.stringify({ type: 'presence', onlineCount: 2 }) })
      )
      expect(mocks.sent).toEqual([])
      provider.disconnectWs()
    } finally {
      vi.useRealTimers()
    }
  })

  it('uploads to a Rust server once its stored state stops arriving', async () => {
    vi.useFakeTimers()
    try {
      mocks.sync.backend = 'rust-server'
      const provider = await import('./yjsProvider')
      const offline = new Y.Map<string>()
      provider.recordsArray.push([offline])
      const socket = mocks.instances[0] as unknown as FakeWebSocket
      socket.readyState = FakeWebSocket.OPEN
      socket.dispatchEvent(new Event('open'))
      socket.dispatchEvent(new MessageEvent('message', { data: new Uint8Array([0, 0]).buffer }))
      expect(mocks.sent).toEqual([])

      await vi.advanceTimersByTimeAsync(1_000)
      expect(mocks.sent).toHaveLength(1)
      provider.disconnectWs()
    } finally {
      vi.useRealTimers()
    }
  })
})

