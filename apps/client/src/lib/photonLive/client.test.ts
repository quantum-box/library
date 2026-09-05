import { waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import {
  createPhotonLiveProvider,
  type PhotonLiveProviderOptions,
} from './client'

class FakeWebSocket {
  static readonly CONNECTING = 0
  static readonly OPEN = 1
  static readonly CLOSING = 2
  static readonly CLOSED = 3

  readonly url: string
  readonly sent: Array<string | ArrayBuffer | ArrayBufferView> = []
  binaryType = 'blob'
  readyState = FakeWebSocket.CONNECTING
  private readonly listeners = new Map<string, Set<(event: { data?: unknown }) => void>>()

  constructor(url: string) {
    this.url = url
  }

  addEventListener(type: string, listener: (event: { data?: unknown }) => void): void {
    const listeners = this.listeners.get(type) ?? new Set()
    listeners.add(listener)
    this.listeners.set(type, listeners)
  }

  removeEventListener(type: string, listener: (event: { data?: unknown }) => void): void {
    this.listeners.get(type)?.delete(listener)
  }

  send(data: string | ArrayBuffer | ArrayBufferView): void {
    this.sent.push(data)
  }

  close(): void {
    if (this.readyState === FakeWebSocket.CLOSED) return
    this.readyState = FakeWebSocket.CLOSED
    this.emit('close', {})
  }

  open(): void {
    this.readyState = FakeWebSocket.OPEN
    this.emit('open', {})
  }

  message(data: unknown): void {
    this.emit('message', { data })
  }

  private emit(type: string, event: { data?: unknown }): void {
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
})
