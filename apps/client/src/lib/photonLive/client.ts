import { loadStoredAuthIdentity, getValidAuthTokens } from '../auth'
import { appKitConfig, type DataLiveConfig } from '../../app/kitConfig'
import {
  PhotonLiveError,
  type PhotonLiveConnectionStatus,
  type PhotonLiveFormat,
  type PhotonLiveProvider,
  type PhotonLiveRecordTarget,
  type PhotonLiveSaveStatus,
  type PhotonLiveSession,
  type PhotonLiveState,
} from './types'
import { Awareness, applyAwarenessUpdate, encodeAwarenessUpdate } from 'y-protocols/awareness'
import * as Y from 'yjs'

const DOC_REMOTE_ORIGIN = 'photon-live-remote'
const AWARENESS_REMOTE_ORIGIN = 'photon-live-awareness-remote'
const MAX_BACKOFF_MS = 30_000
const INITIAL_BACKOFF_MS = 1_000

interface LiveReadyMessage {
  type: 'live-ready'
  initialized?: boolean
  version?: number
  record_version?: string | number
}

interface LiveSavedMessage {
  type: 'live-saved'
  version?: number
  record_version?: string | number
  operation_id?: string
}

interface LiveVersionMessage {
  type: 'live-version'
  version?: number
}

interface LiveConflictMessage {
  type: 'live-conflict'
  version?: number
  record_version?: string | number
  operation_id?: string
  message?: string
}

interface LiveErrorMessage {
  type: 'live-error'
  message?: string
  operation_id?: string
}

interface AwarenessMessage {
  type: 'awareness'
  update?: string
}

type LiveTextMessage =
  | LiveReadyMessage
  | LiveSavedMessage
  | LiveVersionMessage
  | LiveConflictMessage
  | LiveErrorMessage
  | AwarenessMessage

export interface PhotonLiveProviderOptions {
  target: PhotonLiveRecordTarget
  format: PhotonLiveFormat
  config?: DataLiveConfig
  /** Convert the server's canonical body into a Yjs update in a temporary doc. */
  seedUpdate: (body: string, format: PhotonLiveFormat) => Uint8Array
  fetchImpl?: typeof fetch
  webSocketFactory?: (url: string) => WebSocket
  user?: { name: string; color: string }
}

export interface PhotonLiveSessionResponse {
  ticket?: unknown
  room_id?: unknown
  actor_id?: unknown
  format?: unknown
  body?: unknown
  record_version?: unknown
}

function configuredPlatformId(): string {
  return import.meta.env.VITE_LIBRARY_PLATFORM_ID ??
    import.meta.env.VITE_PLATFORM_ID ??
    'tn_01j702qf86pc2j35s0kv0gv3gy'
}

function configuredOperatorId(operatorId: string | undefined): string {
  return operatorId?.trim() ||
    import.meta.env.VITE_LIBRARY_OPERATOR_ID?.trim() ||
    configuredPlatformId()
}

async function accessToken(): Promise<string | undefined> {
  const configured = import.meta.env.VITE_LIBRARY_ACCESS_TOKEN?.trim()
  if (configured) return configured
  return (await getValidAuthTokens())?.accessToken
}

function liveUrl(config: DataLiveConfig, path: string): URL {
  if (!config.baseUrl) {
    throw new PhotonLiveError(
      'Photon Live is not configured',
      'disabled',
      503,
    )
  }
  return new URL(path, `${config.baseUrl.replace(/\/+$/, '')}/`)
}

export function buildPhotonLiveSessionUrl(config: DataLiveConfig): string {
  return liveUrl(config, config.sessionPath).toString()
}

export function buildPhotonLiveWebsocketUrl(
  config: DataLiveConfig,
  ticket: string,
): string {
  const url = liveUrl(config, config.websocketPath)
  url.searchParams.set('ticket', ticket)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  return url.toString()
}

function errorMessage(payload: unknown): string | undefined {
  if (!payload || typeof payload !== 'object') return undefined
  const message = (payload as { message?: unknown }).message
  return typeof message === 'string' && message.trim() ? message : undefined
}

function isExplicitlyDisabled(payload: unknown): boolean {
  if (!payload || typeof payload !== 'object') return false
  const response = payload as {
    disabled?: unknown
    code?: unknown
    error_code?: unknown
    errorCode?: unknown
  }
  if (response.disabled === true) return true
  const code = [response.code, response.error_code, response.errorCode]
    .find((value): value is string => typeof value === 'string')
    ?.trim()
    .toUpperCase()
  return code === 'DISABLED' ||
    code === 'LIVE_DISABLED' ||
    code === 'PHOTON_LIVE_DISABLED' ||
    code === 'DATA_LIVE_DISABLED'
}

function errorForStatus(
  status: number,
  message?: string,
  payload?: unknown,
): PhotonLiveError {
  if (status === 401 || status === 403 || status === 409) {
    return new PhotonLiveError(
      message ?? 'Photon Live session is not authorized for this record',
      'unauthorized',
      status,
    )
  }
  // A configured endpoint returning 503 can be a transient ticket-store or
  // API outage. Only an explicit feature-disabled response may enable the
  // ordinary editor fallback, otherwise the body remains read-only.
  if (status === 503 && isExplicitlyDisabled(payload)) {
    return new PhotonLiveError(
      message ?? 'Photon Live is disabled',
      'disabled',
      status,
    )
  }
  return new PhotonLiveError(
    message ?? `Photon Live session failed (${status})`,
    'server',
    status,
    status >= 500,
  )
}

function toLiveFormat(value: unknown): PhotonLiveFormat | undefined {
  if (value === 'markdown' || value === 'richText') return value
  return undefined
}

function sessionFromResponse(payload: unknown): PhotonLiveSession {
  const response = payload as PhotonLiveSessionResponse
  const ticket = typeof response.ticket === 'string' ? response.ticket : undefined
  const roomId = typeof response.room_id === 'string' ? response.room_id : undefined
  const actorId = typeof response.actor_id === 'string' ? response.actor_id : undefined
  const format = toLiveFormat(response.format)
  const body = typeof response.body === 'string' ? response.body : undefined
  const recordVersion =
    typeof response.record_version === 'string' || typeof response.record_version === 'number'
      ? String(response.record_version)
      : undefined

  if (!ticket || !roomId || !actorId || !format || body === undefined || !recordVersion) {
    throw new PhotonLiveError(
      'Photon Live session returned an invalid scope',
      'protocol',
    )
  }

  return {
    ticket,
    roomId,
    actorId,
    format,
    body,
    recordVersion,
  }
}

export async function requestPhotonLiveSession(
  target: PhotonLiveRecordTarget,
  config: DataLiveConfig = appKitConfig.dataLive,
  fetchImpl: typeof fetch = fetch,
  expectedFormat?: PhotonLiveFormat,
): Promise<PhotonLiveSession> {
  const controller = new AbortController()
  const timeout = globalThis.setTimeout(
    () => controller.abort(),
    config.requestTimeoutMs,
  )

  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': configuredOperatorId(target.operatorId),
  }
  const token = await accessToken()
  if (token) headers.Authorization = `Bearer ${token}`

  try {
    const response = await fetchImpl(buildPhotonLiveSessionUrl(config), {
      method: 'POST',
      headers,
      body: JSON.stringify({
        org: target.org,
        repo: target.repo,
        data_id: target.dataId,
        property_id: target.propertyId,
      }),
      signal: controller.signal,
    })
    const payload: unknown = await response.json().catch(() => ({}))
    if (!response.ok) {
      throw errorForStatus(response.status, errorMessage(payload), payload)
    }
    const session = sessionFromResponse(payload)
    if (expectedFormat && session.format !== expectedFormat) {
      throw new PhotonLiveError(
        'Photon Live session format does not match the body property',
        'protocol',
      )
    }
    return session
  } catch (error: unknown) {
    if (error instanceof PhotonLiveError) throw error
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new PhotonLiveError(
        'Photon Live session request timed out',
        'timeout',
        undefined,
        true,
      )
    }
    if (error instanceof Error && error.name === 'AbortError') {
      throw new PhotonLiveError(
        'Photon Live session request timed out',
        'timeout',
        undefined,
        true,
      )
    }
    throw new PhotonLiveError(
      error instanceof Error ? error.message : 'Photon Live session request failed',
      'transport',
      undefined,
      true,
    )
  } finally {
    globalThis.clearTimeout(timeout)
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize))
  }
  return globalThis.btoa(binary)
}

function base64ToBytes(value: string): Uint8Array {
  const binary = globalThis.atob(value)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }
  return bytes
}

function bytesFromSocketData(data: unknown): Promise<Uint8Array> {
  if (data instanceof ArrayBuffer) return Promise.resolve(new Uint8Array(data))
  if (ArrayBuffer.isView(data)) {
    return Promise.resolve(new Uint8Array(data.buffer, data.byteOffset, data.byteLength))
  }
  if (typeof Blob !== 'undefined' && data instanceof Blob) {
    return data.arrayBuffer().then((buffer) => new Uint8Array(buffer))
  }
  return Promise.reject(new PhotonLiveError('Photon Live sent an invalid binary frame', 'protocol'))
}

function randomOperationId(): string {
  return globalThis.crypto?.randomUUID?.() ??
    `live-${Date.now()}-${Math.random().toString(36).slice(2)}`
}

function defaultUser(): { name: string; color: string } {
  const identity = loadStoredAuthIdentity()
  return {
    name: identity?.username || identity?.email || 'Library user',
    color: '#5b5bf7',
  }
}

interface LiveAttempt {
  receivedSnapshot: boolean
  receivedReady: boolean
  initialized: boolean
  initializationSent: boolean
  initializationEchoed: boolean
}

interface PendingCheckpoint {
  body: string
  operationId: string
  /** Y.Doc generation represented by this serialized body. */
  generation: number
  /** Room version captured for this operation, once it is sent. */
  version?: number
}

class PhotonLiveProviderImpl implements PhotonLiveProvider {
  readonly doc: Y.Doc
  readonly fragment: Y.XmlFragment
  readonly awareness: Awareness
  readonly user: { name: string; color: string }
  get session(): PhotonLiveSession | null {
    return this._session
  }

  private readonly target: PhotonLiveRecordTarget
  private readonly format: PhotonLiveFormat
  private readonly config: DataLiveConfig
  private readonly seedUpdate: PhotonLiveProviderOptions['seedUpdate']
  private readonly fetchImpl: typeof fetch
  private readonly webSocketFactory: (url: string) => WebSocket
  private readonly listeners = new Set<(state: PhotonLiveState) => void>()
  private readonly saveListeners = new Set<(state: PhotonLiveState) => void>()
  private socket: WebSocket | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private handshakeTimer: ReturnType<typeof setTimeout> | null = null
  private checkpointTimer: ReturnType<typeof setTimeout> | null = null
  private attempt: LiveAttempt | null = null
  private _session: PhotonLiveSession | null = null
  private connectionStatus: PhotonLiveConnectionStatus = 'authorizing'
  private saveStatus: PhotonLiveSaveStatus = 'idle'
  private currentError: PhotonLiveError | null = null
  private initialized = false
  private version = 0
  private recordVersion = ''
  private hasUnackedChanges = false
  private inFlight: PendingCheckpoint | null = null
  private pendingCheckpoint: PendingCheckpoint | null = null
  private docGeneration = 0
  private backoff = INITIAL_BACKOFF_MS
  private disposed = false
  private reconnecting = false
  private offline = typeof navigator !== 'undefined' && navigator.onLine === false

  constructor(options: PhotonLiveProviderOptions) {
    this.target = options.target
    this.format = options.format
    this.config = options.config ?? appKitConfig.dataLive
    this.seedUpdate = options.seedUpdate
    this.fetchImpl = options.fetchImpl ?? fetch
    this.webSocketFactory = options.webSocketFactory ?? ((url) => new WebSocket(url))
    this.user = options.user ?? defaultUser()
    this.doc = new Y.Doc()
    this.fragment = this.doc.getXmlFragment(this.config.fragmentName)
    this.awareness = new Awareness(this.doc)
    this.doc.on('update', this.handleDocUpdate)
    this.awareness.on('update', this.handleAwarenessUpdate)
    if (typeof window !== 'undefined') {
      window.addEventListener('offline', this.handleOffline)
      window.addEventListener('online', this.handleOnline)
      window.addEventListener('library-auth-change', this.handleAuthChange)
    }
  }

  start(): void {
    void this.authorizeAndConnect(true)
  }

  getState(): PhotonLiveState {
    return {
      status: this.connectionStatus,
      saveStatus: this.saveStatus,
      error: this.currentError,
      initialized: this.initialized,
      version: this.version,
      recordVersion: this.recordVersion,
      hasUnackedChanges: this.hasUnackedChanges,
      canEdit: this.initialized &&
        this.connectionStatus === 'connected' &&
        this.saveStatus !== 'conflict',
    }
  }

  subscribe(listener: (state: PhotonLiveState) => void): () => void {
    this.listeners.add(listener)
    listener(this.getState())
    return () => this.listeners.delete(listener)
  }

  subscribeSave(listener: (state: PhotonLiveState) => void): () => void {
    this.saveListeners.add(listener)
    listener(this.getState())
    return () => this.saveListeners.delete(listener)
  }

  queueCheckpoint(body: string): void {
    if (this.disposed || !this.initialized) return
    // Coalesce only a body that is still waiting to be sent. A body queued
    // while another checkpoint is in flight is a new idempotency operation;
    // reusing that key would make the worker reject a valid later body.
    const operationId = this.pendingCheckpoint?.body === body
      ? this.pendingCheckpoint.operationId
      : randomOperationId()
    this.pendingCheckpoint = {
      body,
      operationId,
      generation: this.docGeneration,
    }
    this.hasUnackedChanges = true
    this.saveStatus = 'saving'
    this.emit()
    this.scheduleCheckpoint()
  }

  flushCheckpoint(): void {
    if (this.checkpointTimer !== null) {
      globalThis.clearTimeout(this.checkpointTimer)
      this.checkpointTimer = null
    }
    this.sendPendingCheckpoint()
  }

  destroy(): void {
    if (this.disposed) return
    this.disposed = true
    if (this.reconnectTimer !== null) globalThis.clearTimeout(this.reconnectTimer)
    if (this.handshakeTimer !== null) globalThis.clearTimeout(this.handshakeTimer)
    if (this.checkpointTimer !== null) globalThis.clearTimeout(this.checkpointTimer)
    this.reconnectTimer = null
    this.handshakeTimer = null
    this.checkpointTimer = null
    this.doc.off('update', this.handleDocUpdate)
    this.awareness.off('update', this.handleAwarenessUpdate)
    if (typeof window !== 'undefined') {
      window.removeEventListener('offline', this.handleOffline)
      window.removeEventListener('online', this.handleOnline)
      window.removeEventListener('library-auth-change', this.handleAuthChange)
    }
    this.awareness.setLocalState(null)
    this.socket?.close()
    this.socket = null
    this.listeners.clear()
    this.saveListeners.clear()
    this.doc.destroy()
  }

  private emit(): void {
    const state = this.getState()
    this.listeners.forEach((listener) => listener(state))
    this.saveListeners.forEach((listener) => listener(state))
  }

  private setConnectionStatus(status: PhotonLiveConnectionStatus, error?: PhotonLiveError | null): void {
    this.connectionStatus = status
    if (error !== undefined) this.currentError = error
    this.emit()
  }

  private setSaveStatus(status: PhotonLiveSaveStatus, error?: PhotonLiveError | null): void {
    this.saveStatus = status
    if (error !== undefined) this.currentError = error
    this.emit()
  }

  private updateVersion(value: unknown): boolean {
    if (typeof value !== 'number' || !Number.isFinite(value)) return false
    const next = Math.max(this.version, value)
    if (next === this.version) return false
    this.version = next
    return true
  }

  private updateRecordVersion(value: unknown): boolean {
    if (typeof value !== 'string' && typeof value !== 'number') return false
    const next = String(value)
    if (!next || next === this.recordVersion) return false
    if (!this.recordVersion) {
      this.recordVersion = next
      return true
    }

    const decimal = /^\d+$/
    if (decimal.test(this.recordVersion) && decimal.test(next)) {
      if (BigInt(next) <= BigInt(this.recordVersion)) return false
    } else if (decimal.test(this.recordVersion) && !decimal.test(next)) {
      // RecordVersion is decimal on the Live wire. Do not let an invalid or
      // opaque stale response replace a known-good working version.
      return false
    }
    this.recordVersion = next
    return true
  }

  private async authorizeAndConnect(initial: boolean): Promise<void> {
    if (this.disposed || this.reconnecting) return
    this.reconnecting = true
    this.setConnectionStatus('authorizing', initial ? null : undefined)
    try {
      const session = await requestPhotonLiveSession(
        this.target,
        this.config,
        this.fetchImpl,
        this.format,
      )
      if (session.format !== this.format) {
        throw new PhotonLiveError(
          'Photon Live session format does not match the body property',
          'protocol',
        )
      }
      if (this.disposed) return
      this._session = session
      this.updateRecordVersion(session.recordVersion)
      this.currentError = null
      this.connectSocket(session.ticket)
    } catch (error: unknown) {
      if (this.disposed) return
      const liveError = error instanceof PhotonLiveError
        ? error
        : new PhotonLiveError(
          error instanceof Error ? error.message : 'Photon Live could not start',
          'transport',
          undefined,
          true,
        )
      this.setConnectionStatus('failed', liveError)
      if (!initial && liveError.retryable) this.scheduleReconnect()
    } finally {
      this.reconnecting = false
    }
  }

  private connectSocket(ticket: string): void {
    if (this.disposed) return
    if (this.socket && (
      this.socket.readyState === WebSocket.OPEN ||
      this.socket.readyState === WebSocket.CONNECTING
    )) return

    let socket: WebSocket
    try {
      socket = this.webSocketFactory(buildPhotonLiveWebsocketUrl(this.config, ticket))
    } catch (error: unknown) {
      this.handleSocketFailure(new PhotonLiveError(
        error instanceof Error ? error.message : 'Photon Live WebSocket failed',
        'transport',
        undefined,
        true,
      ))
      return
    }

    socket.binaryType = 'arraybuffer'
    this.socket = socket
    this.attempt = {
      receivedSnapshot: false,
      receivedReady: false,
      initialized: false,
      initializationSent: false,
      initializationEchoed: false,
    }
    this.setConnectionStatus('connecting', null)
    this.handshakeTimer = globalThis.setTimeout(() => {
      if (this.socket !== socket || this.initialized && this.connectionStatus === 'connected') return
      socket.close()
      this.handleSocketFailure(new PhotonLiveError(
        'Photon Live did not send a ready snapshot in time',
        'timeout',
        undefined,
        true,
      ))
    }, this.config.requestTimeoutMs)

    socket.addEventListener('open', () => {
      if (this.socket !== socket || this.disposed) {
        socket.close()
        return
      }
      this.backoff = INITIAL_BACKOFF_MS
      this.setConnectionStatus('connecting')
    })
    socket.addEventListener('message', (event) => {
      if (this.socket !== socket || this.disposed) return
      void this.handleSocketMessage(socket, event.data)
    })
    socket.addEventListener('close', () => {
      if (this.socket !== socket || this.disposed) return
      this.socket = null
      this.clearHandshakeTimer()
      this.handleSocketFailure(new PhotonLiveError(
        'Photon Live connection closed',
        'transport',
        undefined,
        true,
      ))
    })
    socket.addEventListener('error', () => {
      if (this.socket !== socket || this.disposed) return
      this.setConnectionStatus('disconnected', new PhotonLiveError(
        'Photon Live connection failed',
        'transport',
        undefined,
        true,
      ))
    })
  }

  private async handleSocketMessage(socket: WebSocket, data: unknown): Promise<void> {
    if (typeof data === 'string') {
      let message: unknown
      try {
        message = JSON.parse(data)
      } catch {
        this.handleProtocolFailure('Photon Live sent invalid JSON')
        return
      }
      this.handleTextMessage(socket, message as LiveTextMessage)
      return
    }

    try {
      const update = await bytesFromSocketData(data)
      if (this.socket !== socket || this.disposed) return
      this.handleBinaryMessage(update)
    } catch (error: unknown) {
      this.handleProtocolFailure(error instanceof Error ? error.message : 'Photon Live binary frame failed')
    }
  }

  private handleBinaryMessage(update: Uint8Array): void {
    const attempt = this.attempt
    if (!attempt) return
    try {
      Y.applyUpdate(this.doc, update, DOC_REMOTE_ORIGIN)
    } catch {
      this.handleProtocolFailure('Photon Live sent an invalid Yjs snapshot')
      return
    }
    if (!attempt.receivedSnapshot) {
      attempt.receivedSnapshot = true
    } else if (attempt.initializationSent) {
      attempt.initializationEchoed = true
    }
    this.finishReadyIfPossible()
  }

  private handleTextMessage(socket: WebSocket, raw: LiveTextMessage): void {
    if (!raw || typeof raw !== 'object' || typeof raw.type !== 'string') {
      this.handleProtocolFailure('Photon Live sent an invalid message')
      return
    }
    switch (raw.type) {
      case 'live-ready':
        this.handleReadyMessage(socket, raw)
        return
      case 'live-version':
        if (this.updateVersion(raw.version)) this.emit()
        return
      case 'live-saved':
        this.handleSavedMessage(raw)
        return
      case 'live-conflict':
        this.handleConflictMessage(raw)
        return
      case 'live-error':
        this.handleLiveErrorMessage(raw)
        return
      case 'awareness':
        if (typeof raw.update !== 'string') {
          this.handleProtocolFailure('Photon Live sent an invalid awareness update')
          return
        }
        try {
          applyAwarenessUpdate(
            this.awareness,
            base64ToBytes(raw.update),
            AWARENESS_REMOTE_ORIGIN,
          )
        } catch {
          this.handleProtocolFailure('Photon Live sent an invalid awareness update')
        }
        return
      default:
        // Future server messages must not take the editor down. Binary and
        // the messages above are the only frames this client acts on.
        return
    }
  }

  private handleReadyMessage(socket: WebSocket, message: LiveReadyMessage): void {
    if (this.socket !== socket) return
    const attempt = this.attempt
    const session = this._session
    if (!attempt || !session) return
    if (!attempt.receivedSnapshot) {
      this.handleProtocolFailure('Photon Live sent live-ready before its snapshot')
      return
    }
    this.updateVersion(message.version)
    if (message.record_version !== undefined) this.updateRecordVersion(message.record_version)

    if (message.initialized === false) {
      attempt.receivedReady = false
      attempt.initialized = false
      if (!attempt.initializationSent) {
        let update: Uint8Array
        try {
          update = this.seedUpdate(session.body, session.format)
        } catch (error: unknown) {
          this.handleProtocolFailure(
            error instanceof Error ? error.message : 'Photon Live could not seed the body',
          )
          return
        }
        this.sendJson(socket, {
          type: 'live-initialize',
          update: bytesToBase64(update),
        })
        attempt.initializationSent = true
        this.setConnectionStatus('initializing')
      }
      return
    }

    attempt.receivedReady = true
    attempt.initialized = true
    this.finishReadyIfPossible()
  }

  private finishReadyIfPossible(): void {
    const attempt = this.attempt
    const socket = this.socket
    if (!attempt || !socket || socket.readyState !== WebSocket.OPEN) return
    if (attempt.initialized && this.initialized && this.connectionStatus === 'connected') return
    if (!attempt.receivedSnapshot || !attempt.receivedReady) return
    if (attempt.initializationSent && !attempt.initializationEchoed) return
    this.clearHandshakeTimer()
    this.initialized = true
    this.setConnectionStatus('connected', null)
    this.sendCurrentState()
    this.sendAwarenessUpdate([this.awareness.clientID])
    if (this.inFlight) this.sendCheckpoint(this.inFlight)
    else if (this.pendingCheckpoint) this.scheduleCheckpoint()
  }

  private sendCurrentState(): void {
    const socket = this.socket
    if (
      !socket ||
      socket.readyState !== WebSocket.OPEN ||
      !this.initialized ||
      !this.hasUnackedChanges
    ) return
    const update = Y.encodeStateAsUpdate(this.doc)
    if (update.byteLength > 0) socket.send(update)
  }

  private handleSavedMessage(message: LiveSavedMessage): void {
    const versionChanged = this.updateVersion(message.version)
    if (message.record_version !== undefined) this.updateRecordVersion(message.record_version)
    const acknowledged = this.inFlight
    // The server broadcasts live-saved to every room participant. A save
    // without a local in-flight checkpoint is another actor's notification;
    // it must not clear this client's unsaved state.
    if (!acknowledged || message.operation_id !== acknowledged.operationId) {
      if (versionChanged) this.emit()
      return
    }
    this.inFlight = null
    if (this.pendingCheckpoint) {
      this.setSaveStatus('saving')
      this.scheduleCheckpoint()
      return
    }
    if (acknowledged.generation !== this.docGeneration) {
      // A local or remote Y.Doc update arrived after this checkpoint. Keep
      // the page visibly unsaved until BlockNote serializes the merged body.
      this.hasUnackedChanges = true
      this.setSaveStatus('saving')
      return
    }
    this.hasUnackedChanges = false
    this.setSaveStatus('saved')
  }

  private handleConflictMessage(message: LiveConflictMessage): void {
    this.updateVersion(message.version)
    if (message.record_version !== undefined) this.updateRecordVersion(message.record_version)
    if (!this.inFlight || message.operation_id !== this.inFlight.operationId) return
    if (this.inFlight && !this.pendingCheckpoint) this.pendingCheckpoint = this.inFlight
    this.inFlight = null
    const error = new PhotonLiveError(
      message.message ?? 'Photon Live checkpoint conflicted with another change',
      'conflict',
      409,
    )
    this.setSaveStatus('conflict', error)
  }

  private handleLiveErrorMessage(message: LiveErrorMessage): void {
    const error = new PhotonLiveError(
      message.message ?? 'Photon Live rejected the checkpoint',
      'server',
    )
    if (!this.inFlight || (
      message.operation_id !== undefined &&
      message.operation_id !== this.inFlight.operationId
    )) return
    if (this.inFlight && !this.pendingCheckpoint) this.pendingCheckpoint = this.inFlight
    this.inFlight = null
    this.setSaveStatus('error', error)
  }

  private handleDocUpdate = (update: Uint8Array, origin: unknown): void => {
    if (this.disposed) return
    this.docGeneration += 1
    if (origin === DOC_REMOTE_ORIGIN) {
      // A peer's update changes the body that the next durable checkpoint
      // must represent. Invalidate a body captured before this merge so a
      // debounce cannot send stale text with the newer room version.
      // The first binary frame on every socket is a server snapshot. It is
      // already authoritative and must not turn a clean reconnect into a
      // phantom local edit. Later room updates are merged peer changes.
      if (this.initialized && this.attempt?.receivedSnapshot) {
        this.hasUnackedChanges = true
        if (this.pendingCheckpoint && this.pendingCheckpoint.generation !== this.docGeneration) {
          this.pendingCheckpoint = null
        }
        this.emit()
      }
      return
    }
    this.hasUnackedChanges = true
    if (this.pendingCheckpoint && this.pendingCheckpoint.generation !== this.docGeneration) {
      this.pendingCheckpoint = null
    }
    const socket = this.socket
    if (this.initialized && socket?.readyState === WebSocket.OPEN) {
      socket.send(update)
    }
    this.emit()
  }

  private handleAwarenessUpdate = (
    changes: { added: number[]; updated: number[]; removed: number[] },
    origin: unknown,
  ): void => {
    if (this.disposed || origin === AWARENESS_REMOTE_ORIGIN) return
    this.sendAwarenessUpdate([
      ...changes.added,
      ...changes.updated,
      ...changes.removed,
    ])
  }

  private handleOffline = (): void => {
    if (this.disposed || this.offline) return
    this.offline = true
    const socket = this.socket
    this.socket = null
    socket?.close()
    this.handleSocketFailure(new PhotonLiveError(
      'Photon Live connection is offline',
      'transport',
      undefined,
      true,
    ))
  }

  private handleOnline = (): void => {
    if (this.disposed || !this.offline) return
    this.offline = false
    if (this.reconnectTimer !== null) {
      globalThis.clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (!this.socket && !this.reconnecting) void this.authorizeAndConnect(false)
  }

  private handleAuthChange = (event: Event): void => {
    const reason = (event as CustomEvent<{ reason?: unknown }>).detail?.reason
    if (reason === 'signed-out' || reason === 'expired') this.destroy()
  }

  private sendAwarenessUpdate(clientIds: number[]): void {
    const socket = this.socket
    if (!clientIds.length || !this.initialized || socket?.readyState !== WebSocket.OPEN) return
    this.sendJson(socket, {
      type: 'awareness',
      update: bytesToBase64(encodeAwarenessUpdate(this.awareness, clientIds)),
    })
  }

  private sendJson(socket: WebSocket, message: Record<string, unknown>): void {
    if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message))
  }

  private scheduleCheckpoint(): void {
    if (this.checkpointTimer !== null) globalThis.clearTimeout(this.checkpointTimer)
    this.checkpointTimer = globalThis.setTimeout(() => {
      this.checkpointTimer = null
      this.sendPendingCheckpoint()
    }, this.config.checkpointDebounceMs)
  }

  private sendPendingCheckpoint(): void {
    const socket = this.socket
    if (
      !this.initialized ||
      !socket ||
      socket.readyState !== WebSocket.OPEN ||
      this.inFlight ||
      !this.pendingCheckpoint
    ) return

    const pending = this.pendingCheckpoint
    if (pending.generation !== this.docGeneration) {
      // BlockNote will queue the merged document after its Yjs transaction.
      // Never send a body snapshot that predates a remote update.
      this.pendingCheckpoint = null
      return
    }
    this.pendingCheckpoint = null
    this.inFlight = { ...pending, version: this.version }
    this.setSaveStatus('saving')
    this.sendCheckpoint(this.inFlight)
  }

  private sendCheckpoint(checkpoint: PendingCheckpoint): void {
    const socket = this.socket
    if (
      !this.initialized ||
      !socket ||
      socket.readyState !== WebSocket.OPEN
    ) return
    this.sendJson(socket, {
      type: 'live-checkpoint',
      // Reconnects replay the exact operation and room version. New
      // checkpoints get their version in sendPendingCheckpoint above.
      version: checkpoint.version ?? this.version,
      body: checkpoint.body,
      operation_id: checkpoint.operationId,
    })
  }

  private clearHandshakeTimer(): void {
    if (this.handshakeTimer !== null) globalThis.clearTimeout(this.handshakeTimer)
    this.handshakeTimer = null
  }

  private handleProtocolFailure(message: string): void {
    const error = new PhotonLiveError(message, 'protocol')
    this.currentError = error
    this.socket?.close()
    this.handleSocketFailure(error)
  }

  private handleSocketFailure(error: PhotonLiveError): void {
    if (this.disposed) return
    this.clearHandshakeTimer()
    this.socket = null
    this.setConnectionStatus('disconnected', error)
    if (this.initialized) this.hasUnackedChanges = this.hasUnackedChanges || Boolean(this.inFlight || this.pendingCheckpoint)
    if (!this.offline) this.scheduleReconnect()
  }

  private scheduleReconnect(): void {
    if (this.disposed || this.offline || this.reconnectTimer !== null) return
    this.reconnectTimer = globalThis.setTimeout(() => {
      this.reconnectTimer = null
      void this.authorizeAndConnect(false)
    }, this.backoff)
    this.backoff = Math.min(this.backoff * 2, MAX_BACKOFF_MS)
  }
}

export function createPhotonLiveProvider(
  options: PhotonLiveProviderOptions,
): PhotonLiveProvider {
  const provider = new PhotonLiveProviderImpl(options)
  provider.start()
  return provider
}
