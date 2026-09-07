import { waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import {
  createPhotonLiveProvider,
  requestPhotonLiveSession,
  type PhotonLiveProviderOptions,
} from './client'
import type { PhotonLiveProvider } from './types'

class FakeWebSocket {
  static readonly CONNECTING = 0
  static readonly OPEN = 1
  static readonly CLOSING = 2
  static readonly CLOSED = 3

  readonly url: string
  readonly sent: Array<string | ArrayBuffer | ArrayBufferView> = []
  binaryType = 'blob'
  readyState = FakeWebSocket.CONNECTING
  private readonly listeners = new Map<string, Set<(event: { data?: unknown; code?: number; reason?: string }) => void>>()

  constructor(url: string) {
    this.url = url
  }

  addEventListener(type: string, listener: (event: { data?: unknown; code?: number; reason?: string }) => void): void {
    const listeners = this.listeners.get(type) ?? new Set()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(type: string, listener: (event: { data?: unknown; code?: number; reason?: string }) => void): void {
    this.listeners.get(type)?.delete(listener)
  }

  send(data: string | ArrayBuffer | ArrayBufferView): void {
    this.sent.push(data)
  }

  close(code = 1000, reason = ''): void {
    if (this.readyState === FakeWebSocket.CLOSED) return
    this.readyState = FakeWebSocket.CLOSED
    this.emit('close', { code, reason })
  }

  open(): void {
    this.readyState = FakeWebSocket.OPEN
    this.emit('open', {})
  }

  message(data: unknown): void {
    this.emit('message', { data })
  }

  private emit(type: string, event: { data?: unknown; code?: number; reason?: string }): void {
    this.listeners.get(type)?.forEach((listener) => listener(event))
  }
}

const config = {
  baseUrl: 'https://live.example.test',
  sessionPath: '/live/session',
  websocketPath: '/live/ws',
  fragmentName: 'prosemirror',
  requestTimeoutMs: 1_000,
  checkpointDebounceMs: 0,
} as const

const target = {
  org: 'quantum-box',
  repo: 'photon',
  dataId: 'data-1',
  propertyId: 'body',
  operatorId: 'operator-1',
} as const

let previousWebSocket: typeof WebSocket

function jsonFrame(value: Record<string, unknown>): string {
  return JSON.stringify(value)
}

function sentJson(socket: FakeWebSocket): Array<Record<string, unknown>> {
  return socket.sent.flatMap((frame) => {
    if (typeof frame !== 'string') return []
    try {
      return [JSON.parse(frame) as Record<string, unknown>]
    } catch {
      return []
    }
  })
}

function createFixture() {
  let socket: FakeWebSocket | undefined
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    expect(String(input)).toBe('https://live.example.test/live/session')
    expect(init?.method).toBe('POST')
    return new Response(JSON.stringify({
      ticket: 'ticket-1',
      room_id: 'room-1',
      actor_id: 'actor-1',
      format: 'markdown',
      body: '# Canonical',
      record_version: '1',
    }), { status: 200, headers: { 'content-type': 'application/json' } })
  })
  const fetchImpl = fetchMock as unknown as typeof fetch
  const options: PhotonLiveProviderOptions = {
    target,
    format: 'markdown',
    config,
    seedUpdate: vi.fn(() => Y.encodeStateAsUpdate(new Y.Doc())),
    fetchImpl,
    webSocketFactory: (url) => {
      socket = new FakeWebSocket(url)
      return socket as unknown as WebSocket
    },
  }
  const provider = createPhotonLiveProvider(options)
  return {
    provider,
    fetchImpl,
    fetchMock,
    options,
    getSocket: () => socket,
  }
}

function validSessionPayload() {
  return {
    ticket: 'ticket-1',
    room_id: 'room-1',
    actor_id: 'actor-1',
    format: 'markdown',
    body: '# Canonical',
    record_version: '1',
  }
}

async function flushMicrotasks(): Promise<void> {
  for (let index = 0; index < 10; index += 1) await Promise.resolve()
}

function snapshotWithText(text: string): Uint8Array {
  const doc = new Y.Doc()
  const fragment = doc.getXmlFragment(config.fragmentName)
  if (text) {
    const xmlText = new Y.XmlText()
    xmlText.insert(0, text)
    fragment.insert(0, [xmlText])
  }
  const update = Y.encodeStateAsUpdate(doc)
  doc.destroy()
  return update
}

function appendText(provider: PhotonLiveProvider, text: string): void {
  const xmlText = new Y.XmlText()
  xmlText.insert(0, text)
  provider.fragment.insert(provider.fragment.length, [xmlText])
}

beforeEach(() => {
  previousWebSocket = globalThis.WebSocket
  globalThis.WebSocket = FakeWebSocket as unknown as typeof WebSocket
})

afterEach(() => {
  globalThis.WebSocket = previousWebSocket
})

describe('Photon Live provider', () => {
  it('authorizes the record scope and waits for initialization echo before ready', async () => {
    const fixture = createFixture()
    const states: ReturnType<typeof fixture.provider.getState>[] = []
    const unsubscribe = fixture.provider.subscribe((state) => states.push(state))
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const socket = fixture.getSocket()!

    expect(fixture.fetchMock).toHaveBeenCalledTimes(1)
    const init = fixture.fetchMock.mock.calls[0]?.[1]
    expect(init?.headers).toMatchObject({
      'content-type': 'application/json',
      'x-platform-id': expect.any(String),
      'x-operator-id': 'operator-1',
    })
    expect(JSON.parse(String(init?.body))).toEqual({
      org: 'quantum-box',
      repo: 'photon',
      data_id: 'data-1',
      property_id: 'body',
    })
    expect(socket.url).toBe('wss://live.example.test/live/ws?ticket=ticket-1')

    socket.open()
    socket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    await new Promise((resolve) => setTimeout(resolve, 0))
    socket.message(jsonFrame({
      type: 'live-ready',
      initialized: false,
      version: 0,
      record_version: '1',
    }))
    await waitFor(() => expect(sentJson(socket).some((frame) => frame.type === 'live-initialize')).toBe(true))
    expect(states.at(-1)?.initialized).toBe(false)

    const initialization = sentJson(socket).find((frame) => frame.type === 'live-initialize')
    expect(initialization?.update).toEqual(expect.any(String))
    socket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    socket.message(jsonFrame({
      type: 'live-ready',
      initialized: true,
      version: 1,
      record_version: '1',
    }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    expect(states.at(-1)?.status).toBe('connected')
    expect(states.at(-1)?.initialized).toBe(true)
    expect(fixture.options.seedUpdate).toHaveBeenCalledWith('# Canonical', 'markdown')

    unsubscribe()
    fixture.provider.destroy()
  })

  it('keeps checkpoint operation ids separate and accepts only its own ACK', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const socket = fixture.getSocket()!
    socket.open()
    socket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    await new Promise((resolve) => setTimeout(resolve, 0))
    socket.message(jsonFrame({ type: 'live-ready', initialized: true, version: 2, record_version: '1' }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))

    fixture.provider.queueCheckpoint('first')
    fixture.provider.flushCheckpoint()
    await waitFor(() => expect(sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')).toHaveLength(1))
    const first = sentJson(socket).find((frame) => frame.type === 'live-checkpoint')!

    fixture.provider.queueCheckpoint('second')
    fixture.provider.flushCheckpoint()
    socket.message(jsonFrame({
      type: 'live-saved',
      version: 3,
      record_version: '2',
      operation_id: 'another-actor-operation',
    }))
    expect(fixture.provider.getState().hasUnackedChanges).toBe(true)

    socket.message(jsonFrame({
      type: 'live-saved',
      version: 3,
      record_version: '2',
      operation_id: first.operation_id,
    }))
    await waitFor(() => expect(sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')).toHaveLength(2))
    const checkpoints = sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')
    expect(checkpoints[1]?.operation_id).not.toBe(first.operation_id)
    expect(checkpoints[1]?.body).toBe('second')
    expect(checkpoints[1]?.version).toBe(3)

    socket.message(jsonFrame({
      type: 'live-saved',
      version: 4,
      record_version: '3',
      operation_id: checkpoints[1]?.operation_id,
    }))
    await waitFor(() => expect(fixture.provider.getState().saveStatus).toBe('saved'))
    expect(fixture.provider.getState().hasUnackedChanges).toBe(false)
    fixture.provider.destroy()
  })

  it('resumes the latest queued body after an older checkpoint becomes stale during authorization', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const socket = fixture.getSocket()!
    socket.open()
    socket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    await new Promise((resolve) => setTimeout(resolve, 0))
    socket.message(jsonFrame({ type: 'live-ready', initialized: true, version: 2, record_version: '1' }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    try {
      fixture.provider.queueCheckpoint('old body')
      fixture.provider.flushCheckpoint()
      const first = sentJson(socket).find((frame) => frame.type === 'live-checkpoint')!
      appendText(fixture.provider, 'peer edit')
      fixture.provider.queueCheckpoint('merged body')
      // Its debounce fires while the old request is still in flight.
      fixture.provider.flushCheckpoint()
      socket.message(jsonFrame({ type: 'live-version', version: 3 }))
      socket.message(jsonFrame({
        type: 'live-error', code: 'CHECKPOINT_STALE',
        message: 'Checkpoint is behind the working version', operation_id: first.operation_id,
      }))
      await waitFor(() => expect(sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')).toHaveLength(2))
      const latest = sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')[1]!
      expect(latest).toMatchObject({ body: 'merged body', version: 3 })
      expect(latest.operation_id).not.toBe(first.operation_id)
      expect(fixture.provider.getState().hasUnackedChanges).toBe(true)
      socket.message(jsonFrame({ type: 'live-saved', version: 3, record_version: '2', operation_id: latest.operation_id }))
      await waitFor(() => expect(fixture.provider.getState().saveStatus).toBe('saved'))
    } finally {
      fixture.provider.destroy()
    }
  })

  it('waits for a fresh serialization instead of retrying a pre-merge body', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const socket = fixture.getSocket()!
    socket.open()
    socket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    await flushMicrotasks()
    socket.message(jsonFrame({ type: 'live-ready', initialized: true, version: 2, record_version: '1' }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    try {
      fixture.provider.queueCheckpoint('pre-merge body')
      fixture.provider.flushCheckpoint()
      const first = sentJson(socket).find((frame) => frame.type === 'live-checkpoint')!
      appendText(fixture.provider, 'merged change')
      socket.message(jsonFrame({ type: 'live-version', version: 3 }))
      socket.message(jsonFrame({ type: 'live-error', code: 'CHECKPOINT_STALE', operation_id: first.operation_id }))
      fixture.provider.flushCheckpoint()
      expect(sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')).toHaveLength(1)
      expect(fixture.provider.getState()).toMatchObject({ saveStatus: 'saving', hasUnackedChanges: true })
      fixture.provider.queueCheckpoint('fresh merged body')
      fixture.provider.flushCheckpoint()
      const latest = sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')[1]!
      expect(latest).toMatchObject({ body: 'fresh merged body', version: 3 })
      expect(latest.operation_id).not.toBe(first.operation_id)
      socket.message(jsonFrame({ type: 'live-conflict', operation_id: latest.operation_id }))
      expect(fixture.provider.getState().saveStatus).toBe('conflict')
      fixture.provider.flushCheckpoint()
      expect(sentJson(socket).filter((frame) => frame.type === 'live-checkpoint')).toHaveLength(2)
    } finally {
      fixture.provider.destroy()
    }
  })

  it('marks the editor read only offline and reauthorizes on reconnect', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const firstSocket = fixture.getSocket()!
    firstSocket.open()
    firstSocket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    await new Promise((resolve) => setTimeout(resolve, 0))
    firstSocket.message(jsonFrame({ type: 'live-ready', initialized: true, version: 1, record_version: '1' }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))

    window.dispatchEvent(new Event('offline'))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(false))
    expect(firstSocket.readyState).toBe(FakeWebSocket.CLOSED)

    window.dispatchEvent(new Event('online'))
    await waitFor(() => expect(fixture.fetchMock).toHaveBeenCalledTimes(2))
    const secondSocket = fixture.getSocket()!
    expect(secondSocket).not.toBe(firstSocket)
    secondSocket.open()
    secondSocket.message(Y.encodeStateAsUpdate(new Y.Doc()))
    await new Promise((resolve) => setTimeout(resolve, 0))
    secondSocket.message(jsonFrame({ type: 'live-ready', initialized: true, version: 1, record_version: '1' }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    fixture.provider.destroy()
  })

  it('recognizes the worker 404 LIVE_DISABLED response without treating generic 404 as disabled', async () => {
    const disabledFetch = vi.fn(async () => new Response(JSON.stringify({
      code: 'LIVE_DISABLED',
      error: 'Photon Live is disabled',
    }), { status: 404, headers: { 'content-type': 'application/json' } }))
    await expect(requestPhotonLiveSession(target, config, disabledFetch)).rejects.toMatchObject({
      kind: 'disabled',
      status: 404,
      retryable: false,
    })

    const missingFetch = vi.fn(async () => new Response(JSON.stringify({
      error: 'record not found',
    }), { status: 404, headers: { 'content-type': 'application/json' } }))
    await expect(requestPhotonLiveSession(target, config, missingFetch)).rejects.toMatchObject({
      kind: 'server',
      status: 404,
      retryable: false,
    })
  })

  it('retries a transient initial session failure and does not retry permanent authorization failures', async () => {
    vi.useFakeTimers()
    try {
      const initialBackoffMs = 1_000
      const responses = [
        new Response(JSON.stringify({ error: 'ticket store unavailable' }), { status: 503 }),
        new Response(JSON.stringify(validSessionPayload()), { status: 200 }),
      ]
      const fetchMock = vi.fn(async () => responses.shift() ?? new Response('{}', { status: 500 }))
      const sockets: FakeWebSocket[] = []
      const provider = createPhotonLiveProvider({
        target,
        format: 'markdown',
        config,
        seedUpdate: vi.fn(() => Y.encodeStateAsUpdate(new Y.Doc())),
        fetchImpl: fetchMock as unknown as typeof fetch,
        webSocketFactory: (url) => {
          const socket = new FakeWebSocket(url)
          sockets.push(socket)
          return socket as unknown as WebSocket
        },
      })

      await flushMicrotasks()
      await vi.advanceTimersByTimeAsync(0)
      expect(fetchMock).toHaveBeenCalledTimes(1)
      expect(provider.getState().error).toMatchObject({ kind: 'server', status: 503, retryable: true })
      await vi.advanceTimersByTimeAsync(initialBackoffMs - 1)
      expect(fetchMock).toHaveBeenCalledTimes(1)
      await vi.advanceTimersByTimeAsync(1)
      await flushMicrotasks()
      expect(fetchMock).toHaveBeenCalledTimes(2)
      expect(sockets).toHaveLength(1)
      provider.destroy()

      const deniedFetch = vi.fn(async () => new Response(JSON.stringify({
        error: 'forbidden',
      }), { status: 403 }))
      const deniedProvider = createPhotonLiveProvider({
        target,
        format: 'markdown',
        config,
        seedUpdate: vi.fn(() => Y.encodeStateAsUpdate(new Y.Doc())),
        fetchImpl: deniedFetch as unknown as typeof fetch,
        webSocketFactory: (url) => new FakeWebSocket(url) as unknown as WebSocket,
      })
      await flushMicrotasks()
      expect(deniedFetch).toHaveBeenCalledTimes(1)
      await vi.advanceTimersByTimeAsync(initialBackoffMs * 2)
      expect(deniedFetch).toHaveBeenCalledTimes(1)
      deniedProvider.destroy()
    } finally {
      vi.useRealTimers()
    }
  })

  it('retries an initial session timeout after the timeout backoff', async () => {
    vi.useFakeTimers()
    try {
      const initialBackoffMs = 1_000
      let attempt = 0
      const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
        attempt += 1
        if (attempt === 1) {
          return await new Promise<Response>((_resolve, reject) => {
            init?.signal?.addEventListener('abort', () => {
              reject(new DOMException('Aborted', 'AbortError'))
            }, { once: true })
          })
        }
        return new Response(JSON.stringify(validSessionPayload()), { status: 200 })
      })
      const sockets: FakeWebSocket[] = []
      const provider = createPhotonLiveProvider({
        target,
        format: 'markdown',
        config,
        seedUpdate: vi.fn(() => Y.encodeStateAsUpdate(new Y.Doc())),
        fetchImpl: fetchMock as unknown as typeof fetch,
        webSocketFactory: (url) => {
          const socket = new FakeWebSocket(url)
          sockets.push(socket)
          return socket as unknown as WebSocket
        },
      })

      await flushMicrotasks()
      expect(fetchMock).toHaveBeenCalledTimes(1)
      await vi.advanceTimersByTimeAsync(config.requestTimeoutMs)
      await flushMicrotasks()
      expect(provider.getState().error).toMatchObject({ kind: 'timeout', retryable: true })
      await vi.advanceTimersByTimeAsync(initialBackoffMs - 1)
      expect(fetchMock).toHaveBeenCalledTimes(1)
      await vi.advanceTimersByTimeAsync(1)
      await flushMicrotasks()
      expect(fetchMock).toHaveBeenCalledTimes(2)
      expect(sockets).toHaveLength(1)
      provider.destroy()
    } finally {
      vi.useRealTimers()
    }
  })

  it('retains the local Y.Doc and suppresses reconnect after a 4410 generation close', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const socket = fixture.getSocket()!
    socket.open()
    socket.message(snapshotWithText(''))
    await flushMicrotasks()
    socket.message(jsonFrame({
      type: 'live-ready',
      initialized: true,
      version: 1,
      record_version: '1',
      room_generation: 'generation-1',
    }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))

    appendText(fixture.provider, 'local text')
    const beforeClose = fixture.provider.fragment.toJSON()
    expect(beforeClose).toContain('local text')

    vi.useFakeTimers()
    try {
      socket.close(4410, 'Live canonical body changed; reconnect required')

      expect(fixture.provider.fragment.toJSON()).toBe(beforeClose)
      expect(fixture.provider.getState()).toMatchObject({
        status: 'failed',
        saveStatus: 'conflict',
        canEdit: false,
        error: {
          kind: 'conflict',
          status: 409,
          message: expect.stringContaining('reload'),
        },
      })

      window.dispatchEvent(new Event('online'))
      await vi.advanceTimersByTimeAsync(60_000)
      expect(fixture.fetchMock).toHaveBeenCalledTimes(1)
      expect(fixture.provider.fragment.toJSON()).toBe(beforeClose)
    } finally {
      vi.useRealTimers()
      fixture.provider.destroy()
    }
  })

  it('rejects a rotated room snapshot without applying it to the retained document', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const firstSocket = fixture.getSocket()!
    firstSocket.open()
    firstSocket.message(snapshotWithText(''))
    await flushMicrotasks()
    firstSocket.message(jsonFrame({
      type: 'live-ready',
      initialized: true,
      version: 1,
      record_version: '1',
      room_generation: 'generation-1',
    }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    appendText(fixture.provider, 'old generation text')
    const retainedBody = fixture.provider.fragment.toJSON()

    window.dispatchEvent(new Event('offline'))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(false))
    window.dispatchEvent(new Event('online'))
    await waitFor(() => expect(fixture.fetchMock).toHaveBeenCalledTimes(2))
    const secondSocket = fixture.getSocket()!
    secondSocket.open()
    secondSocket.message(snapshotWithText('new generation text'))
    await flushMicrotasks()
    secondSocket.message(jsonFrame({
      type: 'live-ready',
      initialized: true,
      version: 2,
      record_version: '2',
      room_generation: 'generation-2',
    }))

    await waitFor(() => expect(fixture.provider.getState().saveStatus).toBe('conflict'))
    expect(fixture.provider.getState()).toMatchObject({
      status: 'failed',
      canEdit: false,
      error: {
        kind: 'conflict',
        message: expect.stringContaining('reload'),
      },
    })
    expect(fixture.provider.fragment.toJSON()).toBe(retainedBody)
    expect(fixture.provider.fragment.toJSON()).not.toContain('new generation text')

    window.dispatchEvent(new Event('online'))
    await flushMicrotasks()
    expect(fixture.fetchMock).toHaveBeenCalledTimes(2)
    fixture.provider.destroy()
  })

  it('merges a reconnect snapshot when the room generation is unchanged', async () => {
    const fixture = createFixture()
    await waitFor(() => expect(fixture.getSocket()).toBeDefined())
    const firstSocket = fixture.getSocket()!
    firstSocket.open()
    firstSocket.message(snapshotWithText(''))
    await flushMicrotasks()
    firstSocket.message(jsonFrame({
      type: 'live-ready',
      initialized: true,
      version: 1,
      record_version: '1',
      room_generation: 'generation-1',
    }))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    appendText(fixture.provider, 'retained local text')

    window.dispatchEvent(new Event('offline'))
    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(false))
    window.dispatchEvent(new Event('online'))
    await waitFor(() => expect(fixture.fetchMock).toHaveBeenCalledTimes(2))
    const secondSocket = fixture.getSocket()!
    secondSocket.open()
    secondSocket.message(snapshotWithText('remote generation text'))
    await flushMicrotasks()
    secondSocket.message(jsonFrame({
      type: 'live-ready',
      initialized: true,
      version: 2,
      record_version: '2',
      room_generation: 'generation-1',
    }))

    await waitFor(() => expect(fixture.provider.getState().canEdit).toBe(true))
    expect(fixture.provider.fragment.toJSON()).toContain('retained local text')
    expect(fixture.provider.fragment.toJSON()).toContain('remote generation text')
    fixture.provider.destroy()
  })
})
