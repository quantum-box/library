import {
  RecordApiError,
  shouldFallbackLibraryRequest,
  type LibraryDataItem,
  type LibraryProperty,
  type LibraryPropertyDataValue,
} from '../recordsApi'
import { getValidAuthTokens, loadStoredAuthIdentity, unexpiredAuthTokens } from '../auth'
import {
  libraryPropertyValueToGraphqlInput,
} from './libraryPropertyInput'

export interface LibraryRepoTarget {
  org: string
  repo: string
  operatorId?: string
  repoName?: string
}

interface LibraryAddDataResponse {
  addData?: LibraryDataItem | null
}

interface LibraryUpdateDataResponse {
  updateData?: LibraryDataItem | null
}

interface LibraryDeleteDataResponse {
  deleteData?: string | null
}

interface LibraryRestDataResponse {
  id: string
  name: string
  record_version?: string
  items: Array<{
    property_id: string
    key: string
    value?: Record<string, unknown> | string | number | string[] | null
  }>
}

const libraryAddDataMutation = `
  mutation LibraryClientAddData($input: AddDataInputData!) {
    addData(input: $input) {
      id
      name
      createdAt
      updatedAt
      propertyData {
        propertyId
        value {
          ... on StringValue { string }
          ... on IntegerValue { number }
          ... on HtmlValue { html }
          ... on MarkdownValue { markdown }
          ... on RichTextValue { richText }
          ... on DateValue { date }
          ... on ImageValue { url }
          ... on BooleanValue { boolean }
          ... on IdValue { id }
          ... on RelationValue { dataIds databaseId }
          ... on SelectValue { optionId }
          ... on MultiSelectValue { optionIds }
          ... on LocationValue { latitude longitude }
        }
      }
    }
  }
`

const libraryUpdateDataMutation = `
  mutation LibraryClientUpdateData($input: UpdateDataInputData!) {
    updateData(input: $input) {
      id
      name
      recordVersion
      createdAt
      updatedAt
      propertyData {
        propertyId
        value {
          ... on StringValue { string }
          ... on IntegerValue { number }
          ... on HtmlValue { html }
          ... on MarkdownValue { markdown }
          ... on RichTextValue { richText }
          ... on DateValue { date }
          ... on ImageValue { url }
          ... on BooleanValue { boolean }
          ... on IdValue { id }
          ... on RelationValue { dataIds databaseId }
          ... on SelectValue { optionId }
          ... on MultiSelectValue { optionIds }
          ... on LocationValue { latitude longitude }
        }
      }
    }
  }
`

const libraryDeleteDataMutation = `
  mutation LibraryClientDeleteData($org: String!, $repo: String!, $dataId: String!) {
    deleteData(orgUsername: $org, repoUsername: $repo, dataId: $dataId)
  }
`

function configuredLibraryApiBaseUrl(): string {
  return (
    import.meta.env.VITE_LIBRARY_API_BASE_URL ??
    import.meta.env.VITE_BACKEND_API_URL ??
    'http://localhost:50053'
  ).replace(/\/+$/, '')
}

function configuredPlatformId(): string {
  return import.meta.env.VITE_LIBRARY_PLATFORM_ID ?? import.meta.env.VITE_PLATFORM_ID ?? 'tn_01j702qf86pc2j35s0kv0gv3gy'
}

function configuredLibraryActor(): string {
  const actorId = loadStoredAuthIdentity()?.userId
  if (actorId) return actorId
  return (
    import.meta.env.VITE_LIBRARY_ACTOR_ID ??
    import.meta.env.VITE_LIBRARY_OPERATOR_ID ??
    configuredPlatformId()
  )
}

/**
 * The access token for a request. One that must outlive the page starts
 * with the token it has while that still works, instead of waiting on a
 * refresh the page may not live through.
 */
async function libraryAccessToken(keepalive?: boolean): Promise<string | undefined> {
  if (keepalive) {
    const current = unexpiredAuthTokens()
    if (current) return current.accessToken
  }
  return (await getValidAuthTokens())?.accessToken
}

async function libraryRestHeaders(
  operatorId?: string,
  keepalive?: boolean,
): Promise<Record<string, string>> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = await libraryAccessToken(keepalive)
  if (token) headers.Authorization = `Bearer ${token}`
  return headers
}

/**
 * Browsers refuse a keepalive request whose body exceeds 64 KiB. Past this,
 * the request goes out as an ordinary one, which an unloading page may cut.
 */
const KEEPALIVE_BODY_LIMIT = 60 * 1024

function keepaliveFor(body: string, keepalive: boolean | undefined): boolean {
  return Boolean(keepalive) && new TextEncoder().encode(body).byteLength <= KEEPALIVE_BODY_LIMIT
}

/**
 * `fetch`, in a request that outlives the page when asked for and allowed.
 *
 * The 64 KiB is a quota over all of a page's keepalive requests in flight,
 * so one can also be refused for the others still on their way. That says
 * nothing about this request, and the page is evidently still here: it is
 * sent again as an ordinary one.
 */
async function fetchOutlivingPage(
  url: string,
  init: RequestInit & { body: string },
  keepalive: boolean | undefined,
): Promise<Response> {
  if (!keepaliveFor(init.body, keepalive)) return fetch(url, init)
  try {
    return await fetch(url, { ...init, keepalive: true })
  } catch {
    return fetch(url, init)
  }
}

async function requestLibraryGraphQL<TData>(
  query: string,
  variables: Record<string, unknown>,
  options?: { operatorId?: string; keepalive?: boolean }
): Promise<TData> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': options?.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = await libraryAccessToken(options?.keepalive)
  if (token) headers.Authorization = `Bearer ${token}`

  let response: Response
  const body = JSON.stringify({ query, variables })
  try {
    response = await fetchOutlivingPage(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
      method: 'POST',
      headers,
      body,
    }, options?.keepalive)
  } catch (error: unknown) {
    const detail = error instanceof Error ? `: ${error.message}` : ''
    throw new RecordApiError(
      `Library GraphQL transport unavailable${detail}`,
      0,
      'transport'
    )
  }
  if (!response.ok) {
    throw new RecordApiError(
      `Library GraphQL request failed: ${response.status}`,
      response.status,
      [404, 405, 501].includes(response.status) ? 'endpoint-unavailable' : 'http'
    )
  }
  let payload: { data?: TData; errors?: Array<{ message?: string }> }
  try {
    payload = await response.json() as typeof payload
  } catch {
    throw new RecordApiError(
      'Library GraphQL returned an invalid JSON response',
      response.status,
      'invalid-response'
    )
  }
  if (payload.errors?.length) {
    throw new RecordApiError(
      payload.errors[0]?.message ?? 'Library GraphQL request failed',
      400,
      'graphql'
    )
  }
  if (payload.data == null) {
    throw new RecordApiError(
      'Library GraphQL returned no data',
      response.status,
      'invalid-response'
    )
  }
  return payload.data
}

function restValueToLibraryPropertyDataValue(
  value: LibraryRestDataResponse['items'][number]['value']
): LibraryPropertyDataValue {
  if (value == null) return {}
  if (typeof value === 'string') return { string: value }
  if (typeof value === 'number') return { number: String(value) }
  if (typeof value === 'boolean') return { boolean: value }
  if (Array.isArray(value)) return { optionIds: value.map(String) }
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>
    if (typeof record.boolean === 'boolean') return { boolean: record.boolean }
    if (typeof record.string === 'string') return { string: record.string }
    if (typeof record.integer === 'number' || typeof record.integer === 'string') {
      return { number: String(record.integer) }
    }
    if (typeof record.number === 'number' || typeof record.number === 'string') {
      return { number: String(record.number) }
    }
    if (typeof record.html === 'string') return { html: record.html }
    if (typeof record.markdown === 'string') return { markdown: record.markdown }
    if (typeof record.date === 'string') return { date: record.date }
    if (typeof record.image === 'string') return { url: record.image }
    if (typeof record.url === 'string') return { url: record.url }
    if (typeof record.id === 'string') return { id: record.id }
    if (typeof record.select === 'string') return { optionId: record.select }
    if (typeof record.optionId === 'string') return { optionId: record.optionId }
    if (typeof record.option_id === 'string') return { optionId: record.option_id }
    if (Array.isArray(record.multiSelect)) return { optionIds: record.multiSelect.map(String) }
    if (Array.isArray(record.optionIds)) return { optionIds: record.optionIds.map(String) }
    if (Array.isArray(record.option_ids)) return { optionIds: record.option_ids.map(String) }
    if (record.relation && typeof record.relation === 'object') {
      const relation = record.relation as Record<string, unknown>
      return {
        dataIds: Array.isArray(relation.data_id) ? relation.data_id.map(String) : [],
        databaseId: typeof relation.database_id === 'string' ? relation.database_id : undefined,
      }
    }
    if (record.location && typeof record.location === 'object') {
      const location = record.location as Record<string, unknown>
      if (typeof location.latitude === 'number' && typeof location.longitude === 'number') {
        return { latitude: location.latitude, longitude: location.longitude }
      }
    }
    const candidate = Object.values(record)[0]
    if (typeof candidate === 'string') return { string: candidate }
    if (typeof candidate === 'number') return { number: String(candidate) }
  }
  return {}
}

function restResponseToLibraryDataItem(payload: LibraryRestDataResponse): LibraryDataItem {
  return {
    id: payload.id,
    name: payload.name,
    ...(payload.record_version ? { recordVersion: payload.record_version } : {}),
    propertyData: payload.items.map((entry) => ({
      propertyId: entry.property_id,
      value: restValueToLibraryPropertyDataValue(entry.value),
    })),
  }
}

function knownPropertyData(
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData'],
  operation: 'create' | 'update'
): LibraryDataItem['propertyData'] {
  const propertyIds = new Set(properties.map((property) => property.id))
  const unknown = propertyData.find((entry) => !propertyIds.has(entry.propertyId))
  if (unknown && operation === 'create') {
    throw new RecordApiError(
      `Cannot create data with unknown Property ${unknown.propertyId}`,
      422,
      'mapping'
    )
  }
  // UpdateData is a patch boundary. Omitting a Property definition that this
  // client cannot interpret preserves its existing server value.
  return propertyData
    .filter((entry) => propertyIds.has(entry.propertyId))
    .filter((entry) => !isPreviewOnlyValue(entry.value))
}

/**
 * Whether a value is a listing's preview of a body rather than the body.
 *
 * Rows come out of a listing holding `preview` and no `richText`, and a
 * write built from one would send the preview as the new document. Since
 * update is a patch, dropping the entry is what leaves the stored document
 * alone -- so editing a Select cell on a row cannot truncate its body.
 */
function isPreviewOnlyValue(value: LibraryPropertyDataValue): boolean {
  return value.preview !== undefined && value.richText === undefined
}

function graphqlPropertyPayload(
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
) {
  const propertyById = new Map(properties.map((property) => [property.id, property]))
  return propertyData.flatMap((entry) => {
    const property = propertyById.get(entry.propertyId)
    if (!property) return []
    let value: Record<string, unknown> | null
    if (property.typ === 'Select' && entry.value.optionId !== undefined) {
      value = { select: entry.value.optionId }
    } else if (property.typ === 'MultiSelect' && entry.value.optionIds !== undefined) {
      value = { multiSelect: entry.value.optionIds }
    } else if (property.typ === 'Relation' && entry.value.dataIds !== undefined) {
      value = { relation: entry.value.dataIds }
    } else if (property.typ === 'Date' && entry.value.date !== undefined) {
      value = { date: entry.value.date }
    } else if (property.typ === 'Image' && entry.value.url !== undefined) {
      value = { image: entry.value.url }
    } else if (property.typ === 'Boolean' && entry.value.boolean !== undefined) {
      value = { boolean: entry.value.boolean }
    } else {
      value = libraryPropertyValueToGraphqlInput(property, entry.value)
    }
    return value ? [{ propertyId: entry.propertyId, value }] : []
  })
}

function restPropertyValue(property: LibraryProperty, value: LibraryPropertyDataValue): unknown {
  switch (property.typ) {
    case 'String':
      return value.string ?? ''
    case 'Id':
      return value.id ?? ''
    case 'Integer': {
      const parsed = Number(value.number)
      if (!Number.isInteger(parsed) || parsed < -2_147_483_648 || parsed > 2_147_483_647) {
        throw new RecordApiError(
          `REST fallback cannot safely encode Integer Property "${property.name}"`,
          422,
          'mapping'
        )
      }
      return parsed
    }
    case 'Html':
      return { html: value.html ?? '' }
    case 'Markdown':
      return { markdown: value.markdown ?? '' }
    case 'RichText':
      // The tagged object is load-bearing: a bare string would reach the
      // API's String input arm and be rejected against a RichText property.
      return { richText: value.richText ?? '' }
    case 'MultiSelect':
      return value.optionIds ?? []
    case 'Boolean':
      // A bare JSON boolean; the API reads it as a Boolean command.
      return value.boolean ?? false
    default:
      throw new RecordApiError(
        `REST fallback cannot safely encode ${property.typ} Property "${property.name}"`,
        422,
        'mapping'
      )
  }
}

function restPropertyPayload(
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
) {
  const propertyById = new Map(properties.map((property) => [property.id, property]))
  return propertyData.flatMap((entry) => {
    const property = propertyById.get(entry.propertyId)
    if (!property) return []
    return [{ property_id: entry.propertyId, value: restPropertyValue(property, entry.value) }]
  })
}

export async function addLibraryData(
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  input: { name: string; propertyData?: LibraryDataItem['propertyData'] }
): Promise<LibraryDataItem> {
  const propertyData = knownPropertyData(properties, input.propertyData ?? [], 'create')
  try {
    const payload = await requestLibraryGraphQL<LibraryAddDataResponse>(
      libraryAddDataMutation,
      {
        input: {
          actor: configuredLibraryActor(),
          orgUsername: target.org,
          repoUsername: target.repo,
          dataName: input.name,
          propertyData: graphqlPropertyPayload(properties, propertyData),
        },
      },
      { operatorId: target.operatorId }
    )
    if (!payload.addData) {
      throw new RecordApiError(
        'Library API did not return created data',
        500,
        'invalid-response'
      )
    }
    return payload.addData
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'create')) throw error
  }

  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data`,
    {
      method: 'POST',
      headers: await libraryRestHeaders(target.operatorId),
      body: JSON.stringify({
        name: input.name,
        property_data: restPropertyPayload(properties, propertyData),
      }),
    }
  )
  if (!response.ok) {
    throw new RecordApiError(`Library REST data create failed: ${response.status}`, response.status)
  }
  const payload = await response.json() as LibraryRestDataResponse
  return restResponseToLibraryDataItem(payload)
}

/**
 * Set once this API has answered that it has no GraphQL update. A save that
 * must outlive the page then goes straight to REST: the page may not live to
 * run the fallback after the GraphQL answer.
 */
let graphqlUpdateUnavailable = false

export async function updateLibraryData(
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  item: LibraryDataItem,
  /** `keepalive`: the page is unloading; let the request outlive it. */
  options?: { keepalive?: boolean }
): Promise<LibraryDataItem> {
  const propertyData = knownPropertyData(properties, item.propertyData, 'update')
  if (!(options?.keepalive && graphqlUpdateUnavailable)) {
    try {
      const payload = await requestLibraryGraphQL<LibraryUpdateDataResponse>(
        libraryUpdateDataMutation,
        {
          input: {
            actor: configuredLibraryActor(),
            orgUsername: target.org,
            repoUsername: target.repo,
            dataId: item.id,
            dataName: item.name,
            propertyData: graphqlPropertyPayload(properties, propertyData),
          },
        },
        { operatorId: target.operatorId, keepalive: options?.keepalive }
      )
      if (!payload.updateData) {
        throw new RecordApiError(
          'Library API did not return updated data',
          500,
          'invalid-response'
        )
      }
      return payload.updateData
    } catch (error: unknown) {
      if (error instanceof RecordApiError && error.kind === 'endpoint-unavailable') {
        graphqlUpdateUnavailable = true
      }
      if (!shouldFallbackLibraryRequest(error, 'update')) throw error
    }
  }

  const restBody = JSON.stringify({
    name: item.name,
    property_data: restPropertyPayload(properties, propertyData),
  })
  const response = await fetchOutlivingPage(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${item.id}`,
    {
      method: 'PUT',
      headers: await libraryRestHeaders(target.operatorId, options?.keepalive),
      body: restBody,
    },
    options?.keepalive,
  )
  if (!response.ok) {
    throw new RecordApiError(`Library REST data update failed: ${response.status}`, response.status)
  }
  const payload = await response.json() as LibraryRestDataResponse
  return restResponseToLibraryDataItem(payload)
}

/**
 * Save a Live body only if the record is still at `expectedRecordVersion`,
 * in a request that outlives the page.
 *
 * For a page that goes away while its room is out of reach: the body is
 * written through the same version-checked checkpoint a room uses, so it
 * never replaces anything saved since that version -- if the record moved
 * on, nothing is written. Resolves with the record version it produced
 * when it was accepted, `null` otherwise.
 */
export async function checkpointLiveBodyOutlivingPage(
  target: { org: string; repo: string; dataId: string; operatorId?: string },
  checkpoint: {
    propertyId: string
    expectedRecordVersion: string
    format: 'markdown' | 'richText'
    body: string
  },
): Promise<string | null> {
  const body = JSON.stringify({
    property_id: checkpoint.propertyId,
    operation_id: globalThis.crypto?.randomUUID?.() ??
      `live-page-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    expected_record_version: checkpoint.expectedRecordVersion,
    format: checkpoint.format,
    body: checkpoint.body,
  })
  try {
    const response = await fetchOutlivingPage(
      `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${target.dataId}/live/checkpoint`,
      { method: 'POST', headers: await libraryRestHeaders(target.operatorId, true), body },
      true,
    )
    if (!response.ok) return null
    const payload = await response.json().catch(() => null) as { record_version?: unknown } | null
    return typeof payload?.record_version === 'string' ? payload.record_version : ''
  } catch {
    return null
  }
}

export async function deleteLibraryData(
  target: LibraryRepoTarget,
  dataId: string
): Promise<void> {
  try {
    const payload = await requestLibraryGraphQL<LibraryDeleteDataResponse>(
      libraryDeleteDataMutation,
      { org: target.org, repo: target.repo, dataId },
      { operatorId: target.operatorId }
    )
    if (!payload.deleteData) {
      throw new RecordApiError(
        'Library API did not delete data',
        500,
        'invalid-response'
      )
    }
    return
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'delete')) throw error
  }

  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${dataId}`,
    {
      method: 'DELETE',
      headers: await libraryRestHeaders(target.operatorId),
    }
  )
  if (!response.ok && response.status !== 404) {
    throw new RecordApiError(`Library REST data delete failed: ${response.status}`, response.status)
  }
}
