import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { SyncTicketResult } from './syncTicket'

const room = 'tenant:library:workspace:library-default:user:us_01abc:records'
const roomUrl = `wss://sync.example.test/ws?room=${room}`

const mocks = vi.hoisted(() => ({
  requestSyncTicket: vi.fn<(url: string) => Promise<SyncTicketResult>>(),
  sockets: [] as string[],
  instances: [] as Array<{ readyState: number }>,
  userId: 'us_01abc' as string | undefined,
}))

vi.mock('../auth', () => ({
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
    sync: {
      yjsArrayName: 'records',
      databasesArrayName: 'databases',
      databaseViewsArrayName: 'databaseViews',
      workflowCanvasesMapName: 'workflowCanvases',
      persistenceKey: 'test',
      workspaceScope: 'tenant:library:workspace:library-default',
      recordsRoomId: room,
      websocketUrl: roomUrl,
      websocketPath: `/ws?room=${room}`,
    },
    attachments: { yjsArrayName: 'attachments' },
  },
  buildRecordsRoomId: (scope: string, actor: string | undefined) =>
    actor ? `${scope}:user:${actor}:records` : undefined,
  resolveBrowserSyncWebsocketUrl: (configured: string | undefined) => configured,
}))

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
  send() {}
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
    mocks.userId = 'us_01abc'
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
})
