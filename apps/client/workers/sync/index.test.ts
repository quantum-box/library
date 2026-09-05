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
  private alarmAt: number | null = null
  private transactionTail = Promise.resolve()

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

  async list<T = unknown>(options: {
    prefix?: string
    startAfter?: string
    limit?: number
  } = {}): Promise<Map<string, T>> {
    const keys = [...this.values.keys()]
      .filter((key) => !options.prefix || key.startsWith(options.prefix))
      .filter((key) => !options.startAfter || key > options.startAfter)
      .sort()
      .slice(0, options.limit ?? 1000)
    return new Map(keys.map((key) => [key, this.values.get(key) as T]))
  }

  async transaction<T>(closure: (transaction: MemoryStorage) => Promise<T>) {
    const run = this.transactionTail.then(() => closure(this), () => closure(this))
    this.transactionTail = run.then(() => undefined, () => undefined)
    return run
  }

  async getAlarm() {
    return this.alarmAt
  }

  async setAlarm(timestamp: number) {
    this.alarmAt = timestamp
    return undefined
  }

  async deleteAlarm() {
    this.alarmAt = null
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

function liveRoomEnvironment(session: typeof liveSession = liveSession, overrides: Record<string, unknown> = {}) {
  return env({
    PHOTON_LIVE_TICKETS: namespace({
      getSession: vi.fn().mockResolvedValue(session),
      deleteSession: vi.fn().mockResolvedValue(undefined),
    }),
    ...overrides,
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
    savedVersion: 0,
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

function internalWebSocketRequest(session: typeof liveSession, target?: string): Request {
  return new Request('https://live.example.test/live/internal-ws', {
    headers: {
      'upgrade': 'websocket',
      'x-photon-live-internal': '1',
      'x-photon-live-session': encodeSessionHeader(session),
      ...(target ? { 'x-photon-live-target': target } : {}),
    },
  })
}

function internalPointerRequest(session: typeof liveSession, roomId: string): Request {
  return new Request('https://live.example.test/live/internal-pointer', {
    method: 'POST',
    headers: {
      'x-photon-live-internal': '1',
      'x-photon-live-session': encodeSessionHeader(session),
      'content-type': 'application/json',
    },
    body: JSON.stringify({
      room_id: roomId,
      body_hash: session.bodyHash,
      record_version: session.recordVersion,
    }),
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

  it('cleans expired ticket and session records in bounded alarm batches', async () => {
    const storage = new MemoryStorage()
    const store = new PhotonLiveTicketStore(ticketStoreContext(storage), {} as never)
    const expired = {
      ...liveSession,
      sessionId: 's'.repeat(24),
      expiresAt: Date.now() - 60_000,
    }
    await storage.put('live:ticket:expired', expired)
    await storage.put('live:session:expired', expired)
    for (let index = 0; index < 129; index += 1) {
      await storage.put(`live:ticket:bulk-${index.toString().padStart(3, '0')}`, expired)
    }

    await store.alarm()
    expect((await storage.list({ prefix: 'live:ticket:' })).size).toBeGreaterThan(0)
    await store.alarm()
    expect((await storage.list({ prefix: 'live:ticket:' })).size).toBe(0)
    expect(await storage.get('live:ticket:expired')).toBeUndefined()
    expect(await storage.get('live:session:expired')).toBeUndefined()
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
      record_version: '7',
      operation_id: 'operation-1',
    })
  })

  it('replays an acknowledged operation while preserving another pending operation', async () => {
    const body = '{"title":"same"}'
    const bodyHash = await normalizeBodyHash('markdown', body)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash }
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
        body,
        record_version: '7',
      }), { status: 200 }),
    )
    const checkpoint = JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body,
      operation_id: 'operation-acknowledged',
    })
    await room.webSocketMessage(socket as unknown as WebSocket, checkpoint)
    const pendingBody = '{"title":"pending"}'
    const pendingBytes = new TextEncoder().encode(pendingBody)
    const pendingDigest = await crypto.subtle.digest('SHA-256', pendingBytes)
    let binary = ''
    for (const byte of new Uint8Array(pendingDigest)) binary += String.fromCharCode(byte)
    const pendingFingerprint = btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
    await storage.put('live:checkpoint:pending:v1', {
      version: 0,
      operationId: 'operation-pending',
      expectedRecordVersion: '7',
      bodyHash: await normalizeBodyHash('markdown', pendingBody),
      fingerprint: pendingFingerprint,
      bodyByteLength: pendingBytes.byteLength,
      chunkCount: 1,
    })
    await storage.put('live:checkpoint:pending:body:000000', pendingBytes.buffer)
    socket.frames.length = 0

    await room.webSocketMessage(socket as unknown as WebSocket, checkpoint)

    expect(fetchMock).toHaveBeenCalledOnce()
    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-saved',
      version: 0,
      record_version: '7',
      operation_id: 'operation-acknowledged',
    })
    expect(await storage.get<{ operationId: string }>('live:checkpoint:pending:v1'))
      .toMatchObject({ operationId: 'operation-pending' })
  })

  it('keeps a newer pending marker when a stale acknowledged retry arrives', async () => {
    const baselineBody = '{"title":"base"}'
    const oldBody = '{"title":"old"}'
    const pendingBody = '{"title":"new"}'
    const baselineHash = await normalizeBodyHash('markdown', baselineBody)
    const digest = async (value: string): Promise<string> => {
      const bytes = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)))
      let binary = ''
      for (const byte of bytes) binary += String.fromCharCode(byte)
      return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
    }
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash: baselineHash, version: 1, recordVersion: '8', savedVersion: 0 })
    await storage.put('live:checkpoint:result:b2xkLW9w', {
      version: 0,
      operationId: 'old-op',
      recordVersion: '8',
      bodyHash: await normalizeBodyHash('markdown', oldBody),
      fingerprint: await digest(oldBody),
      expiresAt: Date.now() + 60_000,
    })
    const pendingBytes = new TextEncoder().encode(pendingBody)
    await storage.put('live:checkpoint:pending:v1', {
      version: 1,
      operationId: 'new-op',
      expectedRecordVersion: '8',
      bodyHash: await normalizeBodyHash('markdown', pendingBody),
      fingerprint: await digest(pendingBody),
      bodyByteLength: pendingBytes.byteLength,
      chunkCount: 1,
    })
    await storage.put('live:checkpoint:pending:body:000000', pendingBytes.buffer)
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash: baselineHash, recordVersion: '8' }
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof session> }).sessions.set(
      socket as unknown as WebSocket,
      session,
    )

    await room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body: oldBody,
      operation_id: 'old-op',
    }))

    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-saved',
      version: 0,
      record_version: '8',
      operation_id: 'old-op',
    })
    expect(await storage.get<{ operationId: string }>('live:checkpoint:pending:v1'))
      .toMatchObject({ operationId: 'new-op', version: 1 })
  })

  it('does not replace a newer pending marker when an unacknowledged stale retry arrives', async () => {
    const baselineBody = '{"title":"base"}'
    const oldBody = '{"title":"old"}'
    const pendingBody = '{"title":"new"}'
    const baselineHash = await normalizeBodyHash('markdown', baselineBody)
    const digest = async (value: string): Promise<string> => {
      const bytes = new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)))
      let binary = ''
      for (const byte of bytes) binary += String.fromCharCode(byte)
      return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
    }
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash: baselineHash, version: 1, recordVersion: '8', savedVersion: 0 })
    const pendingBytes = new TextEncoder().encode(pendingBody)
    await storage.put('live:checkpoint:pending:v1', {
      version: 1,
      operationId: 'new-op',
      expectedRecordVersion: '8',
      bodyHash: await normalizeBodyHash('markdown', pendingBody),
      fingerprint: await digest(pendingBody),
      bodyByteLength: pendingBytes.byteLength,
      chunkCount: 1,
    })
    await storage.put('live:checkpoint:pending:body:000000', pendingBytes.buffer)
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash: baselineHash, recordVersion: '8' }
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
        body: baselineBody,
        record_version: '8',
      }), { status: 200 }),
    )

    await room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body: oldBody,
      operation_id: 'old-op',
    }))

    expect(fetchMock).toHaveBeenCalledOnce()
    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-error',
      message: 'Checkpoint is behind the working version',
      code: 'CHECKPOINT_STALE',
      operation_id: 'old-op',
    })
    expect(await storage.get<{ operationId: string; version: number }>('live:checkpoint:pending:v1'))
      .toMatchObject({ operationId: 'new-op', version: 1 })
  })

  it('does not hold the room lock while authorization and checkpoint API are pending', async () => {
    const baselineBody = '{"title":"base"}'
    const body = '{"title":"edited"}'
    const bodyHash = await normalizeBodyHash('markdown', baselineBody)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash }
    ;(room as unknown as { sessions: WeakMap<WebSocket, typeof session> }).sessions.set(
      socket as unknown as WebSocket,
      session,
    )
    const authorizationResponse = new Response(JSON.stringify({
      tenant_id: 'tenant',
      database_id: 'database',
      data_id: 'data',
      property_id: 'property',
      actor_id: 'actor',
      room_id: 'room',
      format: 'markdown',
      body: baselineBody,
      record_version: '7',
    }), { status: 200 })
    let resolveCheckpoint!: (response: Response) => void
    const checkpointPending = new Promise<Response>((resolve) => {
      resolveCheckpoint = resolve
    })
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockImplementation(async () => {
      if (fetchMock.mock.calls.length === 1) return authorizationResponse
      return checkpointPending
    })
    const checkpointPromise = room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body,
      operation_id: 'operation-lock-free',
    }))
    await new Promise((resolve) => setTimeout(resolve, 0))
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(fetchMock).toHaveBeenCalledTimes(2)

    const doc = new Y.Doc()
    doc.getText('body').insert(0, 'concurrent')
    const update = Y.encodeStateAsUpdate(doc)
    doc.destroy()
    await room.webSocketMessage(socket as unknown as WebSocket, update)
    expect(context.upstreamMessages).toHaveLength(1)

    resolveCheckpoint(new Response(JSON.stringify({ record_version: '8' }), { status: 200 }))
    await checkpointPromise
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('lets two callers save the same body without replacing an active checkpoint', async () => {
    let canonicalBody = 'original'
    let canonicalVersion = '7'
    const bodyHash = await normalizeBodyHash('markdown', canonicalBody)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash })
    const first = new FakeSocket()
    const second = new FakeSocket()
    const room = new PhotonLiveRoom(roomContext(storage, [first, second]) as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash }
    const sessions = (room as unknown as { sessions: WeakMap<WebSocket, typeof session> }).sessions
    sessions.set(first as unknown as WebSocket, session)
    sessions.set(second as unknown as WebSocket, session)
    let finishWrite!: () => void
    const writeGate = new Promise<void>((resolve) => { finishWrite = resolve })
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
      if (String(input).endsWith('/authorize')) {
        return new Response(JSON.stringify({
          tenant_id: 'tenant', database_id: 'database', data_id: 'data', property_id: 'property',
          actor_id: 'actor', room_id: 'room', format: 'markdown',
          body: canonicalBody, record_version: canonicalVersion,
        }), { status: 200 })
      }
      await writeGate
      canonicalBody = 'shared edit'
      canonicalVersion = '8'
      return new Response(JSON.stringify({ record_version: canonicalVersion }), { status: 200 })
    })
    const checkpoint = (operationId: string) => JSON.stringify({
      type: 'live-checkpoint', version: 0, body: 'shared edit', operation_id: operationId,
    })
    const firstSave = room.webSocketMessage(first as unknown as WebSocket, checkpoint('first-save'))
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
    const secondSave = room.webSocketMessage(second as unknown as WebSocket, checkpoint('second-save'))
    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(fetchMock).toHaveBeenCalledTimes(2)
    finishWrite()
    await Promise.all([firstSave, secondSave])
    expect(fetchMock).toHaveBeenCalledTimes(3) // Two authorizations, one durable write.
    for (const [socket, operationId] of [[first, 'first-save'], [second, 'second-save']] as const) {
      const messages = socket.frames.filter((frame) => typeof frame === 'string').map((frame) => JSON.parse(String(frame)))
      expect(messages).toContainEqual(expect.objectContaining({ type: 'live-saved', operation_id: operationId }))
      expect(messages.some((message) => ['live-error', 'live-conflict'].includes(message.type))).toBe(false)
    }
    expect(await storage.get('live:checkpoint:pending:v1')).toBeUndefined()
  })

  it('bounds checkpoint replay records and migrates a legacy result key', async () => {
    const storage = new MemoryStorage()
    const context = roomContext(storage)
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const save = (room as unknown as {
      saveCheckpointResult: (result: Record<string, unknown>) => Promise<void>
    }).saveCheckpointResult.bind(room)
    for (let index = 0; index < 130; index += 1) {
      await save({
        version: index,
        operationId: `operation-${index}`,
        recordVersion: String(index),
        bodyHash: `hash-${index}`,
        fingerprint: `fingerprint-${index}`,
        expiresAt: Date.now() + 60_000,
      })
    }
    expect((await storage.list({ prefix: 'live:checkpoint:result:' })).size).toBeLessThanOrEqual(128)

    const legacyKey = 'live:checkpoint:result:bGVnYWN5LW9wZXJhdGlvbg'
    await storage.put(legacyKey, {
      version: 1,
      operationId: 'legacy-operation',
      recordVersion: '1',
      bodyHash: 'legacy-hash',
      fingerprint: 'legacy-fingerprint',
    })
    const read = (room as unknown as {
      readCheckpointResult: (operationId: string) => Promise<{ expiresAt: number } | null>
    }).readCheckpointResult.bind(room)
    await expect(read('legacy-operation')).resolves.toMatchObject({ expiresAt: expect.any(Number) })
    expect(await storage.get('live:checkpoint:result-index:v1')).toBeDefined()
  })

  it('keeps a result needed by an exact pending retry when cleanup runs', async () => {
    const storage = new MemoryStorage()
    await putRoomMetadata(storage)
    await storage.put('live:checkpoint:pending:v1', {
      version: 0,
      operationId: 'pending-operation',
      expectedRecordVersion: '7',
      bodyHash: 'hash',
      fingerprint: 'fingerprint',
      bodyByteLength: 0,
      chunkCount: 0,
    })
    const room = new PhotonLiveRoom(roomContext(storage) as never, liveRoomEnvironment())
    const save = (room as unknown as {
      saveCheckpointResult: (result: Record<string, unknown>) => Promise<void>
    }).saveCheckpointResult.bind(room)
    await save({
      version: 0,
      operationId: 'pending-operation',
      recordVersion: '8',
      bodyHash: 'hash',
      fingerprint: 'fingerprint',
      expiresAt: Date.now() - 1,
    })
    await room.alarm()
    expect(await storage.get('live:checkpoint:result:cGVuZGluZy1vcGVyYXRpb24')).toBeDefined()
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

  it('acknowledges an already-canonical body without calling checkpoint API', async () => {
    const canonicalBody = '{"title":"same"}'
    const canonicalHash = await normalizeBodyHash('markdown', canonicalBody)
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { bodyHash: canonicalHash, recordVersion: '7' })
    const socket = new FakeSocket()
    const context = roomContext(storage, [socket])
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment())
    const session = { ...liveSession, bodyHash: canonicalHash }
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
        body: canonicalBody,
        record_version: '8',
      }), { status: 200 }),
    )

    await room.webSocketMessage(socket as unknown as WebSocket, JSON.stringify({
      type: 'live-checkpoint',
      version: 0,
      body: canonicalBody,
      operation_id: 'operation-noop',
    }))

    expect(fetchMock).toHaveBeenCalledOnce()
    expect(JSON.parse(String(socket.frames.at(-1)))).toEqual({
      type: 'live-saved',
      version: 0,
      record_version: '8',
      operation_id: 'operation-noop',
    })
    expect(await storage.get('live:checkpoint:pending:v1')).toBeUndefined()
    expect(await storage.get<{ savedVersion: number; recordVersion: string }>('live:room:meta:v1'))
      .toMatchObject({ savedVersion: 0, recordVersion: '8' })
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
    const session = { ...liveSession, recordVersion: '8' }
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment(session))
    const response = await room.fetch(
      internalWebSocketRequest(session),
    )

    expect(response.status).toBe(200)
    expect(await storage.get<{ recordVersion: string; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ recordVersion: '8', bodyHash: 'body-hash' })
  })

  it('rejects a fresh authorization when the canonical body changed', async () => {
    const storage = new MemoryStorage()
    await putRoomMetadata(storage)
    const context = roomContext(storage)
    const session = { ...liveSession, recordVersion: '8', bodyHash: 'changed-body-hash' }
    const room = new PhotonLiveRoom(context as never, liveRoomEnvironment(session))
    const response = await room.fetch(
      internalWebSocketRequest(session),
    )

    expect(response.status).toBe(409)
    expect(await storage.get<{ recordVersion: string; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ recordVersion: '7', bodyHash: 'body-hash' })
  })

  it('preserves a dirty room and does not advertise a generation after an external body change', async () => {
    const storage = new MemoryStorage()
    await putRoomMetadata(storage, { version: 1, savedVersion: 0 })
    const session = { ...liveSession, recordVersion: '8', bodyHash: 'changed-body-hash' }
    const room = new PhotonLiveRoom(roomContext(storage) as never, liveRoomEnvironment(session))
    const response = await room.fetch(
      internalWebSocketRequest(session),
    )

    expect(response.status).toBe(409)
    expect(response.headers.get('x-photon-live-generation')).toBeNull()
    expect(await storage.get<{ version: number; savedVersion: number; bodyHash: string }>('live:room:meta:v1'))
      .toMatchObject({ version: 1, savedVersion: 0, bodyHash: 'body-hash' })
  })

  it('rotates clean rooms by canonical record version without reusing an old generation', async () => {
    const baseStorage = new MemoryStorage()
    await putRoomMetadata(baseStorage, { bodyHash: 'body-a', recordVersion: '7', savedVersion: 0 })
    const firstSession = { ...liveSession, bodyHash: 'body-b', recordVersion: '8' }
    const baseRoom = new PhotonLiveRoom(roomContext(baseStorage) as never, liveRoomEnvironment(firstSession))
    const first = await baseRoom.fetch(
      internalWebSocketRequest(firstSession),
    )
    const generationB = first.headers.get('x-photon-live-generation')
    expect(first.status).toBe(409)
    expect(generationB).toMatch(/^live-generation:/)
    expect(await baseStorage.get<{ roomId: string; bodyHash: string; recordVersion: string }>('live:room:current-generation:v1'))
      .toMatchObject({ roomId: generationB, bodyHash: 'body-b', recordVersion: '8' })

    const generationBStorage = new MemoryStorage()
    await putRoomMetadata(generationBStorage, { bodyHash: 'body-b', recordVersion: '8', savedVersion: 0 })
    const secondSession = { ...liveSession, bodyHash: 'body-c', recordVersion: '9' }
    const pointerStub = {
      fetch: vi.fn(async (request: Request) => {
        const payload = JSON.parse(await request.text()) as {
          room_id: string
          body_hash: string
          record_version: string
        }
        return new Response(JSON.stringify({
          roomId: payload.room_id,
          bodyHash: payload.body_hash,
          recordVersion: payload.record_version,
        }), { status: 200 })
      }),
    }
    const generationBRoom = new PhotonLiveRoom(
      roomContext(generationBStorage) as never,
      liveRoomEnvironment(secondSession, { PHOTON_LIVE_ROOMS: namespace(pointerStub) }),
    )
    const second = await generationBRoom.fetch(
      internalWebSocketRequest(
        secondSession,
        generationB ?? undefined,
      ),
    )
    const generationC = second.headers.get('x-photon-live-generation')
    expect(second.status).toBe(409)
    expect(generationC).toMatch(/^live-generation:/)
    expect(generationC).not.toBe(generationB)

    const generationCStorage = new MemoryStorage()
    await putRoomMetadata(generationCStorage, { bodyHash: 'body-c', recordVersion: '9', savedVersion: 0 })
    const thirdSession = { ...liveSession, bodyHash: 'body-a', recordVersion: '10' }
    const generationCRoom = new PhotonLiveRoom(
      roomContext(generationCStorage) as never,
      liveRoomEnvironment(thirdSession, { PHOTON_LIVE_ROOMS: namespace(pointerStub) }),
    )
    const third = await generationCRoom.fetch(
      internalWebSocketRequest(
        thirdSession,
        generationC ?? undefined,
      ),
    )
    expect(third.headers.get('x-photon-live-generation')).not.toBe(generationB)
  })

  it('returns the winning generation when a stale pointer update races a newer one', async () => {
    const storage = new MemoryStorage()
    const winningSession = {
      ...liveSession,
      sessionId: 'winning-session',
      bodyHash: 'body-d',
      recordVersion: '10',
    }
    const staleSession = {
      ...liveSession,
      sessionId: 'stale-session',
      bodyHash: 'body-c',
      recordVersion: '9',
    }
    const sessions = new Map([
      [winningSession.sessionId, winningSession],
      [staleSession.sessionId, staleSession],
    ])
    const room = new PhotonLiveRoom(roomContext(storage) as never, env({
      PHOTON_LIVE_TICKETS: namespace({
        getSession: vi.fn((sessionId: string) => Promise.resolve(sessions.get(sessionId))),
      }),
    }))
    const winningRoomId = 'live-generation:winning-room'
    const staleRoomId = 'live-generation:stale-room'

    const [winningResponse, staleResponse] = await Promise.all([
      room.fetch(internalPointerRequest(winningSession, winningRoomId)),
      room.fetch(internalPointerRequest(staleSession, staleRoomId)),
    ])

    expect(winningResponse.status).toBe(200)
    expect(staleResponse.status).toBe(200)
    await expect(winningResponse.json()).resolves.toMatchObject({ roomId: winningRoomId })
    await expect(staleResponse.json()).resolves.toMatchObject({ roomId: winningRoomId })
    await expect(storage.get('live:room:current-generation:v1')).resolves.toEqual({
      roomId: winningRoomId,
      bodyHash: winningSession.bodyHash,
      recordVersion: winningSession.recordVersion,
    })
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
      { type: 'live-ready', initialized: true, version: 0, record_version: '7', room_generation: 'live-room' },
      { type: 'live-ready', initialized: true, version: 0, record_version: '7', room_generation: 'live-room' },
    ])
  })
})
