import { appendSyncTicket, appKitConfig, buildSyncTicketUrl } from '../../app/kitConfig.js'
import { libraryGraphqlHeaders } from '../libraryGraphql'

/**
 * How a records-room connection attempt can go before the socket opens.
 *
 * - `ticket`: `url` opens the room once (the ticket is single-use).
 * - `signed-out`: there is no Library token to ask with; reconnecting cannot
 *   help until someone signs in.
 * - `refused`: the relay does not admit this user to this room (401/403).
 * - `unavailable`: the relay or the API could not answer; worth retrying.
 */
export type SyncTicketResult =
  | { kind: 'ticket'; url: string }
  | { kind: 'signed-out' }
  | { kind: 'refused'; status: number }
  | { kind: 'unavailable' }

export interface SyncTicketDeps {
  headers?: () => Promise<Record<string, string>>
  fetchImpl?: typeof fetch
  timeoutMs?: number
}

/**
 * Trade the caller's Library token for a one-time ticket to their own
 * records room. A browser WebSocket cannot send an `Authorization` header,
 * so the relay checks the token here and the socket carries only the ticket.
 */
export async function requestSyncTicket(
  websocketUrl: string,
  {
    headers = () => libraryGraphqlHeaders(),
    fetchImpl = fetch,
    timeoutMs = 10_000,
  }: SyncTicketDeps = {},
): Promise<SyncTicketResult> {
  let room: string | null
  let ticketUrl: string
  try {
    room = new URL(websocketUrl).searchParams.get(appKitConfig.sync.roomParam)
    ticketUrl = buildSyncTicketUrl(websocketUrl, appKitConfig.sync.ticketPath)
  } catch {
    return { kind: 'unavailable' }
  }
  if (!room) return { kind: 'signed-out' }

  // One deadline covers getting the token as well: a refresh that stalls
  // would otherwise hold this attempt, and every reconnect behind it, open.
  const controller = new AbortController()
  const timeout = globalThis.setTimeout(() => controller.abort(), timeoutMs)
  const aborted = new Promise<never>((_resolve, reject) => {
    controller.signal.addEventListener('abort', () => reject(new Error('timed out')))
  })
  aborted.catch(() => {})
  let requestHeaders: Record<string, string>
  try {
    requestHeaders = await Promise.race([headers(), aborted])
  } catch {
    globalThis.clearTimeout(timeout)
    return { kind: 'unavailable' }
  }
  if (!requestHeaders.Authorization) {
    globalThis.clearTimeout(timeout)
    return { kind: 'signed-out' }
  }

  try {
    const response = await fetchImpl(ticketUrl, {
      method: 'POST',
      headers: requestHeaders,
      body: JSON.stringify({ room }),
      signal: controller.signal,
    })
    if (response.status === 401 || response.status === 403) {
      return { kind: 'refused', status: response.status }
    }
    if (!response.ok) return { kind: 'unavailable' }
    const payload = (await response.json().catch(() => null)) as { ticket?: unknown } | null
    if (typeof payload?.ticket !== 'string' || !payload.ticket) return { kind: 'unavailable' }
    return { kind: 'ticket', url: appendSyncTicket(websocketUrl, payload.ticket) }
  } catch {
    return { kind: 'unavailable' }
  } finally {
    globalThis.clearTimeout(timeout)
  }
}
