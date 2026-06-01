import type { LibraryDataItem, LibraryProperty } from '../recordsApi'

class LibraryApiError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'LibraryApiError'
    this.status = status
  }
}
import {
  libraryDataItemToGraphqlPropertyData,
  libraryPropertyValueToRestValue,
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
          ... on DateValue { date }
          ... on ImageValue { url }
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
          ... on DateValue { date }
          ... on ImageValue { url }
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

function libraryAccessToken(): string | null {
  try {
    const stored = localStorage.getItem('library_auth')
    if (!stored) return null
    const parsed = JSON.parse(stored) as { accessToken?: string }
    return parsed.accessToken ?? null
  } catch {
    return null
  }
}

function configuredLibraryActor(): string {
  try {
    const stored = localStorage.getItem('library_auth')
    if (stored) {
      const parsed = JSON.parse(stored) as { userId?: string }
      if (parsed.userId) return parsed.userId
    }
  } catch {
    // Fall back to operator/platform identifiers.
  }
  return (
    import.meta.env.VITE_LIBRARY_ACTOR_ID ??
    import.meta.env.VITE_LIBRARY_OPERATOR_ID ??
    configuredPlatformId()
  )
}

function libraryRestHeaders(operatorId?: string): Record<string, string> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = libraryAccessToken()
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
  const token = libraryAccessToken()
  if (token) headers.Authorization = `Bearer ${token}`

  const response = await fetch(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ query, variables }),
  })
  if (!response.ok) {
    throw new LibraryApiError(`Library GraphQL request failed: ${response.status}`, response.status)
  }
  const payload = await response.json() as {
    data?: TData
    errors?: Array<{ message?: string }>
  }
  if (payload.errors?.length) {
    throw new LibraryApiError(payload.errors[0]?.message ?? 'Library GraphQL request failed', 500)
  }
  return payload.data as TData
}

function restValueToLibraryPropertyDataValue(
  value: LibraryRestDataResponse['items'][number]['value']
): import('../recordsApi').LibraryPropertyDataValue {
  if (value == null) return {}
  if (typeof value === 'string') return { string: value }
  if (typeof value === 'number') return { number: String(value) }
  if (Array.isArray(value)) return { optionIds: value.map(String) }
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>
    if (typeof record.string === 'string') return { string: record.string }
    if (typeof record.optionId === 'string') return { optionId: record.optionId }
    if (typeof record.option_id === 'string') return { optionId: record.option_id }
    if (Array.isArray(record.optionIds)) return { optionIds: record.optionIds.map(String) }
    const candidate = Object.values(record)[0]
    if (typeof candidate === 'string') return { string: candidate }
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

function restPropertyPayload(
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
) {
  return propertyData.map((entry) => {
    const property = properties.find((candidate) => candidate.id === entry.propertyId)
    return {
      property_id: entry.propertyId,
      value: property
        ? libraryPropertyValueToRestValue(property, entry.value)
        : '',
    }
  })
}

export async function addLibraryData(
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  input: { name: string; propertyData?: LibraryDataItem['propertyData'] }
): Promise<LibraryDataItem> {
  const propertyData = input.propertyData ?? []
  try {
    const payload = await requestLibraryGraphQL<LibraryAddDataResponse>(
      libraryAddDataMutation,
      {
        input: {
          actor: configuredLibraryActor(),
          orgUsername: target.org,
          repoUsername: target.repo,
          dataName: input.name,
          propertyData: libraryDataItemToGraphqlPropertyData(properties, propertyData),
        },
      },
      { operatorId: target.operatorId }
    )
    if (!payload.addData) throw new LibraryApiError('Library API did not return created data', 500)
    return payload.addData
  } catch {
    // Fall back to REST.
  }

  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data`,
    {
      method: 'POST',
      headers: libraryRestHeaders(target.operatorId),
      body: JSON.stringify({
        name: input.name,
        property_data: restPropertyPayload(properties, propertyData),
      }),
    }
  )
  if (!response.ok) {
    throw new LibraryApiError(`Library REST data create failed: ${response.status}`, response.status)
  }
  const payload = await response.json() as LibraryRestDataResponse
  return restResponseToLibraryDataItem(payload)
}

export async function updateLibraryData(
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  item: LibraryDataItem
): Promise<LibraryDataItem> {
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
          propertyData: libraryDataItemToGraphqlPropertyData(properties, item.propertyData),
        },
      },
      { operatorId: target.operatorId }
    )
    if (!payload.updateData) throw new LibraryApiError('Library API did not return updated data', 500)
    return payload.updateData
  } catch {
    // Fall back to REST.
  }

  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${item.id}`,
    {
      method: 'PUT',
      headers: libraryRestHeaders(target.operatorId),
      body: JSON.stringify({
        name: item.name,
        property_data: restPropertyPayload(properties, item.propertyData),
      }),
    }
  )
  if (!response.ok) {
    throw new LibraryApiError(`Library REST data update failed: ${response.status}`, response.status)
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
    if (!payload.deleteData) throw new LibraryApiError('Library API did not delete data', 500)
    return
  } catch {
    // Fall back to REST.
  }

  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${dataId}`,
    {
      method: 'DELETE',
      headers: libraryRestHeaders(target.operatorId),
    }
  )
  if (!response.ok) {
    throw new LibraryApiError(`Library REST data delete failed: ${response.status}`, response.status)
  }
}
