import {
  RecordApiError,
  shouldFallbackLibraryRequest,
  type LibraryDataItem,
  type LibraryProperty,
  type LibraryPropertyDataValue,
} from '../recordsApi'
import { getValidAuthTokens, loadStoredAuthIdentity } from '../auth'
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

async function libraryRestHeaders(operatorId?: string): Promise<Record<string, string>> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = (await getValidAuthTokens())?.accessToken
  if (token) headers.Authorization = `Bearer ${token}`
  return headers
}

async function requestLibraryGraphQL<TData>(
  query: string,
  variables: Record<string, unknown>,
  options?: { operatorId?: string }
): Promise<TData> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': options?.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = (await getValidAuthTokens())?.accessToken
  if (token) headers.Authorization = `Bearer ${token}`

  let response: Response
  try {
    response = await fetch(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ query, variables }),
    })
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
  return propertyData.filter((entry) => propertyIds.has(entry.propertyId))
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

export async function updateLibraryData(
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  item: LibraryDataItem
): Promise<LibraryDataItem> {
  const propertyData = knownPropertyData(properties, item.propertyData, 'update')
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
      { operatorId: target.operatorId }
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
    if (!shouldFallbackLibraryRequest(error, 'update')) throw error
  }

  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${item.id}`,
    {
      method: 'PUT',
      headers: await libraryRestHeaders(target.operatorId),
      body: JSON.stringify({
        name: item.name,
        property_data: restPropertyPayload(properties, propertyData),
      }),
    }
  )
  if (!response.ok) {
    throw new RecordApiError(`Library REST data update failed: ${response.status}`, response.status)
  }
  const payload = await response.json() as LibraryRestDataResponse
  return restResponseToLibraryDataItem(payload)
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
