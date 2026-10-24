import { configuredLibraryApiBaseUrl, libraryGraphqlHeaders } from './libraryGraphql'

export interface ProjectionRef {
  provider: string
  workspace_id: string
  kind: 'initiative' | 'project'
  external_id: string
}
export interface PmProjection {
  resource_ref: ProjectionRef
  name: string
  url: string | null
  archived: boolean
  status: string | null
  owner: { external_id: string; name: string } | null
  target_date: string | null
  source_updated_at: string | null
}
export interface ProjectionRecord {
  id: string
  resource_ref: ProjectionRef
  projection: PmProjection
  observed_at: string
  observed_revision: number
  lifecycle: string
  note: string
  document_ids: string[]
}
export interface ProjectionRelation {
  source: ProjectionRef
  target: ProjectionRef
  removed: boolean
}
export interface ProjectionPage {
  records: ProjectionRecord[]
  relations: ProjectionRelation[]
  pending_relations: number
  sync_pending: boolean
  next_after: string | null
}
export interface ProjectionTarget {
  organization: string
  repository: string
  operatorId?: string
}
export type ProjectionErrorKind = 'denied' | 'unavailable' | 'not-connected' | 'conflict' | 'invalid'
export class ProjectionApiError extends Error {
  readonly kind: ProjectionErrorKind
  readonly status: number
  constructor(kind: ProjectionErrorKind, status = 0) {
    super(kind)
    this.kind = kind
    this.status = status
    this.name = 'ProjectionApiError'
  }
}
/** No Photon, query cache, browser storage, or shared/public read path. */
async function request<T>(target: ProjectionTarget, tail: string, signal: AbortSignal, body?: unknown): Promise<T> {
  const base = configuredLibraryApiBaseUrl()
  const path = [target.organization, target.repository].map(encodeURIComponent).join('/')
  const response = await fetch(`${base}/v1beta/repos/${path}/pm-projection${tail}`, {
    method: body === undefined ? 'GET' : 'POST',
    headers: await libraryGraphqlHeaders(target.operatorId),
    signal,
    cache: 'no-store',
    redirect: 'error',
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  })
  if (!response.ok) {
    const kind: ProjectionErrorKind = response.status === 401 || response.status === 403 ? 'denied'
      : response.status === 404 ? 'not-connected'
        : response.status === 409 ? 'conflict'
          : response.status === 400 ? 'invalid' : 'unavailable'
    throw new ProjectionApiError(kind, response.status)
  }
  return response.json() as Promise<T>
}
function validRecord(value: unknown): value is ProjectionRecord {
  if (!value || typeof value !== 'object') return false
  const record = value as ProjectionRecord
  return typeof record.id === 'string' && typeof record.note === 'string'
    && Array.isArray(record.document_ids) && record.document_ids.every(id => typeof id === 'string')
    && typeof record.observed_at === 'string' && typeof record.observed_revision === 'number'
    && !!record.resource_ref && ['initiative', 'project'].includes(record.resource_ref.kind)
    && !!record.projection && typeof record.projection.name === 'string'
    && typeof record.projection.archived === 'boolean'
    && !!record.projection.resource_ref && projectionIdentity(record.resource_ref) === projectionIdentity(record.projection.resource_ref)
}
export async function fetchProjectionPage(target: ProjectionTarget, signal: AbortSignal, after?: string): Promise<ProjectionPage> {
  const query = new URLSearchParams({ limit: '50', ...(after ? { after } : {}) })
  const value = await request<ProjectionPage>(target, `?${query}`, signal)
  if (!Array.isArray(value.records) || !value.records.every(validRecord) || !Array.isArray(value.relations) || typeof value.sync_pending !== 'boolean'
    || typeof value.pending_relations !== 'number' || !(value.next_after === null || typeof value.next_after === 'string')) {
    throw new ProjectionApiError('unavailable')
  }
  return value
}
export async function fetchProjectionRecord(target: ProjectionTarget, id: string, signal: AbortSignal): Promise<ProjectionRecord> {
  const value = await request<ProjectionRecord>(target, `/records/${encodeURIComponent(id)}`, signal)
  if (!validRecord(value) || value.id !== id) throw new ProjectionApiError('unavailable')
  return value
}
export function saveProjectionContext(target: ProjectionTarget, id: string, note: string, documentIds: string[], signal: AbortSignal): Promise<{ saved: boolean }> {
  return request(target, `/${encodeURIComponent(id)}/context`, signal, { note, document_ids: documentIds })
}
export function projectionIdentity(reference: ProjectionRef): string {
  return JSON.stringify([reference.provider, reference.workspace_id, reference.kind, reference.external_id])
}
export function safeProjectionUrl(value: string | null): string | undefined {
  if (!value) return undefined
  try {
    const url = new URL(value)
    return url.protocol === 'https:' ? url.href : undefined
  } catch { return undefined }
}

export function documentIdsFromText(text: string, target: ProjectionTarget): string[] {
  const ids = text.split(/[\n,]/).map(value => value.trim()).filter(Boolean).map(value => {
    if (!value.includes('/') && !value.includes(':')) return value
    let url: URL
    try { url = new URL(value, 'https://library.invalid') } catch { throw new ProjectionApiError('invalid') }
    let parts: string[]
    try { parts = url.pathname.split('/').filter(Boolean).map(decodeURIComponent) } catch { throw new ProjectionApiError('invalid') }
    if (!['http:', 'https:'].includes(url.protocol) || parts.length !== 4 || parts[0] !== target.organization || parts[1] !== target.repository || parts[2] !== 'data') throw new ProjectionApiError('invalid')
    return parts[3]
  })
  if (ids.length > 100) throw new ProjectionApiError('invalid')
  return [...new Set(ids)]
}
