import { apiBaseUrl } from './config'

export type HttpMethod = 'get' | 'post' | 'put' | 'delete' | 'patch'

const METHOD_ORDER: HttpMethod[] = ['get', 'post', 'put', 'patch', 'delete']

export interface OperationParameter {
  name: string
  location: string
  required: boolean
  description?: string
}

export interface Operation {
  method: HttpMethod
  path: string
  summary?: string
  description?: string
  parameters: OperationParameter[]
  requestBody: boolean
  deprecated: boolean
}

export interface OperationGroup {
  /** Path prefix the operations share, used as the section heading. */
  name: string
  operations: Operation[]
}

export interface ApiDocument {
  title: string
  version: string
  groups: OperationGroup[]
  operationCount: number
}

interface RawParameter {
  name?: unknown
  in?: unknown
  required?: unknown
  description?: unknown
}

interface RawOperation {
  summary?: unknown
  description?: unknown
  parameters?: unknown
  requestBody?: unknown
  deprecated?: unknown
}

/**
 * Read the OpenAPI document the API publishes about itself.
 *
 * The endpoint list is generated from this rather than written by hand, so
 * a route added to the API shows up here without anyone remembering to
 * update prose.
 */
export async function fetchApiDocument(
  signal?: AbortSignal,
): Promise<ApiDocument> {
  const response = await fetch(`${apiBaseUrl}/v1beta/api-docs/openapi.json`, {
    signal,
  })
  if (!response.ok) {
    throw new Error(
      `OpenAPI ドキュメントの取得に失敗しました (HTTP ${response.status})`,
    )
  }
  return parseApiDocument(await response.json())
}

export function parseApiDocument(raw: unknown): ApiDocument {
  const doc = raw as {
    info?: { title?: unknown; version?: unknown }
    paths?: Record<string, Record<string, RawOperation>>
  }

  const operations: Operation[] = []
  for (const [path, methods] of Object.entries(doc.paths ?? {})) {
    for (const [method, operation] of Object.entries(methods)) {
      const lower = method.toLowerCase() as HttpMethod
      if (!METHOD_ORDER.includes(lower)) continue
      operations.push({
        method: lower,
        path,
        summary: asString(operation.summary),
        description: asString(operation.description),
        parameters: parseParameters(operation.parameters),
        requestBody: operation.requestBody != null,
        deprecated: operation.deprecated === true,
      })
    }
  }

  return {
    title: asString(doc.info?.title) ?? 'library-api',
    version: asString(doc.info?.version) ?? 'unknown',
    groups: groupOperations(operations),
    operationCount: operations.length,
  }
}

function parseParameters(raw: unknown): OperationParameter[] {
  if (!Array.isArray(raw)) return []
  return raw
    .map(entry => entry as RawParameter)
    .filter(entry => typeof entry.name === 'string')
    .map(entry => ({
      name: entry.name as string,
      location: asString(entry.in) ?? 'query',
      required: entry.required === true,
      description: asString(entry.description),
    }))
}

/**
 * Group by the segment that identifies the resource, which for this API is
 * the one after `/repos/{org}/{repo}` where there is one and the leading
 * segment otherwise. Grouping by tag is not an option: most operations
 * carry none.
 */
function groupOperations(operations: Operation[]): OperationGroup[] {
  const groups = new Map<string, Operation[]>()

  for (const operation of operations) {
    const name = groupNameFor(operation.path)
    const bucket = groups.get(name)
    if (bucket) {
      bucket.push(operation)
    } else {
      groups.set(name, [operation])
    }
  }

  return [...groups.entries()]
    .map(([name, ops]) => ({
      name,
      operations: ops.sort(compareOperations),
    }))
    .sort((a, b) => a.name.localeCompare(b.name))
}

/**
 * Segments that name an action rather than a resource, and the resource
 * each belongs beside. Without this `data-list` and `change-username` each
 * become a section of one, away from the operations they belong with.
 */
const GROUP_ALIASES: Record<string, string> = {
  'data-list': 'data',
  'change-username': 'repos',
}

function groupNameFor(path: string): string {
  const segments = path.split('/').filter(Boolean)
  const repoIndex = segments.indexOf('repos')

  // /v1beta/repos/{org}/{repo}/data/... → "data"; the repo itself → "repos"
  if (repoIndex >= 0) {
    const afterRepo = segments[repoIndex + 3]
    const name =
      afterRepo && !afterRepo.startsWith('{') ? afterRepo : 'repos'
    return GROUP_ALIASES[name] ?? name
  }

  const first = segments[0]
  if (first === 'v1beta') return segments[1] ?? 'v1beta'
  return first ?? '/'
}

function compareOperations(a: Operation, b: Operation): number {
  if (a.path !== b.path) return a.path.localeCompare(b.path)
  return METHOD_ORDER.indexOf(a.method) - METHOD_ORDER.indexOf(b.method)
}

function asString(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}
