/**
 * Photon sync edge — Engine proxy and Live Durable Object relay.
 *
 * `@quantum-box/photon/worker` retains the generic Engine/Yjs relay and its
 * storage on `/ws`. This entrypoint adds the private, authorization-backed
 * Photon Live room and checkpoint adapter used by the data editor.
 */

import { DurableObject } from 'cloudflare:workers'
import * as Y from 'yjs'
import photonWorkerDefault, {
  PhotonSyncRoom as PhotonSyncRoomBase,
} from '@quantum-box/photon/worker'

/**
 * The old Photon Engine relay remains available on `/ws`. Live uses a
 * different Durable Object namespace so a room name or ticket can never route
 * a browser into the old, generic relay.
 */
export { PhotonSyncRoomBase as PhotonSyncRoom }

export interface Env {
  /** Generic Photon Engine push/pull relay. */
  PHOTON_SYNC_ROOMS: DurableObjectNamespace<PhotonSyncRoomBase>
  /** Dedicated Photon Live rooms for data-editor collaboration. */
  PHOTON_LIVE_ROOMS: DurableObjectNamespace<PhotonLiveRoom>
  /** Singleton, short-lived, one-time ticket store. */
  PHOTON_LIVE_TICKETS: DurableObjectNamespace<PhotonLiveTicketStore>
  PHOTON_CLOUD_ENGINE_BASE_URL?: string
  PHOTON_EDGE_SERVICE_TOKEN?: string
  PHOTON_LIVE_API_BASE_URL?: string
  PHOTON_LIVE_ENABLED?: string
  PHOTON_LIVE_ALLOWED_ORIGINS?: string
}

const LIVE_SESSION_PATH = '/live/session'
const LIVE_WEBSOCKET_PATH = '/live/ws'
const LIVE_AUTHORIZE_SUFFIX = 'live/authorize'
const LIVE_CHECKPOINT_SUFFIX = 'live/checkpoint'
const LIVE_TICKET_STORE_NAME = 'live-ticket-store'
const LIVE_TICKET_KEY_PREFIX = 'live:ticket:'
const LIVE_SESSION_KEY_PREFIX = 'live:session:'
const LIVE_TICKET_TTL_MS = 60_000
const LIVE_ROOM_META_KEY = 'live:room:meta:v1'
const LIVE_PENDING_INIT_KEY = 'live:room:init-pending:v1'
const LIVE_PENDING_CHECKPOINT_KEY = 'live:checkpoint:pending:v1'
const LIVE_PENDING_CHECKPOINT_BODY_PREFIX = 'live:checkpoint:pending:body:'
const LIVE_CHECKPOINT_RESULT_PREFIX = 'live:checkpoint:result:'
const LIVE_SESSION_HEADER = 'x-photon-live-session'
const LIVE_INTERNAL_HEADER = 'x-photon-live-internal'
const LIVE_INTERNAL_HEADER_VALUE = '1'
const MAX_SESSION_BODY_BYTES = 64 * 1024
const MAX_API_RESPONSE_BYTES = 8 * 1024 * 1024
const MAX_FIELD_BYTES = 512
const MAX_ROOM_ID_BYTES = 512
const MAX_AUTHORIZATION_BYTES = 8 * 1024
const MAX_BODY_HASH_BYTES = 128
const MAX_TICKET_HEADER_BYTES = 32 * 1024
const MAX_CHECKPOINT_BODY_BYTES = 4 * 1024 * 1024
const CHECKPOINT_BODY_CHUNK_BYTES = 64 * 1024
const MAX_TEXT_FRAME_BYTES = MAX_CHECKPOINT_BODY_BYTES + 32 * 1024
const MAX_UPDATE_BYTES = 128 * 1024 - 4096
const MAX_AWARENESS_BYTES = 64 * 1024
const DEFAULT_CLOUD_ENGINE_BASE_URL = 'http://127.0.0.1:3001'

type JsonObject = Record<string, unknown>

interface LiveRoomIdentity {
  roomId: string
  tenant: string
  database: string
  data: string
  property: string
  format: string
}

interface LiveAuthorization extends LiveRoomIdentity {
  actorId: string
  body: string
  recordVersion: string
}

/** Data carried by a ticket and by the WebSocket session lookup. */
interface LiveTicketSession extends LiveRoomIdentity {
  actorId: string
  recordVersion: string
  bodyHash: string
  org: string
  repo: string
  authorization: string
  expiresAt: number
  sessionId?: string
  platformId?: string
  operatorId?: string
}

interface StoredLiveTicket extends LiveTicketSession {
  sessionId: string
}

interface LiveRoomMetadata extends LiveRoomIdentity {
  initialized: boolean
  version: number
  recordVersion?: string
  bodyHash: string
}

interface LivePendingInitialization {
  update: ArrayBuffer
  recordVersion: string
}

interface LivePendingCheckpoint {
  version: number
  operationId: string
  expectedRecordVersion: string
  bodyHash: string
  fingerprint: string
  bodyByteLength: number
  chunkCount: number
}

interface LiveCheckpointResult {
  version: number
  operationId: string
  recordVersion: string
  bodyHash: string
  fingerprint: string
}

interface LiveSocketAttachment {
  kind: 'photon-live'
  sessionId: string
  expiresAt: number
}

interface LiveFrame extends JsonObject {
  type?: unknown
  update?: unknown
  version?: unknown
  body?: unknown
  operation_id?: unknown
}

type CheckpointResult =
  | { kind: 'saved'; recordVersion: string }
  | { kind: 'conflict' }
  | { kind: 'error' }

function isRecord(value: unknown): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function byteLength(value: string): number {
  return new TextEncoder().encode(value).byteLength
}

function boundedString(value: unknown, maxBytes = MAX_FIELD_BYTES): string | null {
  if (typeof value !== 'string' || value.length === 0) return null
  if (byteLength(value) > maxBytes) return null
  if (value.includes('\u0000') || value.includes('\r') || value.includes('\n')) return null
  return value
}

function optionalHeader(value: string | null): string | undefined {
  if (!value) return undefined
  return boundedString(value) ?? undefined
}

function bearerAuthorization(value: string | null): string | null {
  if (!value || byteLength(value) > MAX_AUTHORIZATION_BYTES) return null
  const authorization = value.trim()
  if (!/^Bearer\s+\S+$/i.test(authorization)) return null
  return authorization
}

function isLiveEnabled(env: Env): boolean {
  const value = env.PHOTON_LIVE_ENABLED?.trim().toLowerCase()
  return value === '1' || value === 'true' || value === 'yes' || value === 'on'
}

function configuredLiveOrigins(env: Env): Set<string> {
  return new Set(
    (env.PHOTON_LIVE_ALLOWED_ORIGINS ?? '')
      .split(',')
      .map((origin) => origin.trim())
      .filter(Boolean),
  )
}

function isAllowedLiveOrigin(request: Request, env: Env): boolean {
  const origin = request.headers.get('origin')
  return origin !== null && configuredLiveOrigins(env).has(origin)
}

function liveCorsHeaders(request: Request, env: Env): Headers {
  const headers = new Headers({
    'access-control-allow-methods': 'GET,POST,OPTIONS',
    'access-control-allow-headers':
      'authorization,content-type,x-platform-id,x-operator-id,x-request-id,x-photon-request-id',
    'access-control-expose-headers': 'x-photon-request-id',
    vary: 'Origin',
  })
  const origin = request.headers.get('origin')
  if (origin && configuredLiveOrigins(env).has(origin)) {
    headers.set('access-control-allow-origin', origin)
  }
  return headers
}

function liveJsonResponse(
  request: Request,
  env: Env,
  payload: unknown,
  init: ResponseInit = {},
): Response {
  const headers = liveCorsHeaders(request, env)
  headers.set('content-type', 'application/json')
  if (init.headers) {
    new Headers(init.headers).forEach((value, key) => headers.set(key, value))
  }
  return new Response(JSON.stringify(payload), { ...init, headers })
}

function liveAccessFailure(request: Request, env: Env): Response | null {
  if (!isLiveEnabled(env)) {
    return liveJsonResponse(
      request,
      env,
      { code: 'LIVE_DISABLED', error: 'Photon Live is disabled' },
      { status: 404 },
    )
  }
  if (!isAllowedLiveOrigin(request, env)) {
    return liveJsonResponse(request, env, { error: 'Live origin is not allowed' }, { status: 403 })
  }
  return null
}

async function readJsonBody(request: Request, maxBytes: number): Promise<unknown> {
  const bytes = await request.arrayBuffer()
  if (bytes.byteLength > maxBytes) throw new Error('request body too large')
  return JSON.parse(new TextDecoder().decode(bytes)) as unknown
}

function base64Encode(bytes: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    const chunk = bytes.subarray(offset, Math.min(offset + chunkSize, bytes.length))
    binary += String.fromCharCode(...chunk)
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function base64Decode(value: string, maxBytes: number): Uint8Array | null {
  if (!value || byteLength(value) > Math.ceil(maxBytes * 4 / 3) + 8) return null
  if (!/^[A-Za-z0-9+/_-]*={0,2}$/.test(value)) return null
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/')
  if (normalized.length % 4 === 1) return null
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=')
  try {
    const decoded = atob(padded)
    if (decoded.length > maxBytes) return null
    const bytes = new Uint8Array(decoded.length)
    for (let index = 0; index < decoded.length; index += 1) {
      bytes[index] = decoded.charCodeAt(index)
    }
    return bytes
  } catch {
    return null
  }
}

function encodeSessionHeader(session: LiveTicketSession): string {
  return base64Encode(new TextEncoder().encode(JSON.stringify(session)))
}

function decodeSessionHeader(value: string | null): LiveTicketSession | null {
  if (!value || byteLength(value) > MAX_TICKET_HEADER_BYTES) return null
  const bytes = base64Decode(value, MAX_TICKET_HEADER_BYTES)
  if (!bytes) return null
  try {
    const parsed = JSON.parse(new TextDecoder().decode(bytes)) as unknown
    return isLiveTicketSession(parsed) ? parsed : null
  } catch {
    return null
  }
}

function isSafeRoomId(value: unknown): value is string {
  return boundedString(value, MAX_ROOM_ID_BYTES) !== null
}

function isLiveRoomIdentity(value: unknown): value is LiveRoomIdentity {
  if (!isRecord(value)) return false
  return (
    isSafeRoomId(value.roomId) &&
    boundedString(value.tenant) !== null &&
    boundedString(value.database) !== null &&
    boundedString(value.data) !== null &&
    boundedString(value.property) !== null &&
    boundedString(value.format) !== null
  )
}

function isLiveTicketSession(value: unknown): value is LiveTicketSession {
  if (!isLiveRoomIdentity(value) || !isRecord(value)) return false
  return (
    boundedString(value.actorId) !== null &&
    boundedString(value.recordVersion) !== null &&
    boundedString(value.bodyHash, MAX_BODY_HASH_BYTES) !== null &&
    boundedString(value.org) !== null &&
    boundedString(value.repo) !== null &&
    typeof value.authorization === 'string' &&
    bearerAuthorization(value.authorization) !== null &&
    typeof value.expiresAt === 'number' &&
    Number.isSafeInteger(value.expiresAt) &&
    value.expiresAt > 0 &&
    (value.sessionId === undefined || boundedString(value.sessionId) !== null) &&
    (value.platformId === undefined || boundedString(value.platformId) !== null) &&
    (value.operatorId === undefined || boundedString(value.operatorId) !== null)
  )
}

function isLiveRoomMetadata(value: unknown): value is LiveRoomMetadata {
  if (!isLiveRoomIdentity(value) || !isRecord(value)) return false
  return (
    typeof value.initialized === 'boolean' &&
    typeof value.version === 'number' &&
    Number.isSafeInteger(value.version) &&
    value.version >= 0 &&
    boundedString(value.bodyHash, MAX_BODY_HASH_BYTES) !== null &&
    (value.recordVersion === undefined || boundedString(value.recordVersion) !== null)
  )
}

function isPendingCheckpoint(value: unknown): value is LivePendingCheckpoint {
  if (!isRecord(value)) return false
  return (
    integerField(value.version) !== null &&
    boundedString(value.operationId) !== null &&
    boundedString(value.expectedRecordVersion) !== null &&
    boundedString(value.bodyHash, MAX_BODY_HASH_BYTES) !== null &&
    boundedString(value.fingerprint) !== null &&
    typeof value.bodyByteLength === 'number' &&
    Number.isSafeInteger(value.bodyByteLength) &&
    value.bodyByteLength >= 0 &&
    value.bodyByteLength <= MAX_CHECKPOINT_BODY_BYTES &&
    typeof value.chunkCount === 'number' &&
    Number.isSafeInteger(value.chunkCount) &&
    value.chunkCount >= 0 &&
    value.chunkCount <= Math.ceil(MAX_CHECKPOINT_BODY_BYTES / CHECKPOINT_BODY_CHUNK_BYTES)
  )
}

function isCheckpointResult(value: unknown): value is LiveCheckpointResult {
  if (!isRecord(value)) return false
  return (
    integerField(value.version) !== null &&
    boundedString(value.operationId) !== null &&
    boundedString(value.recordVersion) !== null &&
    boundedString(value.bodyHash, MAX_BODY_HASH_BYTES) !== null &&
    boundedString(value.fingerprint) !== null
  )
}

function sameRoomIdentity(left: LiveRoomIdentity, right: LiveRoomIdentity): boolean {
  return (
    left.roomId === right.roomId &&
    left.tenant === right.tenant &&
    left.database === right.database &&
    left.data === right.data &&
    left.property === right.property &&
    left.format === right.format
  )
}

async function fallbackRoomId(identity: Omit<LiveRoomIdentity, 'roomId'>): Promise<string> {
  const source = JSON.stringify([
    identity.tenant,
    identity.database,
    identity.data,
    identity.property,
    identity.format,
  ])
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(source))
  return `live:${base64Encode(new Uint8Array(digest))}`
}

function canonicalField(payload: JsonObject, canonical: string, legacy: string): string | null {
  return boundedString(payload[canonical]) ?? boundedString(payload[legacy])
}

async function normalizeAuthorizationResponse(payload: unknown): Promise<LiveAuthorization | null> {
  if (!isRecord(payload)) return null

  const tenant = canonicalField(payload, 'tenant', 'tenant_id')
  const database = canonicalField(payload, 'database', 'database_id')
  const data = canonicalField(payload, 'data', 'data_id')
  const property = canonicalField(payload, 'property', 'property_id')
  const actorId = canonicalField(payload, 'actor_id', 'actorId')
  const format = boundedString(payload.format)
  const body = typeof payload.body === 'string' && byteLength(payload.body) <= MAX_CHECKPOINT_BODY_BYTES
    ? payload.body
    : null
  const recordVersion = canonicalField(payload, 'record_version', 'recordVersion')

  if (!tenant || !database || !data || !property || !actorId || !format || body === null || !recordVersion) {
    return null
  }

  const suppliedRoomId = boundedString(payload.room_id, MAX_ROOM_ID_BYTES)
  const roomId = suppliedRoomId ?? await fallbackRoomId({ tenant, database, data, property, format })
  if (!isSafeRoomId(roomId)) return null

  return { roomId, tenant, database, data, property, format, actorId, body, recordVersion }
}

function liveApiBaseUrl(env: Env): string {
  return (env.PHOTON_LIVE_API_BASE_URL ?? env.PHOTON_CLOUD_ENGINE_BASE_URL ?? DEFAULT_CLOUD_ENGINE_BASE_URL)
    .replace(/\/$/, '')
}

function routeLiveApiUrl(
  env: Env,
  suffix: string,
  session: Pick<LiveTicketSession, 'org' | 'repo' | 'data'>,
): string {
  const encodedOrg = encodeURIComponent(session.org)
  const encodedRepo = encodeURIComponent(session.repo)
  const encodedData = encodeURIComponent(session.data)
  return `${liveApiBaseUrl(env)}/v1beta/repos/${encodedOrg}/${encodedRepo}/data/${encodedData}/${suffix}`
}

function requestHeadersForApi(session: LiveTicketSession, requestId: string): Headers {
  const headers = new Headers({
    authorization: session.authorization,
    'content-type': 'application/json',
    'x-photon-request-id': requestId,
  })
  if (session.platformId) headers.set('x-platform-id', session.platformId)
  if (session.operatorId) headers.set('x-operator-id', session.operatorId)
  return headers
}

async function authorizeLiveSession(
  request: Request,
  env: Env,
  input: { org: string; repo: string; dataId: string; propertyId: string },
): Promise<{ status: number; authorization?: LiveAuthorization }> {
  const authorization = bearerAuthorization(request.headers.get('authorization'))
  if (!authorization) return { status: 401 }

  const requestId = request.headers.get('x-photon-request-id') ?? request.headers.get('x-request-id') ?? crypto.randomUUID()
  const endpoint = `${liveApiBaseUrl(env)}/v1beta/repos/${encodeURIComponent(input.org)}/${encodeURIComponent(input.repo)}` +
    `/data/${encodeURIComponent(input.dataId)}/${LIVE_AUTHORIZE_SUFFIX}`
  let response: Response
  try {
    const platformId = optionalHeader(request.headers.get('x-platform-id'))
    const operatorId = optionalHeader(request.headers.get('x-operator-id'))
    const headers = new Headers({
      authorization,
      'content-type': 'application/json',
      'x-photon-request-id': requestId,
    })
    if (platformId) headers.set('x-platform-id', platformId)
    if (operatorId) headers.set('x-operator-id', operatorId)
    response = await fetch(endpoint, {
      method: 'POST',
      headers,
      body: JSON.stringify({ property_id: input.propertyId }),
      signal: AbortSignal.timeout(15_000),
    })
  } catch {
    return { status: 502 }
  }

  if (!response.ok) {
    // Never return the API error body here. An authorization failure must not
    // disclose the canonical record body or room metadata.
    return { status: response.status >= 400 && response.status < 500 ? response.status : 502 }
  }

  try {
    const responseBytes = await response.arrayBuffer()
    if (responseBytes.byteLength > MAX_API_RESPONSE_BYTES) return { status: 502 }
    const authorizationResponse = await normalizeAuthorizationResponse(
      JSON.parse(new TextDecoder().decode(responseBytes)) as unknown,
    )
    return authorizationResponse ? { status: 200, authorization: authorizationResponse } : { status: 502 }
  } catch {
    return { status: 502 }
  }
}

function ticketStore(env: Env): DurableObjectStub<PhotonLiveTicketStore> {
  const id = env.PHOTON_LIVE_TICKETS.idFromName(LIVE_TICKET_STORE_NAME)
  return env.PHOTON_LIVE_TICKETS.get(id)
}

async function createLiveSession(request: Request, env: Env): Promise<Response> {
  let payload: unknown
  try {
    payload = await readJsonBody(request, MAX_SESSION_BODY_BYTES)
  } catch {
    return liveJsonResponse(request, env, { error: 'Invalid session request' }, { status: 400 })
  }
  if (!isRecord(payload)) {
    return liveJsonResponse(request, env, { error: 'Invalid session request' }, { status: 400 })
  }

  const org = boundedString(payload.org)
  const repo = boundedString(payload.repo)
  const dataId = boundedString(payload.data_id)
  const propertyId = boundedString(payload.property_id)
  if (!org || !repo || !dataId || !propertyId) {
    return liveJsonResponse(request, env, { error: 'Invalid session request' }, { status: 400 })
  }

  const authorized = await authorizeLiveSession(request, env, { org, repo, dataId, propertyId })
  if (!authorized.authorization) {
    return liveJsonResponse(request, env, { error: 'Live authorization failed' }, { status: authorized.status })
  }

  const canonical = authorized.authorization
  const authorization = bearerAuthorization(request.headers.get('authorization'))
  if (!authorization) {
    return liveJsonResponse(request, env, { error: 'Live authorization failed' }, { status: 401 })
  }
  const bodyHash = await normalizeBodyHash(canonical.format, canonical.body)
  const session: LiveTicketSession = {
    roomId: canonical.roomId,
    tenant: canonical.tenant,
    database: canonical.database,
    data: canonical.data,
    property: canonical.property,
    format: canonical.format,
    actorId: canonical.actorId,
    recordVersion: canonical.recordVersion,
    bodyHash,
    org,
    repo,
    authorization,
    expiresAt: Date.now() + LIVE_TICKET_TTL_MS,
    platformId: optionalHeader(request.headers.get('x-platform-id')),
    operatorId: optionalHeader(request.headers.get('x-operator-id')),
  }

  let ticket: string
  try {
    const issued = await ticketStore(env).issue(session)
    ticket = issued.ticket
  } catch {
    return liveJsonResponse(request, env, { error: 'Live session unavailable' }, { status: 503 })
  }

  return liveJsonResponse(request, env, {
    ticket,
    room_id: canonical.roomId,
    actor_id: canonical.actorId,
    format: canonical.format,
    body: canonical.body,
    record_version: canonical.recordVersion,
  })
}

async function openLiveWebSocket(request: Request, env: Env): Promise<Response> {
  if (request.method !== 'GET') {
    return liveJsonResponse(request, env, { error: 'Method not allowed' }, { status: 405 })
  }
  if (request.headers.get('upgrade')?.toLowerCase() !== 'websocket') {
    return liveJsonResponse(request, env, { error: 'Expected WebSocket upgrade' }, { status: 426 })
  }

  const ticket = new URL(request.url).searchParams.get('ticket')
  if (!ticket || !/^[A-Za-z0-9_-]{40,64}$/.test(ticket)) {
    return liveJsonResponse(request, env, { error: 'Invalid or expired Live ticket' }, { status: 401 })
  }

  let session: LiveTicketSession | null
  try {
    session = await ticketStore(env).consume(ticket)
  } catch {
    return liveJsonResponse(request, env, { error: 'Live session unavailable' }, { status: 503 })
  }
  if (!session) {
    return liveJsonResponse(request, env, { error: 'Invalid or expired Live ticket' }, { status: 401 })
  }

  const roomId = env.PHOTON_LIVE_ROOMS.idFromName(session.roomId)
  const internalUrl = new URL(request.url)
  internalUrl.pathname = '/live/internal-ws'
  internalUrl.search = ''
  const internalHeaders = new Headers(request.headers)
  internalHeaders.set(LIVE_INTERNAL_HEADER, LIVE_INTERNAL_HEADER_VALUE)
  internalHeaders.set(LIVE_SESSION_HEADER, encodeSessionHeader(session))
  const internalRequest = new Request(internalUrl, {
    method: 'GET',
    headers: internalHeaders,
  })
  return env.PHOTON_LIVE_ROOMS.get(roomId).fetch(internalRequest)
}

function sendJsonFrame(socket: WebSocket, payload: JsonObject): void {
  try {
    socket.send(JSON.stringify(payload))
  } catch {
    // A concurrently closed socket needs no further action.
  }
}

function sendSocketError(socket: WebSocket, message: string, operationId?: string): void {
  sendJsonFrame(socket, {
    type: 'live-error',
    message,
    ...(operationId ? { operation_id: operationId } : {}),
  })
}

function updateBytes(message: ArrayBuffer | ArrayBufferView): Uint8Array {
  if (message instanceof ArrayBuffer) return new Uint8Array(message)
  return new Uint8Array(message.buffer, message.byteOffset, message.byteLength)
}

function ownedArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength)
  copy.set(bytes)
  return copy.buffer
}

function checkpointResultKey(operationId: string): string {
  return `${LIVE_CHECKPOINT_RESULT_PREFIX}${base64Encode(new TextEncoder().encode(operationId))}`
}

function checkpointBodyKey(index: number): string {
  return `${LIVE_PENDING_CHECKPOINT_BODY_PREFIX}${index.toString().padStart(6, '0')}`
}

async function fingerprintBody(body: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(body))
  return base64Encode(new Uint8Array(digest))
}

function normalizeJsonForBodyHash(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(normalizeJsonForBodyHash)
  if (!isRecord(value)) return value
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, normalizeJsonForBodyHash(value[key])]),
  )
}

/**
 * Hash the canonical body that seeded a room. RichText is JSON, so object key
 * order is normalized before hashing; otherwise an equivalent API response
 * could look like an external body edit after a record-version-only change.
 */
export async function normalizeBodyHash(format: string, body: string): Promise<string> {
  let normalized = body
  if (format === 'richText') {
    try {
      normalized = JSON.stringify(normalizeJsonForBodyHash(JSON.parse(body))) ?? body
    } catch {
      // The checkpoint API remains responsible for rejecting malformed
      // RichText. Hashing the raw value here keeps the authorization path
      // deterministic without accepting it as a valid checkpoint.
    }
  }
  return fingerprintBody(`${format}\u0000${normalized}`)
}

function validYjsUpdate(update: Uint8Array): boolean {
  if (update.byteLength === 0 || update.byteLength > MAX_UPDATE_BYTES) return false
  const doc = new Y.Doc()
  try {
    Y.applyUpdate(doc, update)
    return true
  } catch {
    return false
  } finally {
    doc.destroy()
  }
}

function parseLiveFrame(message: string): LiveFrame | null {
  if (byteLength(message) > MAX_TEXT_FRAME_BYTES) return null
  try {
    const parsed = JSON.parse(message) as unknown
    return isRecord(parsed) ? parsed : null
  } catch {
    return null
  }
}

function integerField(value: unknown): number | null {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : null
}

/**
 * Record versions are opaque strings at the API boundary, but the canonical
 * data API emits monotonically increasing integer versions. A replay may
 * repair metadata after a DO restart; it must never move the room backward if
 * a later checkpoint has already advanced the stored version.
 */
function isRecordVersionNewer(candidate: string, current: string | undefined): boolean {
  if (!current) return true
  if (candidate === current) return false
  try {
    return BigInt(candidate) > BigInt(current)
  } catch {
    // Opaque/non-numeric versions cannot be ordered safely. Keeping the
    // current value is safer than treating an old replay as a newer commit.
    return false
  }
}

function roomReadyFrame(meta: LiveRoomMetadata, session: LiveTicketSession): JsonObject {
  return {
    type: 'live-ready',
    initialized: meta.initialized,
    version: meta.version,
    record_version: meta.recordVersion ?? session.recordVersion,
  }
}

export class PhotonLiveTicketStore extends DurableObject<Env> {
  async issue(session: LiveTicketSession): Promise<{ ticket: string; sessionId: string }> {
    const ticketBytes = new Uint8Array(32)
    const sessionBytes = new Uint8Array(18)
    crypto.getRandomValues(ticketBytes)
    crypto.getRandomValues(sessionBytes)
    const ticket = base64Encode(ticketBytes)
    const sessionId = base64Encode(sessionBytes)
    const stored: StoredLiveTicket = {
      ...session,
      sessionId,
      expiresAt: Date.now() + LIVE_TICKET_TTL_MS,
    }
    await this.ctx.storage.put(`${LIVE_TICKET_KEY_PREFIX}${ticket}`, stored)
    return { ticket, sessionId }
  }

  async consume(ticket: string): Promise<LiveTicketSession | null> {
    if (!/^[A-Za-z0-9_-]{40,64}$/.test(ticket)) return null
    const key = `${LIVE_TICKET_KEY_PREFIX}${ticket}`
    return this.ctx.storage.transaction(async (transaction) => {
      const stored = await transaction.get<StoredLiveTicket>(key)
      if (!stored || stored.expiresAt <= Date.now() || !isLiveTicketSession(stored)) {
        if (stored) await transaction.delete(key)
        return null
      }
      await transaction.delete(key)
      if (!stored.sessionId) return null
      await transaction.put(`${LIVE_SESSION_KEY_PREFIX}${stored.sessionId}`, stored)
      return stored
    })
  }

  async getSession(sessionId: string): Promise<LiveTicketSession | null> {
    if (!/^[A-Za-z0-9_-]{20,64}$/.test(sessionId)) return null
    const key = `${LIVE_SESSION_KEY_PREFIX}${sessionId}`
    return this.ctx.storage.transaction(async (transaction) => {
      const stored = await transaction.get<StoredLiveTicket>(key)
      if (!stored || stored.expiresAt <= Date.now() || !isLiveTicketSession(stored)) {
        if (stored) await transaction.delete(key)
        return null
      }
      return stored
    })
  }

  async deleteSession(sessionId: string): Promise<void> {
    if (!/^[A-Za-z0-9_-]{20,64}$/.test(sessionId)) return
    await this.ctx.storage.delete(`${LIVE_SESSION_KEY_PREFIX}${sessionId}`)
  }
}

export class PhotonLiveRoom extends PhotonSyncRoomBase {
  private readonly liveEnv: Env
  private readonly sessions = new WeakMap<WebSocket, LiveTicketSession>()

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env)
    this.liveEnv = env
  }

  async fetch(request: Request): Promise<Response> {
    if (request.headers.get(LIVE_INTERNAL_HEADER) !== LIVE_INTERNAL_HEADER_VALUE) {
      return new Response('Not found', { status: 404 })
    }
    if (request.headers.get('upgrade')?.toLowerCase() !== 'websocket') {
      return new Response('Expected WebSocket upgrade', { status: 426 })
    }

    const session = decodeSessionHeader(request.headers.get(LIVE_SESSION_HEADER))
    if (!session || !session.sessionId || session.expiresAt <= Date.now()) {
      return new Response('Unauthorized', { status: 401 })
    }
    let roomMetadata = await this.ensureRoomMetadata(session)
    if (!roomMetadata) return new Response('Forbidden', { status: 403 })
    if (roomMetadata.bodyHash !== session.bodyHash) {
      // A fresh authorization with a different canonical body means the
      // durable room no longer represents the record the caller opened.
      // Never send that stale snapshot to the browser.
      return new Response('Live body changed', { status: 409 })
    }
    if (
      roomMetadata.initialized &&
      roomMetadata.recordVersion &&
      roomMetadata.recordVersion !== session.recordVersion
    ) {
      // Title/other-property edits can advance RecordVersion while this body
      // remains unchanged. Move the room's canonical version forward in that
      // case, while accepting an older session without ever moving backward.
      if (isRecordVersionNewer(session.recordVersion, roomMetadata.recordVersion)) {
        roomMetadata = {
          ...roomMetadata,
          recordVersion: session.recordVersion,
        }
        await this.ctx.storage.put(LIVE_ROOM_META_KEY, roomMetadata)
      }
    }
    const recoveredMetadata = await this.recoverPendingInitialization()
    if (!recoveredMetadata) return new Response('Live room recovery failed', { status: 503 })

    const before = new Set(this.ctx.getWebSockets())
    const response = await super.fetch(request)
    const socket = this.ctx.getWebSockets().find((candidate) => !before.has(candidate))
    if (!socket) return response

    this.sessions.set(socket, session)
    try {
      socket.serializeAttachment({
        kind: 'photon-live',
        sessionId: session.sessionId,
        expiresAt: session.expiresAt,
      } satisfies LiveSocketAttachment)
    } catch {
      // The attachment is an optimization for hibernation; the live instance
      // still has the in-memory session and can safely serve this connection.
    }
    await this.scheduleSessionExpiry(session.expiresAt)
    const current = (await this.ctx.storage.get<LiveRoomMetadata>(LIVE_ROOM_META_KEY)) ?? recoveredMetadata
    sendJsonFrame(socket, roomReadyFrame(current, session))
    return response
  }

  async webSocketMessage(sender: WebSocket, message: string | ArrayBuffer | ArrayBufferView): Promise<void> {
    const session = await this.resolveSocketSession(sender)
    if (!session) {
      try {
        sender.close(4401, 'Live session expired')
      } catch {
        // Ignore a socket that has already closed.
      }
      return
    }
    if (session.expiresAt <= Date.now()) {
      try {
        sender.close(4401, 'Live session expired')
      } catch {
        // Ignore a socket that has already closed.
      }
      return
    }

    if (typeof message === 'string') {
      await this.handleTextFrame(sender, session, message)
      return
    }
    await this.handleBinaryFrame(sender, session, updateBytes(message))
  }

  async alarm(): Promise<void> {
    const now = Date.now()
    let nextExpiry: number | null = null
    for (const socket of this.ctx.getWebSockets()) {
      let attachment: LiveSocketAttachment | null = null
      try {
        const value = socket.deserializeAttachment() as unknown
        if (
          isRecord(value) &&
          value.kind === 'photon-live' &&
          typeof value.sessionId === 'string' &&
          typeof value.expiresAt === 'number'
        ) {
          attachment = value as unknown as LiveSocketAttachment
        }
      } catch {
        attachment = null
      }
      const cached = this.sessions.get(socket)
      const expiresAt = cached?.expiresAt ?? attachment?.expiresAt
      if (!expiresAt || expiresAt <= now) {
        try {
          socket.close(4401, 'Live session expired')
        } catch {
          // Ignore a socket that has already closed.
        }
        if (attachment?.sessionId) {
          await ticketStore(this.liveEnv).deleteSession(attachment.sessionId)
        } else if (cached?.sessionId) {
          await ticketStore(this.liveEnv).deleteSession(cached.sessionId)
        }
        continue
      }
      nextExpiry = nextExpiry === null ? expiresAt : Math.min(nextExpiry, expiresAt)
    }
    if (nextExpiry === null) {
      await this.ctx.storage.deleteAlarm()
    } else {
      await this.ctx.storage.setAlarm(nextExpiry)
    }
  }

  private async resolveSocketSession(socket: WebSocket): Promise<LiveTicketSession | null> {
    const cached = this.sessions.get(socket)
    if (cached && cached.expiresAt > Date.now()) return cached

    let attachment: LiveSocketAttachment | null = null
    try {
      const value = socket.deserializeAttachment() as unknown
      if (
        isRecord(value) &&
        value.kind === 'photon-live' &&
        typeof value.sessionId === 'string' &&
        typeof value.expiresAt === 'number'
      ) {
        attachment = value as unknown as LiveSocketAttachment
      }
    } catch {
      return null
    }
    if (!attachment || attachment.expiresAt <= Date.now()) return null

    try {
      const session = await ticketStore(this.liveEnv).getSession(attachment.sessionId)
      if (!session || session.expiresAt <= Date.now() || session.expiresAt !== attachment.expiresAt) return null
      this.sessions.set(socket, session)
      return session
    } catch {
      return null
    }
  }

  private async scheduleSessionExpiry(expiresAt: number): Promise<void> {
    const current = await this.ctx.storage.getAlarm()
    if (current === null || expiresAt < current) {
      await this.ctx.storage.setAlarm(expiresAt)
    }
  }

  private async ensureRoomMetadata(session: LiveTicketSession): Promise<LiveRoomMetadata | null> {
    return this.ctx.blockConcurrencyWhile(async () => {
      const current = await this.ctx.storage.get<LiveRoomMetadata>(LIVE_ROOM_META_KEY)
      if (current) {
        return isLiveRoomMetadata(current) && sameRoomIdentity(current, session) ? current : null
      }
      const initial: LiveRoomMetadata = {
        roomId: session.roomId,
        tenant: session.tenant,
        database: session.database,
        data: session.data,
        property: session.property,
        format: session.format,
        initialized: false,
        version: 0,
        bodyHash: session.bodyHash,
      }
      await this.ctx.storage.put(LIVE_ROOM_META_KEY, initial)
      return initial
    })
  }

  private async currentRoomMetadata(): Promise<LiveRoomMetadata | null> {
    const metadata = await this.ctx.storage.get<LiveRoomMetadata>(LIVE_ROOM_META_KEY)
    return metadata && isLiveRoomMetadata(metadata) ? metadata : null
  }

  /**
   * Initialization has two durable phases: a pending marker and the room
   * metadata flag. If a DO is evicted between the upstream Yjs write and the
   * metadata write, replaying the same update is idempotent and lets the room
   * finish initialization without accepting a second seed from a client.
   */
  private async recoverPendingInitialization(): Promise<LiveRoomMetadata | null> {
    return this.ctx.blockConcurrencyWhile(async () => {
      const metadata = await this.currentRoomMetadata()
      if (!metadata) return null
      const pending = await this.ctx.storage.get<LivePendingInitialization>(LIVE_PENDING_INIT_KEY)
      if (!pending) return metadata
      if (metadata.initialized) {
        await this.ctx.storage.delete(LIVE_PENDING_INIT_KEY)
        return metadata
      }
      if (
        !(pending.update instanceof ArrayBuffer) ||
        pending.update.byteLength === 0 ||
        pending.update.byteLength > MAX_UPDATE_BYTES ||
        !validYjsUpdate(new Uint8Array(pending.update))
      ) {
        return null
      }

      try {
        // There may be no connected socket after a hibernation restart. The
        // upstream relay only uses sender for excluding its echo, so an
        // undefined sender safely persists and broadcasts the replay.
        await super.webSocketMessage(undefined as unknown as WebSocket, new Uint8Array(pending.update))
      } catch {
        return null
      }
      const initialized: LiveRoomMetadata = {
        ...metadata,
        initialized: true,
        version: 0,
        recordVersion: pending.recordVersion,
      }
      await this.ctx.storage.put(LIVE_ROOM_META_KEY, initialized)
      await this.ctx.storage.delete(LIVE_PENDING_INIT_KEY)
      return initialized
    })
  }

  private broadcast(payload: JsonObject): void {
    const frame = JSON.stringify(payload)
    for (const socket of this.ctx.getWebSockets()) {
      try {
        socket.send(frame)
      } catch {
        // A concurrently closed socket needs no further action.
      }
    }
  }

  private async handleTextFrame(
    sender: WebSocket,
    session: LiveTicketSession,
    message: string,
  ): Promise<void> {
    const frame = parseLiveFrame(message)
    if (!frame || typeof frame.type !== 'string') {
      sendSocketError(sender, 'Invalid Live message')
      return
    }

    if (frame.type === 'awareness') {
      const update = boundedString(frame.update, MAX_AWARENESS_BYTES)
      if (!update || !base64Decode(update, MAX_AWARENESS_BYTES)) {
        sendSocketError(sender, 'Invalid awareness update')
        return
      }
      const metadata = await this.currentRoomMetadata()
      if (!metadata?.initialized || !sameRoomIdentity(metadata, session)) return
      await super.webSocketMessage(sender, message)
      return
    }

    if (frame.type === 'live-initialize') {
      const encodedUpdate = boundedString(frame.update, MAX_UPDATE_BYTES * 2)
      const update = encodedUpdate ? base64Decode(encodedUpdate, MAX_UPDATE_BYTES) : null
      if (!update || !validYjsUpdate(update)) {
        sendSocketError(sender, 'Invalid initialization update')
        return
      }
      await this.handleInitialization(sender, session, update)
      return
    }

    if (frame.type === 'live-checkpoint') {
      await this.handleCheckpoint(sender, session, frame)
    }
  }

  private async handleInitialization(
    sender: WebSocket,
    session: LiveTicketSession,
    update: Uint8Array,
  ): Promise<void> {
    await this.ctx.blockConcurrencyWhile(async () => {
      const metadata = await this.currentRoomMetadata()
      if (!metadata || !sameRoomIdentity(metadata, session)) {
        sendSocketError(sender, 'Live room identity mismatch')
        return
      }
      if (metadata.initialized) {
        // A losing initializer must not be applied a second time. Its socket
        // already received the winning update through the normal broadcast.
        sendJsonFrame(sender, roomReadyFrame(metadata, session))
        return
      }

      await this.ctx.storage.put(LIVE_PENDING_INIT_KEY, {
        update: ownedArrayBuffer(update),
        recordVersion: session.recordVersion,
      } satisfies LivePendingInitialization)
      try {
        await super.webSocketMessage(sender, update)
      } catch {
        sendSocketError(sender, 'Live initialization failed')
        return
      }

      const initialized: LiveRoomMetadata = {
        ...metadata,
        initialized: true,
        version: 0,
        recordVersion: session.recordVersion,
      }
      await this.ctx.storage.put(LIVE_ROOM_META_KEY, initialized)
      await this.ctx.storage.delete(LIVE_PENDING_INIT_KEY)
      // Upstream intentionally omits the sender from binary broadcasts. The
      // initializing client needs an echo so its main Y.Doc receives the seed.
      try {
        sender.send(update)
      } catch {
        // The room state is still initialized for the remaining sockets.
      }
      this.broadcast(roomReadyFrame(initialized, session))
    })
  }

  private async handleBinaryFrame(
    sender: WebSocket,
    session: LiveTicketSession,
    update: Uint8Array,
  ): Promise<void> {
    if (!validYjsUpdate(update)) {
      sendSocketError(sender, 'Invalid or oversized Live update')
      return
    }

    await this.ctx.blockConcurrencyWhile(async () => {
      const metadata = await this.currentRoomMetadata()
      if (!metadata || !sameRoomIdentity(metadata, session)) {
        sendSocketError(sender, 'Live room identity mismatch')
        return
      }
      if (!metadata.initialized) {
        sendSocketError(sender, 'Live document is not initialized')
        return
      }
      if (metadata.version >= Number.MAX_SAFE_INTEGER) {
        sendSocketError(sender, 'Live version limit reached')
        return
      }

      try {
        await super.webSocketMessage(sender, update)
      } catch {
        sendSocketError(sender, 'Live update failed')
        return
      }

      const nextMetadata: LiveRoomMetadata = {
        ...metadata,
        version: metadata.version + 1,
      }
      await this.ctx.storage.put(LIVE_ROOM_META_KEY, nextMetadata)
      this.broadcast({ type: 'live-version', version: nextMetadata.version })
    })
  }

  private async readPendingCheckpointBody(pending: LivePendingCheckpoint): Promise<string | null> {
    const keys = Array.from({ length: pending.chunkCount }, (_, index) => checkpointBodyKey(index))
    const chunks = await this.ctx.storage.get<ArrayBuffer>(keys)
    if (chunks.size !== pending.chunkCount) return null
    const bytes = new Uint8Array(pending.bodyByteLength)
    let offset = 0
    for (let index = 0; index < pending.chunkCount; index += 1) {
      const chunk = chunks.get(checkpointBodyKey(index))
      if (!(chunk instanceof ArrayBuffer) || chunk.byteLength > CHECKPOINT_BODY_CHUNK_BYTES) return null
      if (offset + chunk.byteLength > bytes.byteLength) return null
      bytes.set(new Uint8Array(chunk), offset)
      offset += chunk.byteLength
    }
    if (offset !== pending.bodyByteLength) return null
    return new TextDecoder().decode(bytes)
  }

  private async savePendingCheckpoint(pending: LivePendingCheckpoint, body: string): Promise<void> {
    const bytes = new TextEncoder().encode(body)
    const entries: Record<string, ArrayBuffer> = {}
    for (let offset = 0, index = 0; offset < bytes.byteLength; offset += CHECKPOINT_BODY_CHUNK_BYTES, index += 1) {
      entries[checkpointBodyKey(index)] = ownedArrayBuffer(
        bytes.subarray(offset, Math.min(offset + CHECKPOINT_BODY_CHUNK_BYTES, bytes.byteLength)),
      )
    }
    await this.ctx.storage.transaction(async (transaction) => {
      await transaction.put(LIVE_PENDING_CHECKPOINT_KEY, pending)
      if (Object.keys(entries).length > 0) await transaction.put(entries)
    })
  }

  private async deletePendingCheckpoint(): Promise<void> {
    await this.ctx.storage.transaction(async (transaction) => {
      const pending = await transaction.get<LivePendingCheckpoint>(LIVE_PENDING_CHECKPOINT_KEY)
      if (pending && isPendingCheckpoint(pending)) {
        const keys = Array.from({ length: pending.chunkCount }, (_, index) => checkpointBodyKey(index))
        if (keys.length > 0) await transaction.delete(keys)
      }
      await transaction.delete(LIVE_PENDING_CHECKPOINT_KEY)
    })
  }

  private async readCheckpointResult(operationId: string): Promise<LiveCheckpointResult | null> {
    const result = await this.ctx.storage.get<LiveCheckpointResult>(checkpointResultKey(operationId))
    return result && isCheckpointResult(result) ? result : null
  }

  private async saveCheckpointResult(result: LiveCheckpointResult): Promise<void> {
    await this.ctx.storage.put(checkpointResultKey(result.operationId), result)
  }

  /**
   * A page reload can lose the actor that created a pending checkpoint. A new
   * authorized session may therefore need to inspect the canonical record
   * before replacing that pending request. This call forwards the new user's
   * bearer and is deliberately scoped to the room's canonical Property.
   */
  private async authorizeCurrentSession(session: LiveTicketSession): Promise<LiveAuthorization | null> {
    try {
      const request = new Request('https://photon-live.internal/live/authorize', {
        method: 'POST',
        headers: requestHeadersForApi(session, `live_recover_${crypto.randomUUID()}`),
        body: JSON.stringify({ property_id: session.property }),
      })
      const result = await authorizeLiveSession(request, this.liveEnv, {
        org: session.org,
        repo: session.repo,
        dataId: session.data,
        propertyId: session.property,
      })
      if (!result.authorization || !sameRoomIdentity(result.authorization, session)) return null
      return result.authorization
    } catch {
      return null
    }
  }

  private async handleCheckpoint(
    sender: WebSocket,
    session: LiveTicketSession,
    frame: LiveFrame,
  ): Promise<void> {
    const version = integerField(frame.version)
    const body = typeof frame.body === 'string' && byteLength(frame.body) <= MAX_CHECKPOINT_BODY_BYTES
      ? frame.body
      : null
    const operationId = boundedString(frame.operation_id)
    if (version === null || body === null || !operationId) {
      sendSocketError(sender, 'Invalid checkpoint', operationId ?? undefined)
      return
    }

    await this.ctx.blockConcurrencyWhile(async () => {
      let metadata = await this.currentRoomMetadata()
      if (!metadata || !sameRoomIdentity(metadata, session)) {
        sendSocketError(sender, 'Live room identity mismatch', operationId)
        return
      }
      if (!metadata.initialized) {
        sendSocketError(sender, 'Live document is not initialized', operationId)
        return
      }

      const fingerprint = await fingerprintBody(body)
      const pendingValue = await this.ctx.storage.get<LivePendingCheckpoint>(LIVE_PENDING_CHECKPOINT_KEY)
      let pending = pendingValue === undefined
        ? null
        : isPendingCheckpoint(pendingValue)
          ? pendingValue
          : undefined
      if (pending === undefined) {
        sendSocketError(sender, 'Live checkpoint recovery failed', operationId)
        return
      }
      let refreshedAuthorization: LiveAuthorization | null = null

      // If an earlier API call completed while its WebSocket ACK was lost,
      // replay the durable result without changing the expected CAS version.
      const priorResult = await this.readCheckpointResult(operationId)
      if (priorResult) {
        if (
          priorResult.fingerprint !== fingerprint ||
          priorResult.version !== version ||
          (pending && (pending.operationId !== operationId || pending.fingerprint !== fingerprint))
        ) {
          sendSocketError(sender, 'Checkpoint operation was reused', operationId)
          return
        }
        if (isRecordVersionNewer(priorResult.recordVersion, metadata.recordVersion)) {
          await this.ctx.storage.put(LIVE_ROOM_META_KEY, {
            ...metadata,
            recordVersion: priorResult.recordVersion,
            bodyHash: priorResult.bodyHash,
          } satisfies LiveRoomMetadata)
        }
        if (pending) await this.deletePendingCheckpoint()
        sendJsonFrame(sender, {
          type: 'live-saved',
          // This ACK describes the operation being replayed. The room may
          // already have a newer working version, which must not be reported
          // as saved by this old operation.
          version: priorResult.version,
          record_version: priorResult.recordVersion,
          operation_id: operationId,
        })
        return
      }

      if (pending && pending.operationId === operationId && pending.fingerprint !== fingerprint) {
        sendSocketError(sender, 'Checkpoint operation was reused', operationId)
        return
      }

      if (pending && pending.operationId !== operationId) {
        // The original page may have disappeared after the pending marker was
        // written. Re-authorize with the new caller before releasing the gate:
        // CAS decides any race with the old API request, while the canonical
        // body check prevents silently adopting an external body change.
        const authorized = await this.authorizeCurrentSession(session)
        if (!authorized) {
          sendSocketError(sender, 'Live checkpoint recovery failed', operationId)
          return
        }
        refreshedAuthorization = authorized
        const authorizedBodyHash = await normalizeBodyHash(authorized.format, authorized.body)
        if (authorized.recordVersion === pending.expectedRecordVersion) {
          // The pending body is a proposed update, so it is expected to differ
          // from the canonical body while the old API request is unresolved.
          // Compare against the room's accepted baseline before replacing the
          // marker with the new operation.
          if (authorizedBodyHash !== metadata.bodyHash) {
            await this.deletePendingCheckpoint()
            this.broadcast({ type: 'live-conflict', operation_id: operationId })
            return
          }
          await this.deletePendingCheckpoint()
          metadata = {
            ...metadata,
            recordVersion: authorized.recordVersion,
            bodyHash: authorizedBodyHash,
          }
          await this.ctx.storage.put(LIVE_ROOM_META_KEY, metadata)
          pending = null
        } else if (isRecordVersionNewer(authorized.recordVersion, pending.expectedRecordVersion)) {
          if (authorizedBodyHash !== pending.bodyHash) {
            await this.deletePendingCheckpoint()
            this.broadcast({ type: 'live-conflict', operation_id: operationId })
            return
          }
          metadata = {
            ...metadata,
            recordVersion: authorized.recordVersion,
            bodyHash: authorizedBodyHash,
          }
          await this.ctx.storage.put(LIVE_ROOM_META_KEY, metadata)
          await this.deletePendingCheckpoint()
          pending = null

          const authorizedRequestBodyHash = await normalizeBodyHash(metadata.format, body)
          if (version === metadata.version && authorizedRequestBodyHash === authorizedBodyHash) {
            // The old operation already advanced the canonical record. Treat
            // an equivalent new request as an idempotent saved acknowledgement
            // and persist that alias for another reconnect.
            await this.saveCheckpointResult({
              version: metadata.version,
              operationId,
              recordVersion: authorized.recordVersion,
              bodyHash: authorizedBodyHash,
              fingerprint,
            })
            this.broadcast({
              type: 'live-saved',
              version: metadata.version,
              record_version: authorized.recordVersion,
              operation_id: operationId,
            })
            return
          }
        } else {
          await this.deletePendingCheckpoint()
          this.broadcast({ type: 'live-conflict', operation_id: operationId })
          return
        }
      }

      let checkpointMetadata = metadata
      let checkpointBody = body
      if (pending) {
        // A pending checkpoint gates all other checkpoints until its exact
        // operation is retried. This protects the API CAS boundary after a
        // transient network failure or a DO eviction.
        if (
          pending.operationId !== operationId ||
          pending.fingerprint !== fingerprint
        ) {
          sendSocketError(sender, 'Another checkpoint is pending', operationId)
          return
        }
        const pendingBody = await this.readPendingCheckpointBody(pending)
        if (pendingBody === null || await fingerprintBody(pendingBody) !== pending.fingerprint) {
          sendSocketError(sender, 'Live checkpoint recovery failed', operationId)
          return
        }
        checkpointBody = pendingBody
        checkpointMetadata = {
          ...metadata,
          version: pending.version,
          recordVersion: pending.expectedRecordVersion,
        }
      } else {
        if (!metadata.recordVersion) {
          sendSocketError(sender, 'Live document is not initialized', operationId)
          return
        }
        const expectedRecordVersion = metadata.recordVersion
        const authorized = refreshedAuthorization ?? await this.authorizeCurrentSession(session)
        if (!authorized) {
          sendSocketError(sender, 'Live checkpoint recovery failed', operationId)
          return
        }
        const authorizedBodyHash = await normalizeBodyHash(authorized.format, authorized.body)
        if (authorizedBodyHash !== metadata.bodyHash) {
          this.broadcast({ type: 'live-conflict', operation_id: operationId })
          return
        }
        if (isRecordVersionNewer(authorized.recordVersion, metadata.recordVersion)) {
          metadata = {
            ...metadata,
            recordVersion: authorized.recordVersion,
          }
          await this.ctx.storage.put(LIVE_ROOM_META_KEY, metadata)
        }
        checkpointMetadata = metadata
        // Only the current working version may cross the CAS boundary. A
        // stale checkpoint is retained by the client and never sent to API.
        if (version !== metadata.version) {
          sendSocketError(sender, 'Checkpoint is behind the working version', operationId)
          return
        }
        const bytes = new TextEncoder().encode(body)
        const pendingCheckpoint: LivePendingCheckpoint = {
          version,
          operationId,
          expectedRecordVersion,
          bodyHash: await normalizeBodyHash(metadata.format, body),
          fingerprint,
          bodyByteLength: bytes.byteLength,
          chunkCount: Math.ceil(bytes.byteLength / CHECKPOINT_BODY_CHUNK_BYTES),
        }
        await this.savePendingCheckpoint(pendingCheckpoint, body)
      }

      const result = await this.persistCheckpoint(session, checkpointMetadata, checkpointBody, operationId)
      if (result.kind === 'conflict') {
        await this.deletePendingCheckpoint()
        this.broadcast({ type: 'live-conflict', operation_id: operationId })
        return
      }
      if (result.kind === 'error') {
        sendSocketError(sender, 'Live checkpoint failed', operationId)
        return
      }

      const checkpointBodyHash = await normalizeBodyHash(checkpointMetadata.format, checkpointBody)
      await this.saveCheckpointResult({
        version: checkpointMetadata.version,
        operationId,
        recordVersion: result.recordVersion,
        bodyHash: checkpointBodyHash,
        fingerprint,
      })
      const saved: LiveRoomMetadata = {
        ...metadata,
        recordVersion: result.recordVersion,
        bodyHash: checkpointBodyHash,
      }
      await this.ctx.storage.put(LIVE_ROOM_META_KEY, saved)
      await this.deletePendingCheckpoint()
      this.broadcast({
        type: 'live-saved',
        version: saved.version,
        record_version: saved.recordVersion,
        operation_id: operationId,
      })
    })
  }

  private async persistCheckpoint(
    session: LiveTicketSession,
    metadata: LiveRoomMetadata,
    body: string,
    operationId: string,
  ): Promise<CheckpointResult> {
    const endpoint = routeLiveApiUrl(this.liveEnv, LIVE_CHECKPOINT_SUFFIX, session)
    const requestId = `live_checkpoint_${crypto.randomUUID()}`
    let response: Response
    try {
      response = await fetch(endpoint, {
        method: 'POST',
        headers: requestHeadersForApi(session, requestId),
        body: JSON.stringify({
          property_id: metadata.property,
          operation_id: operationId,
          expected_record_version: metadata.recordVersion,
          format: metadata.format,
          body,
        }),
        signal: AbortSignal.timeout(15_000),
      })
    } catch {
      return { kind: 'error' }
    }

    if (response.status === 409) return { kind: 'conflict' }
    if (!response.ok) return { kind: 'error' }

    try {
      const responseBytes = await response.arrayBuffer()
      if (responseBytes.byteLength > MAX_API_RESPONSE_BYTES) return { kind: 'error' }
      const payload = JSON.parse(new TextDecoder().decode(responseBytes)) as unknown
      if (!isRecord(payload)) return { kind: 'error' }
      const recordVersion = canonicalField(payload, 'record_version', 'recordVersion')
      return recordVersion ? { kind: 'saved', recordVersion } : { kind: 'error' }
    } catch {
      return { kind: 'error' }
    }
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url)

    if (url.pathname === LIVE_SESSION_PATH || url.pathname === LIVE_WEBSOCKET_PATH) {
      const denied = liveAccessFailure(request, env)
      if (denied) return denied

      if (request.method === 'OPTIONS') {
        return new Response(null, { status: 204, headers: liveCorsHeaders(request, env) })
      }
      if (url.pathname === LIVE_SESSION_PATH) {
        if (request.method !== 'POST') {
          return liveJsonResponse(request, env, { error: 'Method not allowed' }, { status: 405 })
        }
        return createLiveSession(request, env)
      }
      return openLiveWebSocket(request, env)
    }

    // The imported upstream handler continues to own `/ws` and all Engine
    // proxy/debug routes. It only receives the old PHOTON_SYNC_ROOMS binding.
    return photonWorkerDefault.fetch(request, env)
  },
}
