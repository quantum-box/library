import { beforeEach, describe, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import worker, { normalizeBodyHash, PhotonLiveRoom, PhotonLiveTicketStore } from './index'

const origin = 'http://127.0.0.1:5187'

function namespace(stub?: Record<string, unknown>) {
  return {
    idFromName: vi.fn((name: string) => ({ name })),
    get: vi.fn(() => stub ?? {}),
  }
}

function env(overrides: Record<string, unknown> = {}) {
  return {
    PHOTON_SYNC_ROOMS: namespace(),
    PHOTON_LIVE_ROOMS: namespace(),
    PHOTON_LIVE_TICKETS: namespace(),
    PHOTON_LIVE_ENABLED: 'true',
    PHOTON_LIVE_ALLOWED_ORIGINS: origin,
    PHOTON_LIVE_API_BASE_URL: 'http://api.example.test',
    ...overrides,
  } as never
}

function request(body: unknown, headers: Record<string, string> = {}) {
  return new Request('https://live.example.test/live/session', {
    method: 'POST',
    headers: { Origin: origin, 'Content-Type': 'application/json', ...headers },
    body: JSON.stringify(body),
  })
}

class MemoryStorage {
  private readonly values = new Map<string, unknown>()

  async get<T = unknown>(key: string): Promise<T | undefined>
  async get<T = unknown>(key: string[]): Promise<Map<string, T>>
  async get<T = unknown>(key: string | string[]): Promise<T | undefined | Map<string, T>> {
    if (Array.isArray(key)) {
      return new Map(
        key.flatMap((entry) => this.values.has(entry)
          ? [[entry, this.values.get(entry) as T]]
          : []),
      )
    }
    return this.values.get(key) as T | undefined
  }

  async put(key: string | Record<string, unknown>, value?: unknown) {
    if (typeof key === 'string') {
      this.values.set(key, value)
      return
    }
    for (const [entry, entryValue] of Object.entries(key)) this.values.set(entry, entryValue)
  }

  async delete(key: string | string[]) {
    if (Array.isArray(key)) return key.reduce((count, entry) => count + Number(this.values.delete(entry)), 0)
    return this.values.delete(key)
  }

  async transaction<T>(closure: (transaction: MemoryStorage) => Promise<T>) {
    return closure(this)
  }

  async getAlarm() {
    return null
  }

  async setAlarm() {
    return undefined
  }

  async deleteAlarm() {
    return undefined
  }
}

class FakeSocket {
  readonly frames: Array<string | ArrayBuffer | ArrayBufferView> = []
  closed: { code: number; reason: string } | null = null
  private attachment: unknown = null

  send(frame: string | ArrayBuffer | ArrayBufferView): void {
    this.frames.push(frame)
  }

  close(code = 1000, reason = ''): void {
    this.closed = { code, reason }
  }

  serializeAttachment(value: unknown): void {
    this.attachment = value
  }

  deserializeAttachment(): unknown {
    return this.attachment
  }
}

interface TestRoomContext {
  storage: MemoryStorage
  upstreamMessages: Array<{ sender: WebSocket; message: string | ArrayBuffer | ArrayBufferView }>
  getWebSockets: () => WebSocket[]
  blockConcurrencyWhile<T>(closure: () => Promise<T>): Promise<T>
}

function roomContext(storage: MemoryStorage, sockets: FakeSocket[] = []): TestRoomContext {
  let lock = Promise.resolve()
  return {
    storage,
    upstreamMessages: [] as Array<{ sender: WebSocket; message: string | ArrayBuffer | ArrayBufferView }>,
    getWebSockets: () => sockets as unknown as WebSocket[],
    blockConcurrencyWhile<T>(closure: () => Promise<T>): Promise<T> {
      const run = lock.then(closure, closure)
      lock = run.then(() => undefined, () => undefined)
      return run
    },
  }
}

const liveSession = {
  roomId: 'room',
  tenant: 'tenant',
  database: 'database',
  data: 'data',
  property: 'property',
  format: 'markdown',
  actorId: 'actor',
  recordVersion: '7',
  bodyHash: 'body-hash',
  org: 'org',
  repo: 'repo',
  authorization: 'Bearer user-token',
  expiresAt: Date.now() + 60_000,
  sessionId: 'session-id',
}

function liveRoomEnvironment() {
  return env({
    PHOTON_LIVE_TICKETS: namespace({
      getSession: vi.fn().mockResolvedValue(liveSession),
      deleteSession: vi.fn().mockResolvedValue(undefined),
    }),
  })
}

function putRoomMetadata(storage: MemoryStorage, overrides: Record<string, unknown> = {}) {
  return storage.put('live:room:meta:v1', {
    roomId: 'room',
    tenant: 'tenant',
    database: 'database',
    data: 'data',
    property: 'property',
    format: 'markdown',
    initialized: true,
    version: 0,
    recordVersion: '7',
    bodyHash: 'body-hash',
    ...overrides,
  })
}

function encodeSessionHeader(session: typeof liveSession): string {
  const bytes = new TextEncoder().encode(JSON.stringify(session))
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function internalWebSocketRequest(session: typeof liveSession): Request {
  return new Request('https://live.example.test/live/internal-ws', {
    headers: {
      'upgrade': 'websocket',
      'x-photon-live-internal': '1',
      'x-photon-live-session': encodeSessionHeader(session),
    },
  })
}

function ticketStoreContext(storage: MemoryStorage) {
  return { storage } as never
}

describe('Photon Live edge', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it('returns an explicit disabled code when the Live feature flag is off', async () => {
    const response = await worker.fetch(
      request({ org: 'org', repo: 'repo', data_id: 'data', property_id: 'prop' }),
      env({ PHOTON_LIVE_ENABLED: 'false' }),
    )

    expect(response.status).toBe(404)
    expect(await response.json()).toEqual({
      code: 'LIVE_DISABLED',
      error: 'Photon Live is disabled',
    })
  })

  it('fails closed for an enabled worker with no exact Origin allowlist match', async () => {
    const response = await worker.fetch(
      request({ org: 'org', repo: 'repo', data_id: 'data', property_id: 'prop' }, { Origin: 'https://attacker.example' }),
      env(),
    )

    expect(response.status).toBe(403)
    expect(await response.json()).toEqual({ error: 'Live origin is not allowed' })
  })

  it('does not expose canonical room data when API authorization fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ room_id: 'secret-room', body: 'secret body' }), { status: 403 }),
    )

    const response = await worker.fetch(
      request(
        { org: 'org', repo: 'repo', data_id: 'data', property_id: 'prop' },
        { Authorization: 'Bearer user-token' },
      ),
      env(),
    )

    expect(response.status).toBe(403)
    expect(await response.json()).toEqual({ error: 'Live authorization failed' })
  })

  it('forwards the user Bearer and returns only the short-lived ticket plus canonical seed', async () => {
    const issued = { ticket: 'a'.repeat(43), sessionId: 'b'.repeat(24) }
    const issue = vi.fn().mockResolvedValue(issued)
    const tickets = namespace({ issue })
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockImplementation(async (_input, init) => {
      expect(init?.method).toBe('POST')
      expect(new Headers(init?.headers).get('authorization')).toBe('Bearer user-token')
      expect(new Headers(init?.headers).get('x-platform-id')).toBe('platform')
      const requestBody = JSON.parse(String(init?.body)) as { property_id: string }
      expect(requestBody.property_id).toBe('prop')
      return new Response(JSON.stringify({
        tenant_id: 'tenant',
        database_id: 'database',
        data_id: 'data',
        property_id: 'prop',
        actor_id: 'actor',
        room_id: 'room',
        format: 'markdown',
        body: 'seed',
        record_version: '7',
      }), { status: 200 })
    })

    const response = await worker.fetch(
      request(
        { org: 'org', repo: 'repo', data_id: 'data', property_id: 'prop' },
        { Authorization: 'Bearer user-token', 'x-platform-id': 'platform' },
      ),
      env({ PHOTON_LIVE_TICKETS: tickets }),
    )
    const payload = await response.json() as Record<string, unknown>

    expect(response.status).toBe(200)
    expect(payload).toEqual({
      ticket: issued.ticket,
      room_id: 'room',
      actor_id: 'actor',
      format: 'markdown',
      body: 'seed',
      record_version: '7',
    })
    expect(JSON.stringify(payload)).not.toContain('user-token')
    expect(fetchMock).toHaveBeenCalledOnce()
    expect(issue).toHaveBeenCalledOnce()
  })

  it('keeps the generic /ws route on the upstream handler', async () => {
    const response = await worker.fetch(
      new Request('https://live.example.test/ws?room=room'),
      env({ PHOTON_LIVE_ENABLED: 'false' }),
    )
    expect(response.status).toBe(200)
    expect(await response.text()).toBe('upstream')
  })
})

describe('PhotonLiveTicketStore', () => {
  it('consumes each ticket once and keeps the session lookup separate', async () => {
    const storage = new MemoryStorage()
    const store = new PhotonLiveTicketStore(ticketStoreContext(storage), {} as never)
    const issued = await store.issue({
      roomId: 'room',
      tenant: 'tenant',
      database: 'database',
      data: 'data',
      property: 'property',
      format: 'markdown',
      actorId: 'actor',
      recordVersion: '1',
      bodyHash: 'body-hash',
      org: 'org',
      repo: 'repo',
      authorization: 'Bearer token',
      expiresAt: Date.now() + 60_000,
    })

    const consumed = await store.consume(issued.ticket)
    expect(consumed?.sessionId).toBe(issued.sessionId)
    expect(await store.consume(issued.ticket)).toBeNull()
    expect(await store.getSession(issued.sessionId)).toMatchObject({ roomId: 'room' })
  })
})

describe('PhotonLiveRoom coordination', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
  })

  it('replays an old checkpoint without moving a newer record version backward', async () => {
    const body = '{"title":"first"}'
    const bodyHash = await normalizeBodyHash('markdown', body)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof liveSession> }).sessions.set(
      socket as unknown as WebSocket,
      liveSession,
    )
    vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(new Response(JSON.stringify({
        tenant_id: 'tenant',
        database_id: 'database',
        data_id: 'data',
        property_id: 'property',
        actor_id: 'actor',
        room_id: 'room',
        format: 'markdown',
        body,
        record_version: '7',
      }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ record_version: '8' }), { status: 200 }))

    const checkpoint = JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body,
      operation_id: 'operation-1',
    })
    await room.webSocketMessage(socket as unknown as WebSocket, checkpoint)
    await storage.put('live:room:meta:v1', {
      ...(await storage.get<Record<string, unknown>>('live:room:meta:v1') ?? {}),
      version: 3,
      recordVersion: '11',
    })
    socket.frames.length = 0

    await room.webSocketMessage(socket as unknown as WebSocket, checkpoint)

    const metadata = await storage.get<{ version: number; recordVersion: string }>('live:room:meta:v1')
    expect(metadata).toMatchObject({ version: 3, recordVersion: '11', bodyHash })
    expect(socket.frames).toHaveLength(1)
    expect(JSON.parse(String(socket.frames[0]))).toEqual({
      type: 'live-saved',
      version: 0,
      record_version: '8',
      operation_id: 'operation-1',
    })
  })

  it('releases a pending checkpoint for a new authorized session after a transient API failure', async () => {
    const baselineBody = '{"title":"base"}'
    const body = '{"title":"first"}'
    const bodyHash = await normalizeBodyHash('markdown', baselineBody)
    const nextBodyHash = await normalizeBodyHash('markdown', body)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, actorId: 'actor-new', bodyHash }
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof session> }).sessions.set(
      socket as unknown as WebSocket,
      session,
    )
    const authorizationResponse = new Response(JSON.stringify({
        tenant_id: 'tenant',
        database_id: 'database',
        data_id: 'data',
        property_id: 'property',
        actor_id: 'actor-new',
        room_id: 'room',
        format: 'markdown',
        body: baselineBody,
        record_version: '7',
      }), { status: 200 })
    const fetchMock = vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(authorizationResponse)
      .mockRejectedValueOnce(new Error('temporary API failure'))
      .mockResolvedValueOnce(new Response(JSON.stringify({
        tenant_id: 'tenant',
        database_id: 'database',
        data_id: 'data',
        property_id: 'property',
        actor_id: 'actor-new',
        room_id: 'room',
        format: 'markdown',
        body: baselineBody,
        record_version: '7',
      }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ record_version: '8' }), { status: 200 }))

    const firstCheckpoint = JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body,
      operation_id: 'operation-old',
    })
    await room.webSocketMessage(socket as unknown as WebSocket, firstCheckpoint)
    expect(await storage.get('live:checkpoint:pending:v1')).toBeDefined()
    socket.frames.length = 0

    await room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body,
      operation_id: 'operation-new',
    }))

    expect(fetchMock).toHaveBeenCalledTimes(4)
    expect(await storage.get('live:checkpoint:pending:v1')).toBeUndefined()
    expect(await storage.get<{ recordVersion: string; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ recordVersion: '8', bodyHash: nextBodyHash })
    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-saved',
      version: 0,
      record_version: '8',
      operation_id: 'operation-new',
    })
  })

  it('refreshes the canonical record version before a new checkpoint on an active socket', async () => {
    const baselineBody = '{"title":"base"}'
    const body = '{"title":"edited"}'
    const baselineHash = await normalizeBodyHash('markdown', baselineBody)
    const nextHash = await normalizeBodyHash('markdown', body)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash: baselineHash })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash: baselineHash }
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof session> }).sessions.set(
      socket as unknown as WebSocket,
      session,
    )
    const fetchMock = vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(new Response(JSON.stringify({
        tenant_id: 'tenant',
        database_id: 'database',
        data_id: 'data',
        property_id: 'property',
        actor_id: 'actor',
        room_id: 'room',
        format: 'markdown',
        body: baselineBody,
        record_version: '8',
      }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ record_version: '9' }), { status: 200 }))

    await room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body,
      operation_id: 'operation-after-title-edit',
    }))

    expect(fetchMock).toHaveBeenCalledTimes(2)
    const checkpointRequest = fetchMock.mock.calls[1]?.[1]
    expect(JSON.parse(String(checkpointRequest?.body))).toMatchObject({
      expected_record_version: '8',
      body,
    })
    expect(await storage.get<{ recordVersion: string; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ recordVersion: '9', bodyHash: nextHash })
    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-saved',
      version: 0,
      record_version: '9',
      operation_id: 'operation-after-title-edit',
    })
  })

  it('stops a new checkpoint when fresh authorization reports an external body change', async () => {
    const baselineBody = '{"title":"base"}'
    const externalBody = '{"title":"external"}'
    const baselineHash = await normalizeBodyHash('markdown', baselineBody)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash: baselineHash })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash: baselineHash }
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof session> }).sessions.set(
      socket as unknown as WebSocket,
      session,
    )
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({
        tenant_id: 'tenant',
        database_id: 'database',
        data_id: 'data',
        property_id: 'property',
        actor_id: 'actor',
        room_id: 'room',
        format: 'markdown',
        body: externalBody,
        record_version: '8',
      }), { status: 200 }),
    )

    await room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body: '{"title":"local"}',
      operation_id: 'operation-external-conflict',
    }))

    expect(fetchMock).toHaveBeenCalledOnce()
    expect(await storage.get('live:checkpoint:pending:v1')).toBeUndefined()
    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-conflict',
      operation_id: 'operation-external-conflict',
    })
  })

  it('advances the room record version when a fresh authorization has the same body', async () => {
    const storage = new MemoryStorage()
    await putRoomMetadata(storage)
    const context = roomContext(storage)
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const response = await room.fetch(
      internalWebSocketRequest({ ...liveSession, recordVersion: '8' }),
    )

    expect(response.status).toBe(200)
    expect(await storage.get<{ recordVersion: string; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ recordVersion: '8', bodyHash: 'body-hash' })
  })

  it('rejects a fresh authorization when the canonical body changed', async () => {
    const storage = new MemoryStorage()
    await putRoomMetadata(storage)
    const context = roomContext(storage)
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const response = await room.fetch(
      internalWebSocketRequest({ ...liveSession, recordVersion: '8', bodyHash: 'changed-body-hash' }),
    )

    expect(response.status).toBe(409)
    expect(await storage.get<{ recordVersion: string; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ recordVersion: '7', bodyHash: 'body-hash' })
  })

  it('normalizes RichText object key order before comparing body hashes', async () => {
    await expect(normalizeBodyHash('richText', '{"a":1,"nested":{"b":2,"a":3}}'))
      .resolves.toBe(await normalizeBodyHash('richText', '{"nested":{"a":3,"b":2},"a":1}'))
  })

  it('serializes competing initialization messages and applies only the winner', async () => {
    const storage = new MemoryStorage()
    await storage.put('live:room:meta:v1', {
      roomId: 'room',
      tenant: 'tenant',
      database: 'database',
      data: 'data',
      property: 'property',
      format: 'markdown',
      initialized: false,
      version: 0,
      bodyHash: 'body-hash',
    })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof liveSession> }).sessions.set(
      socket as unknown as WebSocket,
      liveSession,
    )
    const first = new Y.Doc()
    first.getText('body').insert(0, 'winner')
    const update = Y.encodeStateAsUpdate(first)
    first.destroy()

    const initialize = (room as unknown as {
      handleInitialization: (sender: WebSocket, session: typeof liveSession, update: Uint8Array) => Promise<void>
    }).handleInitialization
    await Promise.all([
      initialize.call(room, socket as unknown as WebSocket, liveSession, update),
      initialize.call(room, socket as unknown as WebSocket, liveSession, update),
    ])

    const metadata = await storage.get<{ initialized: boolean; version: number; recordVersion: string }>('live:room:meta:v1')
    expect(metadata).toMatchObject({ initialized: true, version: 0, recordVersion: '7' })
    expect(context.upstreamMessages).toHaveLength(1)
    expect(socket.frames.filter((frame) => typeof frame !== 'string')).toHaveLength(1)
    expect(socket.frames.filter((frame) => typeof frame === 'string').map((frame) => JSON.parse(String(frame)))).toEqual([
      { type: 'live-ready', initialized: true, version: 0, record_version: '7' },
      { type: 'live-ready', initialized: true, version: 0, record_version: '7' },
    ])
  })
})
