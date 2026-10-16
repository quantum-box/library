import { describe, expect, it, vi } from 'vitest'
import { requestSyncTicket } from './syncTicket'

const room = 'tenant:library:workspace:library-default:user:us_01abc:records'
const socketUrl = `wss://sync.example.test/ws?room=${room}`
const signedIn = async () => ({
  'content-type': 'application/json',
  'x-platform-id': 'platform-test',
  'x-operator-id': 'operator-test',
  Authorization: 'Bearer test-only',
})

describe('requestSyncTicket', () => {
  it('asks for a ticket to the user room with their token and opens it with that ticket', async () => {
    const fetchImpl = vi.fn(async () => Response.json({ ticket: 't'.repeat(43), room }))
    const result = await requestSyncTicket(socketUrl, { headers: signedIn, fetchImpl })

    expect(fetchImpl).toHaveBeenCalledTimes(1)
    const [url, init] = fetchImpl.mock.calls[0] as unknown as [string, RequestInit]
    expect(url).toBe('https://sync.example.test/ws/ticket')
    expect(init.method).toBe('POST')
    expect(init.headers).toMatchObject({ Authorization: 'Bearer test-only', 'x-operator-id': 'operator-test' })
    expect(JSON.parse(init.body as string)).toEqual({ room })
    expect(result.kind).toBe('ticket')
    const opened = new URL((result as { url: string }).url)
    expect(opened.searchParams.get('room')).toBe(room)
    expect(opened.searchParams.get('ticket')).toBe('t'.repeat(43))
  })

  it('does not ask at all without a token', async () => {
    const fetchImpl = vi.fn()
    const result = await requestSyncTicket(socketUrl, {
      headers: async () => ({ 'content-type': 'application/json' }),
      fetchImpl,
    })
    expect(result).toEqual({ kind: 'signed-out' })
    expect(fetchImpl).not.toHaveBeenCalled()
  })

  it('does not ask for a socket URL that names no room', async () => {
    const fetchImpl = vi.fn()
    const result = await requestSyncTicket('wss://sync.example.test/ws', { headers: signedIn, fetchImpl })
    expect(result).toEqual({ kind: 'signed-out' })
    expect(fetchImpl).not.toHaveBeenCalled()
  })

  it('tells a refusal apart from an outage', async () => {
    for (const status of [401, 403]) {
      const result = await requestSyncTicket(socketUrl, {
        headers: signedIn,
        fetchImpl: async () => Response.json({ error: 'no' }, { status }),
      })
      expect(result).toEqual({ kind: 'refused', status })
    }
    for (const fetchImpl of [
      async () => Response.json({ error: 'down' }, { status: 502 }),
      // A relay that predates /ws/ticket.
      async () => new Response('Not found', { status: 404 }),
      async () => Response.json({}),
      async () => { throw new TypeError('network') },
    ]) {
      const result = await requestSyncTicket(socketUrl, { headers: signedIn, fetchImpl })
      expect(result).toEqual({ kind: 'unavailable' })
    }
  })

  it('gives up on a ticket request that does not answer', async () => {
    const fetchImpl = (_url: string | URL | Request, init?: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init?.signal?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')))
      })
    const result = await requestSyncTicket(socketUrl, { headers: signedIn, fetchImpl, timeoutMs: 5 })
    expect(result).toEqual({ kind: 'unavailable' })
  })
})
