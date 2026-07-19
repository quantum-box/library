import { appKitConfig } from '../app/kitConfig.js'
import { type DatabaseRecord, type Priority, type Status } from '../data/mock'
import {
  propertyValueList,
  propertyValueText,
} from './libraryTable/libraryPropertyFormat'
import {
  libraryPropertyValueToGraphqlInput,
  mergeLibraryDataProperty,
} from './libraryTable/libraryPropertyInput'
import {
  deleteClientEngineRecord,
  listClientEngineRecords,
  patchClientEngineRecord,
  upsertClientEngineRecord,
} from './photonEngine/client'
import {
  getValidAuthTokens,
  loadAuthTokens,
  loadStoredAuthIdentity,
  storeAuthTokens,
} from './auth'

export { getLibraryDataPropertyValue, propertyValueText } from './libraryTable/libraryPropertyFormat'

export interface ServerRecord {
  id: string
  identifier?: string
  title: string
  description?: string
  status?: string
  priority?: string
  assignee?: string | null
  labels?: string[] | string | null
  project?: string
  created_at?: string
  updated_at?: string
}

export interface ServerRecordListResponse {
  records: ServerRecord[]
  total: number
}

type LibraryPropertyType =
  | 'String'
  | 'Integer'
  | 'Html'
  | 'Markdown'
  | 'Relation'
  | 'Select'
  | 'MultiSelect'
  | 'Id'
  | 'Location'
  | 'Date'
  | 'Image'
  | string

export interface LibrarySelectOption {
  id: string
  key?: string | null
  name?: string | null
}

export interface LibraryProperty {
  id: string
  name: string
  typ: LibraryPropertyType
  meta?: {
    options?: LibrarySelectOption[]
  } | null
}

export interface LibraryPropertyDataValue {
  __typename?: string
  string?: string
  number?: string
  html?: string
  markdown?: string
  date?: string
  url?: string
  id?: string
  optionId?: string
  optionIds?: string[]
  dataIds?: string[]
  databaseId?: string
  latitude?: number
  longitude?: number
}

export interface LibraryDataItem {
  id: string
  name: string
  createdAt?: string
  updatedAt?: string
  propertyData: Array<{
    propertyId: string
    value: LibraryPropertyDataValue
  }>
}

interface LibraryRepoDataResponse {
  repo?: {
    id: string
    name: string
    dataList: {
      items: LibraryDataItem[]
      paginator?: LibraryGraphqlPaginator
    }
    properties: LibraryProperty[]
  } | null
}

interface LibraryPropertiesResponse {
  properties?: LibraryProperty[]
}

interface LibraryGraphqlPaginator {
  currentPage: number
  itemsPerPage: number
  totalItems: number
  totalPages: number
}

interface LibraryDataResponse {
  data?: LibraryDataItem | null
  properties?: LibraryProperty[]
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

export interface LibraryRepository {
  id: string
  username: string
  name: string
  description?: string | null
  orgUsername?: string
  operatorId?: string
  platformTenantId?: string
}

interface LibraryRestRepository {
  id: string
  username: string
  name: string
  description?: string | null
  organization_id?: string
  organizationId?: string
  org_username?: string
  orgUsername?: string
}

interface LibraryOrganizationReposResponse {
  organization?: {
    id: string
    name: string
    username: string
    repos: LibraryRepository[]
  } | null
}

export interface LibraryOrganization {
  id: string
  operatorName: string
  platformTenantId: string
  repos: LibraryRepository[]
}

interface LibraryMeOrganizationsResponse {
  me?: {
    id: string
    email?: string | null
    tenantIdList?: string[]
    organizations: LibraryOrganization[]
  } | null
}

interface LibraryMeTenantListResponse {
  me?: {
    id: string
    email?: string | null
    tenantIdList: string[]
  } | null
}

export interface CreateLibraryOrganizationInput {
  name: string
  username: string
}

export interface CreatedLibraryOrganization {
  id: string
  name: string
  username: string
}

interface LibraryCreateOrganizationResponse {
  createOrganization: CreatedLibraryOrganization
}

interface TachyonOperatorResponse {
  id: string
  name?: string
  operatorName?: string
  platformId?: string
}

interface LibraryRestPropertyData {
  property_id: string
  key: string
  value?: Record<string, unknown> | string | number | string[] | null
}

interface LibraryRestDataResponse {
  id: string
  name: string
  items: LibraryRestPropertyData[]
}

interface LibraryRestDataListResponse {
  data: LibraryRestDataResponse[]
  paginator?: {
    current_page: number
    items_per_page: number
    total_items: number
    total_pages: number
  }
}

interface LibraryRestPropertyResponse {
  id: string
  name: string
  property_type: string
}

export interface LibraryRepoTableData {
  items: LibraryDataItem[]
  properties: LibraryProperty[]
  repoName: string
}

export interface ServerCreateRecordData {
  title: string
  status?: Status
  priority?: Priority
  assignee?: string | null
  description?: string
  labels?: string[]
  project?: string
  orgUsername?: string
  repoUsername?: string
  operatorId?: string
}

export interface ServerUpdateRecordData {
  title?: string
  status?: Status
  priority?: Priority
  assignee?: string | null
  description?: string
  labels?: string[]
  project?: string
  orgUsername?: string
  repoUsername?: string
  operatorId?: string
}

const statuses: Status[] = [
  'backlog',
  'todo',
  'in_progress',
  'in_review',
  'done',
  'cancelled',
]
const priorities: Priority[] = ['urgent', 'high', 'medium', 'low', 'none']
const statusAliases: Record<string, Status> = {
  backlog: 'backlog',
  todo: 'todo',
  open: 'todo',
  doing: 'in_progress',
  inprogress: 'in_progress',
  progress: 'in_progress',
  review: 'in_review',
  inreview: 'in_review',
  done: 'done',
  closed: 'done',
  complete: 'done',
  completed: 'done',
  cancelled: 'cancelled',
  canceled: 'cancelled',
}
const priorityAliases: Record<string, Priority> = {
  urgent: 'urgent',
  high: 'high',
  medium: 'medium',
  normal: 'medium',
  low: 'low',
  none: 'none',
}
const libraryRecordsCollection = 'library_data_records'

const libraryRepoDataQuery = `
  query LibraryClientRepoData($org: String!, $repo: String!, $pageSize: Int, $page: Int) {
    repo(orgUsername: $org, repoUsername: $repo) {
      id
      name
      dataList(pageSize: $pageSize, page: $page) {
        items {
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
        paginator {
          currentPage
          itemsPerPage
          totalItems
          totalPages
        }
      }
      properties {
        id
        name
        typ
        meta {
          ... on SelectType {
            options { id key name }
          }
          ... on MultiSelectType {
            options { id key name }
          }
        }
      }
    }
  }
`

const libraryPropertiesQuery = `
  query LibraryClientProperties($org: String!, $repo: String!) {
    properties(orgUsername: $org, repoUsername: $repo) {
      id
      name
      typ
      meta {
        ... on SelectType {
          options { id key name }
        }
        ... on MultiSelectType {
          options { id key name }
        }
      }
    }
  }
`

const libraryOrganizationReposQuery = `
  query LibraryClientOrganizationRepos($org: String!) {
    organization(username: $org) {
      id
      name
      username
      repos {
        id
        username
        name
        description
      }
    }
  }
`

const libraryMeOrganizationsQuery = `
  query LibraryClientMeOrganizations {
    me {
      id
      email
      tenantIdList
      organizations {
        id
        operatorName
        platformTenantId
      }
    }
  }
`

const libraryMeTenantListQuery = `
  query LibraryClientMeTenantList {
    me {
      id
      email
      tenantIdList
    }
  }
`

const libraryCreateOrganizationMutation = `
  mutation LibraryClientCreateOrganization($input: CreateOrganizationInput!) {
    createOrganization(input: $input) {
      id
      name
      username
    }
  }
`

const libraryDataDetailQuery = `
  query LibraryClientDataDetail($org: String!, $repo: String!, $dataId: String!) {
    data(orgUsername: $org, repoUsername: $repo, dataId: $dataId) {
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
    properties(orgUsername: $org, repoUsername: $repo) {
      id
      name
      typ
      meta {
        ... on SelectType {
          options { id key name }
        }
        ... on MultiSelectType {
          options { id key name }
        }
      }
    }
  }
`

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

export type RecordApiFailureKind =
  | 'transport'
  | 'endpoint-unavailable'
  | 'http'
  | 'graphql'
  | 'invalid-response'
  | 'mapping'

export type LibraryFallbackOperation = 'read' | 'create' | 'update' | 'delete'

export class RecordApiError extends Error {
  readonly status: number
  readonly kind: RecordApiFailureKind

  constructor(message: string, status: number, kind: RecordApiFailureKind = 'http') {
    super(message)
    this.name = 'RecordApiError'
    this.status = status
    this.kind = kind
  }
}

export class RecordPropertyMappingError extends RecordApiError {
  readonly field: keyof ServerCreateRecordData

  constructor(field: keyof ServerCreateRecordData, message: string) {
    super(message, 422, 'mapping')
    this.name = 'RecordPropertyMappingError'
    this.field = field
  }
}

const unavailableGraphqlStatuses = new Set([404, 405, 501])

export function shouldFallbackLibraryRequest(
  error: unknown,
  operation: LibraryFallbackOperation
): boolean {
  if (!(error instanceof RecordApiError)) return false
  if (error.kind === 'endpoint-unavailable') return true
  // A create whose response is lost may already have committed. Without an
  // idempotency key in AddDataInputData, retrying it through REST can duplicate
  // the record, so an ambiguous transport failure must be surfaced instead.
  return error.kind === 'transport' && operation !== 'create'
}

export function libraryApiConfigured(): boolean {
  return Boolean(import.meta.env.VITE_LIBRARY_ORG && import.meta.env.VITE_LIBRARY_REPO)
}

export function libraryOrgConfigured(): boolean {
  return Boolean(import.meta.env.VITE_LIBRARY_ORG)
}

function normalizeStatus(value: string | undefined): Status {
  return canonicalStatus(value) ?? 'backlog'
}

function normalizePriority(value: string | undefined): Priority {
  return canonicalPriority(value) ?? 'none'
}

function normalizeLabels(value: ServerRecord['labels']): string[] {
  if (Array.isArray(value)) {
    return value.filter((label): label is string => typeof label === 'string')
  }

  if (typeof value === 'string' && value.trim()) {
    try {
      const parsed = JSON.parse(value) as unknown
      if (Array.isArray(parsed)) {
        return parsed.filter((label): label is string => typeof label === 'string')
      }
    } catch {
      return []
    }
  }

  return []
}

function configuredLibraryApiBaseUrl(): string {
  return (
    import.meta.env.VITE_LIBRARY_API_BASE_URL ??
    import.meta.env.VITE_BACKEND_API_URL ??
    appKitConfig.server.apiBaseUrl ??
    'http://localhost:50053'
  ).replace(/\/+$/, '')
}

function configuredPlatformId(): string {
  return import.meta.env.VITE_LIBRARY_PLATFORM_ID ?? import.meta.env.VITE_PLATFORM_ID ?? 'tn_01j702qf86pc2j35s0kv0gv3gy'
}

function configuredTachyonApiBaseUrl(): string {
  return (import.meta.env.VITE_TACHYON_API_BASE_URL ?? 'https://api.n1.tachy.one').replace(/\/+$/, '')
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

async function validLibraryAccessToken(): Promise<string | undefined> {
  return (
    import.meta.env.VITE_LIBRARY_ACCESS_TOKEN ||
    (await getValidAuthTokens())?.accessToken
  )
}

async function libraryRestHeaders(operatorId?: string): Promise<Record<string, string>> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = await validLibraryAccessToken()
  if (token) headers.Authorization = `Bearer ${token}`
  return headers
}

function updateStoredAuthFromLibraryUser(user: { id: string; email?: string | null } | null | undefined) {
  if (!user) return

  const parsed = loadAuthTokens()
  if (!parsed) return
  try {
    const next = {
      ...parsed,
      userId: user.id,
      email: user.email ?? parsed.email ?? '',
    }
    if (next.userId !== parsed.userId || next.email !== parsed.email) {
      storeAuthTokens(next)
    }
  } catch {
    // Auth storage is best-effort; API reads can continue without mutating it.
  }
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
  const token = await validLibraryAccessToken()
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
      unavailableGraphqlStatuses.has(response.status) ? 'endpoint-unavailable' : 'http'
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

function normalizedPropertyName(value: string): string {
  return value.trim().toLowerCase().replace(/[\s_-]+/g, '')
}

function canonicalStatus(value: string | undefined): Status | undefined {
  if (!value) return undefined
  const normalized = normalizedPropertyName(value)
  return statusAliases[normalized] ?? (
    statuses.includes(value as Status) ? value as Status : undefined
  )
}

function canonicalPriority(value: string | undefined): Priority | undefined {
  if (!value) return undefined
  const normalized = normalizedPropertyName(value)
  return priorityAliases[normalized] ?? (
    priorities.includes(value as Priority) ? value as Priority : undefined
  )
}

type StandardRecordPropertyField =
  | 'status'
  | 'priority'
  | 'assignee'
  | 'labels'
  | 'description'
  | 'project'

const standardPropertySpecs: Record<
  StandardRecordPropertyField,
  { aliases: string[]; supportedTypes: string[] }
> = {
  status: { aliases: ['status', 'state'], supportedTypes: ['Select', 'String'] },
  priority: { aliases: ['priority'], supportedTypes: ['Select', 'String'] },
  assignee: { aliases: ['assignee', 'owner', '担当'], supportedTypes: ['Select', 'String'] },
  labels: { aliases: ['labels', 'tags', 'tag'], supportedTypes: ['MultiSelect', 'String'] },
  description: {
    aliases: ['description', 'body', 'content', 'markdown', 'html'],
    supportedTypes: ['Markdown', 'Html', 'String'],
  },
  project: { aliases: ['project', 'repo', 'repository'], supportedTypes: ['Select', 'String'] },
}

function standardFieldNeedsProperty(
  field: StandardRecordPropertyField,
  value: ServerCreateRecordData[StandardRecordPropertyField]
): boolean {
  if (field === 'project') return false
  if (field === 'status' || field === 'priority') return value !== undefined
  if (Array.isArray(value)) return value.length > 0
  return typeof value === 'string' && value.length > 0
}

function findStandardProperty(
  properties: LibraryProperty[],
  field: StandardRecordPropertyField,
  value: ServerCreateRecordData[StandardRecordPropertyField]
): LibraryProperty | undefined {
  const spec = standardPropertySpecs[field]
  const aliases = new Set(spec.aliases.map(normalizedPropertyName))
  const candidates = properties.filter((property) => aliases.has(normalizedPropertyName(property.name)))

  if (candidates.length === 0) {
    if (!standardFieldNeedsProperty(field, value)) return undefined
    throw new RecordPropertyMappingError(
      field,
      `Repository schema has no Property for ${field}; expected one of: ${spec.aliases.join(', ')}`
    )
  }

  const property = candidates.find((candidate) => spec.supportedTypes.includes(candidate.typ))
  if (!property) {
    throw new RecordPropertyMappingError(
      field,
      `Repository Property for ${field} has incompatible type ${candidates[0].typ}; expected ${spec.supportedTypes.join(' or ')}`
    )
  }
  return property
}

function optionMatchesStandardField(
  field: StandardRecordPropertyField,
  candidate: string,
  desired: string
): boolean {
  if (normalizedPropertyName(candidate) === normalizedPropertyName(desired)) return true
  if (field === 'status') return canonicalStatus(candidate) === canonicalStatus(desired)
  if (field === 'priority') return canonicalPriority(candidate) === canonicalPriority(desired)
  return false
}

function resolveSelectOptionId(
  field: StandardRecordPropertyField,
  property: LibraryProperty,
  desired: string
): string {
  if (!desired) return ''
  const option = property.meta?.options?.find((candidate) =>
    [candidate.id, candidate.key, candidate.name]
      .filter((value): value is string => typeof value === 'string')
      .some((value) => optionMatchesStandardField(field, value, desired))
  )
  if (!option) {
    throw new RecordPropertyMappingError(
      field,
      `Repository Select Property "${property.name}" has no option matching "${desired}"`
    )
  }
  return option.id
}

function standardPropertyValue(
  field: StandardRecordPropertyField,
  property: LibraryProperty,
  value: ServerCreateRecordData[StandardRecordPropertyField]
): LibraryPropertyDataValue {
  switch (field) {
    case 'status':
    case 'priority': {
      const scalar = String(value)
      return property.typ === 'Select'
        ? { optionId: resolveSelectOptionId(field, property, scalar) }
        : { string: scalar }
    }
    case 'assignee': {
      const scalar = typeof value === 'string' ? value : ''
      return property.typ === 'Select'
        ? { optionId: resolveSelectOptionId(field, property, scalar) }
        : { string: scalar }
    }
    case 'labels': {
      const labels = Array.isArray(value) ? value : []
      return property.typ === 'MultiSelect'
        ? { optionIds: labels.map((label) => resolveSelectOptionId(field, property, label)) }
        : { string: labels.join(', ') }
    }
    case 'description': {
      const scalar = typeof value === 'string' ? value : ''
      if (property.typ === 'Markdown') return { markdown: scalar }
      if (property.typ === 'Html') return { html: scalar }
      return { string: scalar }
    }
    case 'project': {
      const scalar = typeof value === 'string' ? value : ''
      return property.typ === 'Select'
        ? { optionId: resolveSelectOptionId(field, property, scalar) }
        : { string: scalar }
    }
  }
}

function standardRecordPropertyData(
  properties: LibraryProperty[],
  data: ServerCreateRecordData | ServerUpdateRecordData
): LibraryDataItem['propertyData'] {
  const fields: StandardRecordPropertyField[] = [
    'status',
    'priority',
    'assignee',
    'labels',
    'description',
    'project',
  ]
  return fields.flatMap((field) => {
    const value = data[field]
    if (value === undefined) return []
    const property = findStandardProperty(properties, field, value)
    if (!property) return []
    return [{ propertyId: property.id, value: standardPropertyValue(field, property, value) }]
  })
}

function graphqlPropertyData(
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
): Array<{ propertyId: string; value: Record<string, unknown> }> {
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
    case 'MultiSelect':
      return value.optionIds ?? []
    default:
      throw new RecordApiError(
        `REST fallback cannot safely encode ${property.typ} Property "${property.name}"`,
        422,
        'mapping'
      )
  }
}

function restPropertyData(
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

export function libraryDataToRecord(
  item: LibraryDataItem,
  properties: LibraryProperty[],
  repoName: string,
  source?: { orgUsername?: string; repoUsername?: string; operatorId?: string }
): DatabaseRecord {
  const byName = new Map<string, string | string[]>()
  const propertyById = new Map(properties.map((property) => [property.id, property]))

  for (const propertyData of item.propertyData) {
    const property = propertyById.get(propertyData.propertyId)
    if (!property) continue
    const name = normalizedPropertyName(property.name)
    byName.set(name, propertyValueList(property, propertyData.value) ?? propertyValueText(property, propertyData.value) ?? '')
  }

  const textValue = (...names: string[]) => {
    for (const name of names) {
      const value = byName.get(normalizedPropertyName(name))
      if (typeof value === 'string' && value) return value
      if (Array.isArray(value) && value.length > 0) return value.join(', ')
    }
    return undefined
  }

  const listValue = (...names: string[]) => {
    for (const name of names) {
      const value = byName.get(normalizedPropertyName(name))
      if (Array.isArray(value)) return value
      if (typeof value === 'string' && value) {
        return value.split(',').map((item) => item.trim()).filter(Boolean)
      }
    }
    return []
  }

  return {
    id: item.id,
    identifier: textValue('identifier', 'id', 'slug') ?? item.id,
    title: item.name,
    status: normalizeStatus(textValue('status', 'state')),
    priority: normalizePriority(textValue('priority')),
    assignee: textValue('assignee', 'owner', '担当') ?? null,
    labels: listValue('labels', 'tags', 'tag'),
    project: textValue('project', 'repo', 'repository') ?? repoName,
    createdAt: item.createdAt ?? new Date().toISOString(),
    updatedAt: item.updatedAt ?? item.createdAt ?? new Date().toISOString(),
    description: textValue('description', 'body', 'content', 'markdown', 'html') ?? '',
    orgUsername: source?.orgUsername,
    repoUsername: source?.repoUsername,
    operatorId: source?.operatorId,
  }
}

interface LibraryRepoTarget {
  org: string
  repo: string
  operatorId?: string
  repoName?: string
}

function configuredLibraryPageSize(): number {
  const configured = Number(import.meta.env.VITE_LIBRARY_PAGE_SIZE ?? 100)
  if (!Number.isInteger(configured) || configured < 1) return 100
  return Math.min(configured, 100)
}

async function fetchLibraryGraphqlRepoTableData(
  target: LibraryRepoTarget
): Promise<LibraryRepoTableData> {
  const items: LibraryDataItem[] = []
  let properties: LibraryProperty[] = []
  let repoName = target.repoName ?? target.repo
  let page = 1
  let totalPages = 1

  do {
    const payload = await requestLibraryGraphQL<LibraryRepoDataResponse>(
      libraryRepoDataQuery,
      {
        org: target.org,
        repo: target.repo,
        pageSize: configuredLibraryPageSize(),
        page,
      },
      { operatorId: target.operatorId }
    )
    const repoData = payload.repo
    if (!repoData) {
      return { items: [], properties: [], repoName }
    }
    if (page === 1) {
      properties = repoData.properties
      repoName = target.repoName ?? repoData.name
    }
    items.push(...repoData.dataList.items)
    totalPages = Math.max(page, repoData.dataList.paginator?.totalPages ?? page)
    page += 1
  } while (page <= totalPages)

  return { items, properties, repoName }
}

export async function fetchLibraryRepoTableData(
  target: LibraryRepoTarget
): Promise<LibraryRepoTableData> {
  try {
    return await fetchLibraryGraphqlRepoTableData(target)
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'read')) throw error
    return fetchLibraryRestRepoTableData(target)
  }
}

export async function fetchLibraryRecords(target?: LibraryRepoTarget): Promise<DatabaseRecord[]> {
  const org = target?.org ?? import.meta.env.VITE_LIBRARY_ORG
  const repo = target?.repo ?? import.meta.env.VITE_LIBRARY_REPO
  if (!org || !repo) return []

  const resolvedTarget = {
    org,
    repo,
    operatorId: target?.operatorId,
    repoName: target?.repoName,
  }
  try {
    const repoData = await fetchLibraryGraphqlRepoTableData(resolvedTarget)
    return repoData.items.map((item) => libraryDataToRecord(
      item,
      repoData.properties,
      repoData.repoName,
      { orgUsername: org, repoUsername: repo, operatorId: target?.operatorId }
    ))
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'read')) throw error
    return fetchLibraryRestRecords(resolvedTarget)
  }
}

export async function fetchLibraryRepositories(): Promise<LibraryRepository[]> {
  const org = import.meta.env.VITE_LIBRARY_ORG
  if (!org) {
    const organizations = await fetchLibraryOrganizations()
    return organizations.flatMap((organization) =>
      organization.repos.map((repo) => ({
        ...repo,
        orgUsername: organization.operatorName,
        operatorId: organization.id,
        platformTenantId: organization.platformTenantId,
      }))
    )
  }

  const payload = await requestLibraryGraphQL<LibraryOrganizationReposResponse>(
    libraryOrganizationReposQuery,
    { org }
  )
  const operatorId = import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? payload.organization?.id
  const platformTenantId = configuredPlatformId()
  const repos = (payload.organization?.repos ?? []).map((repo) => ({
    ...repo,
    orgUsername: org,
    operatorId,
    platformTenantId,
  }))
  const configuredRepo = import.meta.env.VITE_LIBRARY_REPO
  if (!configuredRepo || repos.some((repo) => repo.username === configuredRepo)) {
    return repos
  }

  return [
    ...repos,
    {
      id: configuredRepo,
      username: configuredRepo,
      name: configuredRepo,
      orgUsername: org,
      operatorId,
      platformTenantId,
    },
  ]
}

export async function fetchLibraryOrganizations(): Promise<LibraryOrganization[]> {
  const restRepos = await fetchLibraryRestRepositories()
  const token = await validLibraryAccessToken()
  if (!token) return hydrateOrganizationsFromRestRepositories(restRepos)

  try {
    const payload = await requestLibraryGraphQL<LibraryMeOrganizationsResponse>(
      libraryMeOrganizationsQuery,
      {}
    )
    updateStoredAuthFromLibraryUser(payload.me)
    const organizations = payload.me?.organizations ?? []
    const scopedOrganizations = organizations.filter(
      (organization) => organization.platformTenantId === configuredPlatformId()
    )
    const selectedOrganizations = scopedOrganizations.length > 0 ? scopedOrganizations : organizations

    return Promise.all(
      selectedOrganizations.map(async (organization) => {
        try {
          const repos = await fetchLibraryOrganizationRepos(organization)
          return {
            ...organization,
            repos: repos.length > 0
              ? repos
              : reposFromRestForOrganization(restRepos, organization),
          }
        } catch {
          return {
            ...organization,
            repos: reposFromRestForOrganization(restRepos, organization),
          }
        }
      })
    )
  } catch {
    try {
      const payload = await requestLibraryGraphQL<LibraryMeTenantListResponse>(
        libraryMeTenantListQuery,
        {}
      )
      updateStoredAuthFromLibraryUser(payload.me)
      return Promise.all(
        (payload.me?.tenantIdList ?? []).map(async (tenantId) => {
          const operator = await fetchTachyonOperator(tenantId)
          const organization = {
            id: tenantId,
            operatorName: operator?.operatorName ?? tenantId,
            platformTenantId: operator?.platformId ?? configuredPlatformId(),
          }
          return {
            ...organization,
            repos: reposFromRestForOrganization(restRepos, organization),
          }
        })
      )
    } catch {
      return hydrateOrganizationsFromRestRepositories(restRepos)
    }
  }
}

export async function createLibraryOrganization(
  input: CreateLibraryOrganizationInput
): Promise<CreatedLibraryOrganization> {
  const payload = await requestLibraryGraphQL<LibraryCreateOrganizationResponse>(
    libraryCreateOrganizationMutation,
    {
      input: {
        name: input.name.trim(),
        username: input.username.trim(),
      },
    }
  )
  return payload.createOrganization
}

async function fetchTachyonOperator(tenantId: string): Promise<TachyonOperatorResponse | null> {
  const token = await validLibraryAccessToken()
  if (!token) return null

  try {
    const response = await fetch(`${configuredTachyonApiBaseUrl()}/v1/auth/operators/${tenantId}`, {
      headers: {
        'x-platform-id': configuredPlatformId(),
        'x-operator-id': tenantId,
        Authorization: `Bearer ${token}`,
      },
    })
    if (!response.ok) return null
    return await response.json() as TachyonOperatorResponse
  } catch {
    return null
  }
}

async function fetchLibraryRestRepositories(): Promise<LibraryRestRepository[]> {
  const publicRepos = await requestLibraryRestRepositories()
  if (publicRepos.length > 0) return publicRepos

  const headers: Record<string, string> = {
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  const token = await validLibraryAccessToken()
  if (token) headers.Authorization = `Bearer ${token}`

  return requestLibraryRestRepositories(headers)
}

async function requestLibraryRestRepositories(
  headers?: Record<string, string>
): Promise<LibraryRestRepository[]> {
  const baseUrls = [
    configuredLibraryApiBaseUrl(),
    'https://library-api.txcloud.app',
  ].filter((url, index, urls) => urls.indexOf(url) === index)

  for (const baseUrl of baseUrls) {
    try {
      const response = await fetch(`${baseUrl}/v1beta/repos`, {
        ...(headers ? { headers } : {}),
      })
      if (!response.ok) continue
      const payload = await response.json()
      if (Array.isArray(payload)) return payload as LibraryRestRepository[]
    } catch {
      // Try the next known Library API base URL.
    }
  }
  return []
}

function restRepoOrganizationId(repo: LibraryRestRepository): string | undefined {
  return repo.organization_id ?? repo.organizationId
}

function restRepoOrgUsername(repo: LibraryRestRepository): string | undefined {
  return repo.org_username ?? repo.orgUsername
}

function restRepoToLibraryRepository(
  repo: LibraryRestRepository,
  organization: Pick<LibraryOrganization, 'id' | 'operatorName' | 'platformTenantId'>
): LibraryRepository {
  return {
    id: repo.id,
    username: repo.username,
    name: repo.name,
    description: repo.description,
    orgUsername: restRepoOrgUsername(repo) ?? organization.operatorName,
    operatorId: organization.id,
    platformTenantId: organization.platformTenantId,
  }
}

function restValueToLibraryPropertyDataValue(
  value: LibraryRestPropertyData['value']
): LibraryPropertyDataValue {
  if (value == null) return {}
  if (typeof value === 'string') return { string: value }
  if (typeof value === 'number') return { number: String(value) }
  if (Array.isArray(value)) {
    return { optionIds: value.map((item) => String(item)) }
  }
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>
    if (typeof record.string === 'string') return { string: record.string }
    if (typeof record.integer === 'string' || typeof record.integer === 'number') {
      return { number: String(record.integer) }
    }
    if (typeof record.number === 'string' || typeof record.number === 'number') {
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
    if (Array.isArray(record.multiSelect)) {
      return { optionIds: record.multiSelect.map((item) => String(item)) }
    }
    if (Array.isArray(record.optionIds)) {
      return { optionIds: record.optionIds.map((item) => String(item)) }
    }
    if (Array.isArray(record.option_ids)) {
      return { optionIds: record.option_ids.map((item) => String(item)) }
    }
    if (Array.isArray(record.dataIds)) {
      return { dataIds: record.dataIds.map((item) => String(item)) }
    }
    if (record.relation && typeof record.relation === 'object') {
      const relation = record.relation as Record<string, unknown>
      const ids = Array.isArray(relation.data_id)
        ? relation.data_id.map((item) => String(item))
        : []
      return {
        dataIds: ids,
        databaseId: typeof relation.database_id === 'string' ? relation.database_id : undefined,
      }
    }
    if (record.location && typeof record.location === 'object') {
      const location = record.location as Record<string, unknown>
      if (typeof location.latitude === 'number' && typeof location.longitude === 'number') {
        return { latitude: location.latitude, longitude: location.longitude }
      }
    }
    if (typeof record.latitude === 'number' && typeof record.longitude === 'number') {
      return { latitude: record.latitude, longitude: record.longitude }
    }
    const candidate = Object.values(record)[0]
    if (typeof candidate === 'string') return { string: candidate }
    if (typeof candidate === 'number') return { number: String(candidate) }
  }
  return {}
}

function restDataToLibraryDataItem(item: LibraryRestDataResponse): LibraryDataItem {
  return {
    id: item.id,
    name: item.name,
    propertyData: item.items.map((propertyData) => ({
      propertyId: propertyData.property_id,
      value: restValueToLibraryPropertyDataValue(propertyData.value),
    })),
  }
}

function restPropertyToLibraryProperty(property: LibraryRestPropertyResponse): LibraryProperty {
  return {
    id: property.id,
    name: property.name,
    typ: property.property_type,
    meta: null,
  }
}

async function fetchLibraryRestProperties(target: LibraryRepoTarget): Promise<LibraryProperty[]> {
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/properties`,
    { headers: await libraryRestHeaders(target.operatorId) }
  )
  if (!response.ok) {
    throw new RecordApiError(
      `Library REST properties request failed: ${response.status}`,
      response.status
    )
  }
  const payload = await response.json() as LibraryRestPropertyResponse[]
  if (!Array.isArray(payload)) {
    throw new RecordApiError(
      'Library REST properties returned an invalid response',
      response.status,
      'invalid-response'
    )
  }
  return payload.map(restPropertyToLibraryProperty)
}

export async function fetchLibraryRepoProperties(
  target: LibraryRepoTarget
): Promise<LibraryProperty[]> {
  try {
    const payload = await requestLibraryGraphQL<LibraryPropertiesResponse>(
      libraryPropertiesQuery,
      { org: target.org, repo: target.repo },
      { operatorId: target.operatorId }
    )
    if (!Array.isArray(payload.properties)) {
      throw new RecordApiError(
        'Library GraphQL returned no Property definitions',
        200,
        'invalid-response'
      )
    }
    return payload.properties
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'read')) throw error
    return fetchLibraryRestProperties(target)
  }
}

async function fetchLibraryRestRepoTableData(
  target: LibraryRepoTarget
): Promise<LibraryRepoTableData> {
  const pageSize = configuredLibraryPageSize()
  const headers = await libraryRestHeaders(target.operatorId)
  const baseUrl = configuredLibraryApiBaseUrl()
  const propertiesResponsePromise = fetch(
    `${baseUrl}/v1beta/repos/${target.org}/${target.repo}/properties`,
    { headers }
  )
  const items: LibraryDataItem[] = []
  let page = 1
  let totalPages = 1
  do {
    const dataResponse = await fetch(
      `${baseUrl}/v1beta/repos/${target.org}/${target.repo}/data-list?page=${page}&page_size=${pageSize}`,
      { headers }
    )
    if (!dataResponse.ok) {
      throw new RecordApiError(`Library REST data list failed: ${dataResponse.status}`, dataResponse.status)
    }
    const dataPayload = await dataResponse.json() as LibraryRestDataListResponse
    items.push(...(dataPayload.data ?? []).map(restDataToLibraryDataItem))
    totalPages = Math.max(page, dataPayload.paginator?.total_pages ?? page)
    page += 1
  } while (page <= totalPages)

  const propertiesResponse = await propertiesResponsePromise
  let properties: LibraryProperty[] = []
  if (propertiesResponse.ok) {
    const propertiesPayload = await propertiesResponse.json() as LibraryRestPropertyResponse[]
    properties = (Array.isArray(propertiesPayload) ? propertiesPayload : []).map(restPropertyToLibraryProperty)
  }
  return {
    items,
    properties,
    repoName: target.repoName ?? target.repo,
  }
}

async function fetchLibraryRestRecords(target: LibraryRepoTarget): Promise<DatabaseRecord[]> {
  const { items, properties, repoName } = await fetchLibraryRestRepoTableData(target)
  return items.map((item) => libraryDataToRecord(
    item,
    properties,
    repoName,
    { orgUsername: target.org, repoUsername: target.repo, operatorId: target.operatorId }
  ))
}

async function createLibraryRestData(
  data: ServerCreateRecordData,
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
): Promise<LibraryDataItem> {
  const response = await fetch(`${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data`, {
    method: 'POST',
    headers: await libraryRestHeaders(target.operatorId),
    body: JSON.stringify({
      name: data.title,
      property_data: restPropertyData(properties, propertyData),
    }),
  })
  if (!response.ok) throw new RecordApiError(`Library REST data create failed: ${response.status}`, response.status)
  const payload = await response.json() as LibraryRestDataResponse
  return restDataToLibraryDataItem(payload)
}

async function updateLibraryRestData(
  dataId: string,
  dataName: string,
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
): Promise<LibraryDataItem> {
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${dataId}`,
    {
      method: 'PUT',
      headers: await libraryRestHeaders(target.operatorId),
      body: JSON.stringify({
        name: dataName,
        property_data: restPropertyData(properties, propertyData),
      }),
    }
  )
  if (!response.ok) {
    throw new RecordApiError(`Library REST data update failed: ${response.status}`, response.status)
  }
  return restDataToLibraryDataItem(await response.json() as LibraryRestDataResponse)
}

function reposFromRestForOrganization(
  repos: LibraryRestRepository[],
  organization: Pick<LibraryOrganization, 'id' | 'operatorName' | 'platformTenantId'>
): LibraryRepository[] {
  return repos
    .filter((repo) => restRepoOrganizationId(repo) === organization.id)
    .map((repo) => restRepoToLibraryRepository(repo, organization))
}

async function hydrateOrganizationsFromRestRepositories(
  repos: LibraryRestRepository[]
): Promise<LibraryOrganization[]> {
  const organizations = new Map<string, LibraryOrganization>()
  repos.forEach((repo) => {
    const organizationId = restRepoOrganizationId(repo)
    if (!organizationId) return
    const orgUsername = restRepoOrgUsername(repo)
    const organization = organizations.get(organizationId) ?? {
      id: organizationId,
      operatorName: orgUsername ?? organizationId,
      platformTenantId: configuredPlatformId(),
      repos: [],
    }
    if (orgUsername && organization.operatorName === organizationId) {
      organization.operatorName = orgUsername
    }
    organization.repos.push(restRepoToLibraryRepository(repo, organization))
    organizations.set(organizationId, organization)
  })
  return Promise.all(
    [...organizations.values()].map(async (organization) => {
      const operator = await fetchTachyonOperator(organization.id)
      if (!operator?.operatorName) return organization
      const operatorName = organization.operatorName === organization.id
        ? operator.operatorName
        : organization.operatorName
      const nextOrganization = {
        ...organization,
        operatorName,
        platformTenantId: operator.platformId ?? organization.platformTenantId,
      }
      return {
        ...nextOrganization,
        repos: organization.repos.map((repo) => ({
          ...repo,
          orgUsername: nextOrganization.operatorName,
          platformTenantId: nextOrganization.platformTenantId,
        })),
      }
    })
  )
}

async function fetchLibraryOrganizationRepos(
  organization: Omit<LibraryOrganization, 'repos'> | LibraryOrganization
): Promise<LibraryRepository[]> {
  const payload = await requestLibraryGraphQL<LibraryOrganizationReposResponse>(
    libraryOrganizationReposQuery,
    { org: organization.operatorName },
    { operatorId: organization.id }
  )
  return (payload.organization?.repos ?? []).map((repo) => ({
    ...repo,
    orgUsername: organization.operatorName,
    operatorId: organization.id,
    platformTenantId: organization.platformTenantId,
  }))
}

async function fetchLibraryRestDataDetail(
  dataId: string,
  target: LibraryRepoTarget
): Promise<{ item: LibraryDataItem; properties: LibraryProperty[] }> {
  const headers = await libraryRestHeaders(target.operatorId)
  const baseUrl = configuredLibraryApiBaseUrl()
  const [dataResponse, properties] = await Promise.all([
    fetch(`${baseUrl}/v1beta/repos/${target.org}/${target.repo}/data/${dataId}`, { headers }),
    fetchLibraryRestProperties(target),
  ])
  if (!dataResponse.ok) {
    throw new RecordApiError(
      `Library REST data detail failed: ${dataResponse.status}`,
      dataResponse.status
    )
  }
  const payload = await dataResponse.json() as LibraryRestDataResponse
  return { item: restDataToLibraryDataItem(payload), properties }
}

export async function fetchLibraryDataDetail(dataId: string, target?: Partial<LibraryRepoTarget>): Promise<{
  item: LibraryDataItem
  properties: LibraryProperty[]
}> {
  const org = target?.org ?? import.meta.env.VITE_LIBRARY_ORG
  const repo = target?.repo ?? import.meta.env.VITE_LIBRARY_REPO
  if (!org || !repo) throw new RecordApiError('Library API is not configured', 400)

  const resolvedTarget = { org, repo, operatorId: target?.operatorId }
  try {
    const payload = await requestLibraryGraphQL<LibraryDataResponse>(
      libraryDataDetailQuery,
      { org, repo, dataId },
      { operatorId: target?.operatorId }
    )
    if (!payload.data) throw new RecordApiError('Data not found', 404)
    if (!Array.isArray(payload.properties)) {
      throw new RecordApiError(
        'Library GraphQL returned no Property definitions',
        200,
        'invalid-response'
      )
    }
    return { item: payload.data, properties: payload.properties }
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'read')) throw error
    return fetchLibraryRestDataDetail(dataId, resolvedTarget)
  }
}

export function toRecord(serverRecord: ServerRecord): DatabaseRecord {
  return {
    id: serverRecord.id,
    identifier: serverRecord.identifier ?? serverRecord.id,
    title: serverRecord.title,
    status: normalizeStatus(serverRecord.status),
    priority: normalizePriority(serverRecord.priority),
    assignee: serverRecord.assignee || null,
    labels: normalizeLabels(serverRecord.labels),
    project: serverRecord.project ?? appKitConfig.records.defaultProject,
    createdAt: serverRecord.created_at ?? new Date().toISOString(),
    updatedAt: serverRecord.updated_at ?? serverRecord.created_at ?? new Date().toISOString(),
    description: serverRecord.description ?? '',
  }
}

function withoutUndefined<T extends object>(payload: T): Partial<T> {
  return Object.fromEntries(
    Object.entries(payload).filter(([, value]) => value !== undefined)
  ) as Partial<T>
}

function randomRecordId() {
  return globalThis.crypto?.randomUUID?.() ?? `record-${Date.now()}-${Math.random().toString(36).slice(2)}`
}

function nextIdentifier(records: DatabaseRecord[]) {
  const prefix = appKitConfig.records.identifierPrefix
  const maxNumber = records.reduce((max, record) => {
    const match = record.identifier?.match(new RegExp(`^${prefix}-(\\d+)$`))
    return match ? Math.max(max, Number(match[1])) : max
  }, 100)
  return `${prefix}-${maxNumber + 1}`
}

function activeRecordsCollection(): string {
  return libraryApiConfigured() ? libraryRecordsCollection : 'records'
}

export async function fetchServerRecords(): Promise<DatabaseRecord[]> {
  if (libraryApiConfigured()) {
    try {
      const libraryRecords = await fetchLibraryRecords()
      await Promise.all(
        libraryRecords.map((record) => upsertClientEngineRecord(libraryRecordsCollection, record.id, record))
      )
      return libraryRecords
    } catch (error) {
      const cached = await listClientEngineRecords<DatabaseRecord>(libraryRecordsCollection)
      if (cached.length > 0) {
        return cached.map((record) => record.value)
      }
      throw error
    }
  }

  if (await validLibraryAccessToken()) {
    try {
      const repositories = await fetchLibraryRepositories()
      const libraryRecords = (await Promise.all(
        repositories.map((repo) =>
          repo.orgUsername
            ? fetchLibraryRecords({
                org: repo.orgUsername,
                repo: repo.username,
                operatorId: repo.operatorId,
                repoName: `${repo.orgUsername} / ${repo.name || repo.username}`,
              })
            : Promise.resolve([])
        )
      )).flat()
      await Promise.all(
        libraryRecords.map((record) => upsertClientEngineRecord(libraryRecordsCollection, record.id, record))
      )
      return libraryRecords
    } catch (error) {
      const cached = await listClientEngineRecords<DatabaseRecord>(libraryRecordsCollection)
      if (cached.length > 0) {
        return cached.map((record) => record.value)
      }
      throw error
    }
  }

  const records = await listClientEngineRecords<DatabaseRecord>('records')
  if (import.meta.env.MODE === 'test') {
    return records.map((record) => record.value)
  }
  return []
}

export async function createServerRecord(data: ServerCreateRecordData): Promise<DatabaseRecord> {
  const targetOrg = data.orgUsername ?? import.meta.env.VITE_LIBRARY_ORG
  const targetRepo = data.repoUsername ?? import.meta.env.VITE_LIBRARY_REPO
  const targetOperatorId = data.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID
  if (targetOrg && targetRepo && (libraryApiConfigured() || await validLibraryAccessToken())) {
    const target = {
      org: targetOrg,
      repo: targetRepo,
      operatorId: targetOperatorId,
      repoName: data.project ?? targetRepo,
    }
    const properties = await fetchLibraryRepoProperties(target)
    const propertyData = standardRecordPropertyData(properties, data)
    let created: LibraryDataItem
    try {
      const payload = await requestLibraryGraphQL<LibraryAddDataResponse>(
        libraryAddDataMutation,
        {
          input: {
            actor: configuredLibraryActor(),
            orgUsername: targetOrg,
            repoUsername: targetRepo,
            dataName: data.title,
            propertyData: graphqlPropertyData(properties, propertyData),
          },
        },
        { operatorId: targetOperatorId }
      )
      if (!payload.addData) {
        throw new RecordApiError(
          'Library API did not return created data',
          500,
          'invalid-response'
        )
      }
      created = payload.addData
    } catch (error: unknown) {
      if (!shouldFallbackLibraryRequest(error, 'create')) throw error
      created = await createLibraryRestData(data, target, properties, propertyData)
    }

    const record = libraryDataToRecord(created, properties, target.repoName, {
      orgUsername: targetOrg,
      repoUsername: targetRepo,
      operatorId: targetOperatorId,
    })
    await upsertClientEngineRecord(libraryRecordsCollection, record.id, record)
    return record
  }

  const collection = activeRecordsCollection()
  const records = (await listClientEngineRecords<DatabaseRecord>(collection)).map((record) => record.value)
  const now = new Date().toISOString()
  const record: DatabaseRecord = {
    id: randomRecordId(),
    identifier: nextIdentifier(records),
    title: data.title,
    status: data.status ?? 'todo',
    priority: data.priority ?? 'none',
    assignee: data.assignee ?? null,
    labels: data.labels ?? [],
    project: data.project ?? appKitConfig.records.defaultProject,
    createdAt: now,
    updatedAt: now,
    description: data.description ?? '',
    orgUsername: data.orgUsername,
    repoUsername: data.repoUsername,
    operatorId: data.operatorId,
  }
  const storedRecord = await upsertClientEngineRecord(collection, record.id, record)
  return storedRecord.value
}

export async function updateServerRecord(
  recordId: string,
  data: ServerUpdateRecordData
): Promise<DatabaseRecord> {
  const cachedLibraryRecord = (await listClientEngineRecords<DatabaseRecord>(libraryRecordsCollection))
    .find((record) => record.recordId === recordId)?.value
  const targetOrg = data.orgUsername ?? cachedLibraryRecord?.orgUsername ?? import.meta.env.VITE_LIBRARY_ORG
  const targetRepo = data.repoUsername ?? cachedLibraryRecord?.repoUsername ?? import.meta.env.VITE_LIBRARY_REPO
  const targetOperatorId = data.operatorId ?? cachedLibraryRecord?.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID
  if (targetOrg && targetRepo && (libraryApiConfigured() || await validLibraryAccessToken())) {
    const target = {
      org: targetOrg,
      repo: targetRepo,
      operatorId: targetOperatorId,
      repoName: data.project ?? cachedLibraryRecord?.project ?? targetRepo,
    }
    const existing = await fetchLibraryDataDetail(recordId, target)
    const nextName = data.title ?? existing.item.name
    const changedPropertyData = standardRecordPropertyData(existing.properties, data)
    const propertyData = changedPropertyData.reduce(
      (item, entry) => mergeLibraryDataProperty(item, entry.propertyId, entry.value),
      existing.item
    ).propertyData
    let updated: LibraryDataItem
    try {
      const payload = await requestLibraryGraphQL<LibraryUpdateDataResponse>(
        libraryUpdateDataMutation,
        {
          input: {
            actor: configuredLibraryActor(),
            orgUsername: targetOrg,
            repoUsername: targetRepo,
            dataId: recordId,
            dataName: nextName,
            propertyData: graphqlPropertyData(existing.properties, propertyData),
          },
        },
        { operatorId: targetOperatorId }
      )
      if (!payload.updateData) {
        throw new RecordApiError(
          'Library API did not return updated data',
          500,
          'invalid-response'
        )
      }
      updated = payload.updateData
    } catch (error: unknown) {
      if (!shouldFallbackLibraryRequest(error, 'update')) throw error
      updated = await updateLibraryRestData(
        recordId,
        nextName,
        target,
        existing.properties,
        propertyData
      )
    }
    const completeUpdated = updated.propertyData.reduce(
      (item, entry) => mergeLibraryDataProperty(item, entry.propertyId, entry.value),
      { ...updated, propertyData }
    )
    const record = libraryDataToRecord(completeUpdated, existing.properties, target.repoName, {
      orgUsername: targetOrg,
      repoUsername: targetRepo,
      operatorId: targetOperatorId,
    })
    await upsertClientEngineRecord(libraryRecordsCollection, record.id, record)
    return record
  }

  const collection = activeRecordsCollection()
  const existing = (await listClientEngineRecords<DatabaseRecord>(collection))
    .find((record) => record.recordId === recordId)?.value
  if (!existing) {
    throw new RecordApiError('Record not found', 404)
  }

  const record: DatabaseRecord = {
    ...existing,
    ...withoutUndefined(data),
    assignee: data.assignee === undefined ? existing.assignee : data.assignee,
    labels: data.labels ?? existing.labels,
    updatedAt: new Date().toISOString(),
  }
  const storedRecord = await patchClientEngineRecord<DatabaseRecord>(collection, recordId, record)
  if (!storedRecord) throw new RecordApiError('Record not found', 404)
  return storedRecord.value
}

export async function deleteServerRecord(recordId: string): Promise<void> {
  const existing = (await listClientEngineRecords<DatabaseRecord>(libraryRecordsCollection))
    .find((record) => record.recordId === recordId)?.value
  const targetOrg = existing?.orgUsername ?? import.meta.env.VITE_LIBRARY_ORG
  const targetRepo = existing?.repoUsername ?? import.meta.env.VITE_LIBRARY_REPO
  const targetOperatorId = existing?.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID
  if (targetOrg && targetRepo && (libraryApiConfigured() || await validLibraryAccessToken())) {
    try {
      const payload = await requestLibraryGraphQL<LibraryDeleteDataResponse>(
        libraryDeleteDataMutation,
        { org: targetOrg, repo: targetRepo, dataId: recordId },
        { operatorId: targetOperatorId }
      )
      if (!payload.deleteData) {
        throw new RecordApiError(
          'Library API did not delete data',
          500,
          'invalid-response'
        )
      }
    } catch (error: unknown) {
      if (!shouldFallbackLibraryRequest(error, 'delete')) throw error
      const response = await fetch(
        `${configuredLibraryApiBaseUrl()}/v1beta/repos/${targetOrg}/${targetRepo}/data/${recordId}`,
        {
          method: 'DELETE',
          headers: await libraryRestHeaders(targetOperatorId),
        }
      )
      if (!response.ok && response.status !== 404) {
        throw new RecordApiError(`Library REST data delete failed: ${response.status}`, response.status)
      }
    }
    await deleteClientEngineRecord(libraryRecordsCollection, recordId)
    return
  }

  await deleteClientEngineRecord(activeRecordsCollection(), recordId)
}
