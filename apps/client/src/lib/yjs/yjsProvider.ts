import * as Y from 'yjs'
import { IndexeddbPersistence } from 'y-indexeddb'
import {
  appKitConfig,
  buildRecordsRoomId,
  resolveBrowserSyncWebsocketUrl,
} from '../../app/kitConfig.js'
import { authStorageKey, loadStoredAuthIdentity } from '../auth'
import { requestSyncTicket } from './syncTicket'
import { missingUpdates } from './initialSyncUpload'

// ---------------------------------------------------------------------------
// Y.Doc singleton
// ---------------------------------------------------------------------------

export const ydoc = new Y.Doc()
export const recordsArray = ydoc.getArray<Y.Map<string>>(appKitConfig.sync.yjsArrayName)
export const databasesArray = ydoc.getArray<Y.Map<string>>(appKitConfig.sync.databasesArrayName)
export const databaseViewsArray = ydoc.getArray<Y.Map<string>>(
  appKitConfig.sync.databaseViewsArrayName
)
export const workflowCanvasesMap = ydoc.getMap<string>(
  appKitConfig.sync.workflowCanvasesMapName
)
export const attachmentsArray = ydoc.getArray<Y.Map<string>>(appKitConfig.attachments.yjsArrayName)

// ---------------------------------------------------------------------------
// IndexedDB persistence
// ---------------------------------------------------------------------------

export const persistence = new IndexeddbPersistence(appKitConfig.sync.persistenceKey, ydoc)

/** Resolves when the local IndexedDB state has been loaded into the Y.Doc. */
export const idbSynced: Promise<void> = new Promise((resolve) => {
  persistence.once('synced', () => resolve())
})

// ---------------------------------------------------------------------------
// Connection status
// ---------------------------------------------------------------------------

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected'
export interface SyncPresence {
  onlineCount: number
}

type StatusListener = (status: ConnectionStatus) => void
type PresenceListener = (presence: SyncPresence) => void

let _status: ConnectionStatus = 'disconnected'
let _presence: SyncPresence = { onlineCount: 0 }
const _listeners = new Set<StatusListener>()
const _presenceListeners = new Set<PresenceListener>()

function setStatus(s: ConnectionStatus) {
  if (s === _status) return
  _status = s
  _listeners.forEach((fn) => fn(s))
}

function setPresence(presence: SyncPresence) {
  if (presence.onlineCount === _presence.onlineCount) return
  _presence = presence
  _presenceListeners.forEach((fn) => fn(presence))
}

export const connectionStatus = {
  get value() {
    return _status
  },
  subscribe(fn: StatusListener) {
    _listeners.add(fn)
    return () => {
      _listeners.delete(fn)
    }
  },
}

export const syncPresence = {
  get value() {
    return _presence
  },
  subscribe(fn: PresenceListener) {
    _presenceListeners.add(fn)
    return () => {
      _presenceListeners.delete(fn)
    }
  },
}

// ---------------------------------------------------------------------------
// Photon Live: WebSocket realtime UX sync
//
// This module keeps the shared UI feeling live through Yjs updates, presence,
// IndexedDB hydration, and reconnect behavior. It is deliberately separate
// from Photon Engine, which owns durable mutation logs and push/pull sync.
// ---------------------------------------------------------------------------

const WS_REMOTE = 'ws-remote'

let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let backoff = 1000
const MAX_BACKOFF = 30_000
/** How long a join to the Rust server waits for more stored state before uploading the rest. */
const INITIAL_SYNC_QUIET_MS = 1_000
let disposed = false
/** True while this page is frozen or hidden and has given up its socket. */
let suspended = false
/**
 * Counts connection attempts. A ticket request that resolves after a newer
 * attempt started, or after the page gave up its socket, is dropped.
 */
let attempt = 0
let ticketPending = false

function getWsUrl(): string | undefined {
  return resolveBrowserSyncWebsocketUrl(
    appKitConfig.sync.websocketUrl,
    appKitConfig.sync.websocketPath,
  )
}

/**
 * Whether the stored session is the user this page's records room is named
 * after. The room is fixed when the page loads; after a sign-out, or a sign-in
 * as someone else without a reload, it is no longer this session's to use.
 */
function sessionOwnsRoom(): boolean {
  const roomId = appKitConfig.sync.recordsRoomId
  return Boolean(roomId) && buildRecordsRoomId(
    appKitConfig.sync.workspaceScope,
    loadStoredAuthIdentity()?.userId,
  ) === roomId
}

/** Gives up the socket and any attempt in flight, without reconnecting. */
function releaseSocket() {
  attempt += 1
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  ydoc.off('update', onDocUpdate)
  const socket = ws
  ws = null
  socket?.close()
  setStatus('disconnected')
  setPresence({ onlineCount: 0 })
}

function scheduleReconnect() {
  if (
    disposed ||
    suspended ||
    reconnectTimer ||
    ticketPending ||
    !getWsUrl() ||
    ws?.readyState === WebSocket.OPEN ||
    ws?.readyState === WebSocket.CONNECTING
  ) return

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null
    connectWs()
  }, backoff)
  backoff = Math.min(backoff * 2, MAX_BACKOFF)
}

/** Send local Y.Doc updates to the server (skip remote-origin updates). */
function onDocUpdate(update: Uint8Array, origin: unknown) {
  if (origin === WS_REMOTE) return
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(update)
  }
}

export function connectWs() {
  if (disposed || suspended || ticketPending) return
  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
    return
  }

  // No room without a signed-in user: the relay admits each user only to
  // the records room named after them.
  const roomUrl = sessionOwnsRoom() ? getWsUrl() : undefined
  if (!roomUrl) {
    setStatus('disconnected')
    setPresence({ onlineCount: 0 })
    return
  }

  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }

  setStatus('connecting')
  // Only the Cloudflare relay asks for a ticket. A Rust server (local and
  // on-prem) serves `/ws` itself, without `/ws/ticket`.
  if (appKitConfig.sync.backend !== 'cloudflare-durable-object') {
    openSocket(roomUrl)
    return
  }
  const current = ++attempt
  ticketPending = true
  void requestSyncTicket(roomUrl).then((result) => {
    ticketPending = false
    if (disposed || suspended) return
    // The page was hidden and shown again while this was in flight; the
    // `pageshow` reconnect was skipped for it, so start over now.
    if (current !== attempt) {
      connectWs()
      return
    }
    if (result.kind === 'ticket') {
      openSocket(result.url)
      return
    }
    setStatus('disconnected')
    setPresence({ onlineCount: 0 })
    // Signed out: nothing to retry with until the page has a session again
    // (`library-auth-change`, `online` and `pageshow` try again). Refused:
    // the token may refresh, but not soon, so wait the longest interval.
    if (result.kind === 'signed-out') return
    if (result.kind === 'refused') backoff = MAX_BACKOFF
    scheduleReconnect()
  })
}

function openSocket(wsUrl: string) {
  let socket: WebSocket
  try {
    socket = new WebSocket(wsUrl)
  } catch {
    setStatus('disconnected')
    setPresence({ onlineCount: 0 })
    scheduleReconnect()
    return
  }
  socket.binaryType = 'arraybuffer'
  ws = socket

  // The room as this socket received it. The relay sends its stored state as
  // it is on join and then a presence frame; once that has arrived, what the
  // room lacks is known and only that is sent, instead of the whole document.
  let roomCopy: Y.Doc | null = new Y.Doc()
  let roomFrames = 0
  let uploadTimer: ReturnType<typeof setTimeout> | null = null
  // The Cloudflare relay accepts the socket before it reads storage and may
  // relay another client's update first, so a lull proves nothing there; its
  // presence frame, sent after the stored state, is the only signal. The Rust
  // server sends no presence and is taken to be done once frames stop.
  const finishesOnPresence = appKitConfig.sync.backend === 'cloudflare-durable-object'

  const uploadMissing = () => {
    if (uploadTimer) {
      clearTimeout(uploadTimer)
      uploadTimer = null
    }
    const room = roomCopy
    roomCopy = null
    if (!room || ws !== socket || socket.readyState !== WebSocket.OPEN) return
    const { updates, skippedClients, deletionsSkipped } = missingUpdates(ydoc, room)
    room.destroy()
    for (const update of updates) socket.send(update)
    if (skippedClients.length > 0) {
      console.warn(
        `[yjs] ${skippedClients.length} client(s) hold more than the sync relay accepts in one update; their history stays local`
      )
    }
    if (deletionsSkipped) {
      console.warn(
        '[yjs] deletions made here are larger than the sync relay accepts in one update; they stay local'
      )
    }
  }

  socket.addEventListener('open', () => {
    if (ws !== socket) {
      socket.close()
      return
    }
    backoff = 1000 // reset backoff on successful connect
    setStatus('connected')
    ydoc.off('update', onDocUpdate)
    ydoc.on('update', onDocUpdate)
  })

  socket.addEventListener('message', (event) => {
    if (ws !== socket) return

    if (typeof event.data === 'string') {
      try {
        const message = JSON.parse(event.data) as {
          type?: string
          onlineCount?: number
        }
        if (message.type === 'presence' && typeof message.onlineCount === 'number') {
          setPresence({ onlineCount: message.onlineCount })
          // The relay sends presence after the room's stored state.
          if (roomCopy && roomFrames > 0) uploadMissing()
        }
      } catch {
        // Ignore non-protocol text messages.
      }
      return
    }

    const data = new Uint8Array(event.data as ArrayBuffer)
    try {
      Y.applyUpdate(ydoc, data, WS_REMOTE)
    } catch (error) {
      // The room sends its stored log as it is, and cannot fully check each
      // row against the document without rebuilding it. One this document
      // rejects is skipped; the rest of the room still arrives.
      console.warn('[yjs] skipped an update the document rejected', error)
    }

    if (roomCopy) {
      try {
        Y.applyUpdate(roomCopy, data)
      } catch {
        // Rejected above as well; the room copy skips it the same way.
      }
      roomFrames += 1
      if (!finishesOnPresence) {
        if (uploadTimer) clearTimeout(uploadTimer)
        uploadTimer = setTimeout(uploadMissing, INITIAL_SYNC_QUIET_MS)
      }
    }
  })

  socket.addEventListener('close', () => {
    if (uploadTimer) {
      clearTimeout(uploadTimer)
      uploadTimer = null
    }
    roomCopy?.destroy()
    roomCopy = null
    if (ws !== socket) return

    ydoc.off('update', onDocUpdate)
    ws = null
    setStatus('disconnected')
    setPresence({ onlineCount: 0 })
    scheduleReconnect()
  })

  socket.addEventListener('error', () => {
    // The 'close' event will fire after 'error', so reconnect is handled there.
  })
}

export function disconnectWs() {
  disposed = true
  attempt += 1
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  ydoc.off('update', onDocUpdate)
  if (ws) {
    ws.close()
    ws = null
  }
  setStatus('disconnected')
  setPresence({ onlineCount: 0 })
  window.removeEventListener('online', connectWs)
  window.removeEventListener('library-auth-change', handleAuthChange)
  window.removeEventListener('storage', handleStorage)
}

// ---------------------------------------------------------------------------
// Initial sync ready
// ---------------------------------------------------------------------------

/**
 * Resolves once:
 *  1. IndexedDB has been loaded, AND
 *  2. Either the first WebSocket message has been received, or 2 s elapsed
 *     (to support the offline-first case where the server is unreachable).
 */
export const initialSyncReady: Promise<void> = new Promise((resolve) => {
  let idbDone = false
  let wsDone = false
  let resolved = false

  function tryResolve() {
    if (resolved) return
    if (idbDone && wsDone) {
      resolved = true
      resolve()
    }
  }

  idbSynced.then(() => {
    idbDone = true
    tryResolve()

    // If WS hasn't delivered anything in 2 s, proceed anyway (offline)
    setTimeout(() => {
      wsDone = true
      tryResolve()
    }, 2000)
  })

  // Mark WS done as soon as the first message is applied
  const handler = (_update: Uint8Array, origin: unknown) => {
    if (origin === WS_REMOTE) {
      wsDone = true
      ydoc.off('update', handler)
      tryResolve()
    }
  }
  ydoc.on('update', handler)
})

// ---------------------------------------------------------------------------
// Auto-connect
// ---------------------------------------------------------------------------

/**
 * Hand the socket back when this page stops running.
 *
 * A page that is frozen into the back/forward cache -- on a phone, simply
 * switching away from the tab -- keeps its socket handle on both ends while
 * nothing can travel over it. The room counts this page as present until the
 * runtime eventually notices, which is how a workspace ends up reporting more
 * people online than are actually there. Closing deliberately makes the room
 * see the departure immediately, and `pageshow` reconnects a page that
 * returns.
 */
function handlePageHide(event: PageTransitionEvent) {
  suspended = true
  releaseSocket()
  // A page that is not being cached is not coming back to this module.
  if (!event.persisted) disposed = true
}

function handlePageShow(event: PageTransitionEvent) {
  if (!event.persisted || disposed) return
  suspended = false
  backoff = 1000
  connectWs()
}

/**
 * A session that ends -- signed out, expired, or replaced by another account
 * without a reload -- closes the room at once, so it stops sending and
 * receiving the user's records. A session that comes back (a refresh that
 * succeeds after a transient failure, or signing in again) connects now
 * instead of waiting for an unrelated `online` or `pageshow`.
 */
function handleAuthChange() {
  if (!sessionOwnsRoom()) {
    releaseSocket()
    return
  }
  backoff = 1000
  connectWs()
}

/**
 * Another tab signing out or in changes the shared session too, but only
 * that tab gets `library-auth-change`; this one sees the storage write.
 * `key` is null when storage was cleared outright.
 */
function handleStorage(event: StorageEvent) {
  if (event.key === authStorageKey || event.key === null) handleAuthChange()
}

window.addEventListener('online', connectWs)
window.addEventListener('library-auth-change', handleAuthChange)
window.addEventListener('storage', handleStorage)
window.addEventListener('pagehide', handlePageHide)
window.addEventListener('pageshow', handlePageShow)
connectWs()
