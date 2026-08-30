import type { RestListResult, RestResource } from '@quantum-box/photon'

import { appKitConfig } from '../app/kitConfig.js'
import { type DatabaseRecord, type Priority, type Status } from '../data/mock'
import {
  propertyValueList,
  propertyValueText,
} from './libraryTable/libraryPropertyFormat'
import { mergeLibraryDataProperty } from './libraryTable/libraryPropertyInput'
import {
  deleteAndPushClientEngineRecord,
  deleteClientEngineRecord,
  ingestClientEngineRecords,
  listClientEngineRecords,
  newClientEngineRecordId,
  patchAndPushClientEngineRecord,
  patchClientEngineRecord,
  subscribeClientEngineRollbacks,
  subscribeClientEngineSettlements,
  upsertAndPushClientEngineRecord,
  upsertClientEngineRecord,
  type ClientEngineWriteResult,
} from './photonEngine/client'
import {
  knownLibraryRepositories,
  LEGACY_LIBRARY_RECORDS_COLLECTION,
  LIBRARY_REPOSITORIES_COLLECTION,
  type LibraryRecordsRepository,
  libraryRecordsCollection,
  libraryRecordsDatabaseId,
  libraryRepositoryByName,
  rememberLibraryRepositories,
  setLibraryRecordsResourceFactory,
} from './photonEngine/libraryCollections'
import {
  carryLegacyLibraryRecords,
  legacyRecordsCollectionPending,
} from './photonEngine/recordsCollectionMigration'
import {
  getValidAuthTokens,
  loadAuthTokens,
  loadStoredAuthIdentity,
  storeAuthTokens,
} from './auth'
import { t } from '../i18n'

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
  | 'RichText'
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

/**
 * The Library API serializes PropertyType as SCREAMING_SNAKE_CASE on both
 * GraphQL (`MARKDOWN`, `MULTI_SELECT`, …) and REST (`property_type`).
 * This client compares against PascalCase (`Markdown`, `MultiSelect`, …), so
 * every Property ingestion path must normalize through this map.
 */
const libraryPropertyTypeByWireValue: Record<string, LibraryPropertyType> = {
  STRING: 'String',
  INTEGER: 'Integer',
  HTML: 'Html',
  MARKDOWN: 'Markdown',
  RELATION: 'Relation',
  SELECT: 'Select',
  MULTI_SELECT: 'MultiSelect',
  ID: 'Id',
  LOCATION: 'Location',
  DATE: 'Date',
  IMAGE: 'Image',
  RICH_TEXT: 'RichText',
}

export function normalizeLibraryPropertyType(typ: string): LibraryPropertyType {
  return libraryPropertyTypeByWireValue[typ.toUpperCase()] ?? typ
}

function normalizeLibraryProperty(property: LibraryProperty): LibraryProperty {
  return { ...property, typ: normalizeLibraryPropertyType(property.typ) }
}

export interface LibraryPropertyDataValue {
  __typename?: string
  string?: string
  number?: string
  html?: string
  markdown?: string
  /** The block document as JSON text. Authoritative for RichText. */
  richText?: string
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

/**
 * A Tachyon tenant the signed-in user belongs to, annotated with whether it is
 * already usable as a Library organization. Tenant membership alone does not
 * grant Library access, so these are the candidates for `seedLibraryTenant`.
 */
export interface LibraryAccessibleTenant {
  tenantId: string
  name: string
  username: string
  /**
   * Members of the tenant, or `null` when the API could not count them —
   * listing a tenant's users needs permission inside that tenant, which
   * belonging to it does not by itself grant. Never render a missing count
   * as `0`; the two mean different things to someone sizing up an import.
   */
  staffCount: number | null
  hasLibraryOrg: boolean
  canImportToLibrary: boolean
}

interface LibraryAccessibleTenantsResponse {
  accessibleTenants?: LibraryAccessibleTenant[] | null
}

interface LibrarySeedTenantResponse {
  seedLibraryTenant: {
    organization: CreatedLibraryOrganization
    seeded: boolean
    staffCount: number
  }
}

export interface CreateLibraryRepositoryInput {
  orgUsername: string
  operatorId: string
  name: string
  username: string
  description?: string
  isPublic: boolean
}

export interface CreatedLibraryRepository {
  id: string
  name: string
  username: string
  description?: string | null
  orgUsername: string
  isPublic: boolean
}

interface LibraryCreateRepositoryResponse {
  createRepo: CreatedLibraryRepository
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
  /**
   * The repository's canonical Library id, when the response carried one.
   *
   * This is what names the collection the rows are cached in, so it is read
   * here rather than looked up separately. Optional because the REST fallback
   * asks for it on a request of its own, which is allowed to fail.
   */
  repoId?: string
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
              ... on RichTextValue { richText }
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

const libraryAccessibleTenantsQuery = `
  query LibraryClientAccessibleTenants {
    accessibleTenants {
      tenantId
      name
      username
      staffCount
      hasLibraryOrg
      canImportToLibrary
    }
  }
`

const librarySeedTenantMutation = `
  mutation LibraryClientSeedLibraryTenant($tenantId: String!) {
    seedLibraryTenant(tenantId: $tenantId) {
      organization {
        id
        name
        username
      }
      seeded
      staffCount
    }
  }
`

const libraryCreateRepositoryMutation = `
  mutation LibraryClientCreateRepository($input: CreateRepoInput!) {
    createRepo(input: $input) {
      id
      name
      username
      description
      orgUsername
      isPublic
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
          ... on RichTextValue { richText }
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

/**
 * `anonymous` sends the request with no Authorization header even when a
 * session exists. The public read-only view is the only caller: a page that
 * claims to show what a signed-out visitor sees has to actually ask as one,
 * or an owner previewing their own repository would be shown their own
 * privileged read.
 */
interface LibraryRequestOptions {
  anonymous?: boolean
}

async function libraryRestHeaders(
  operatorId?: string,
  options?: LibraryRequestOptions
): Promise<Record<string, string>> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  if (options?.anonymous) return headers

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
  options?: LibraryRequestOptions & { operatorId?: string }
): Promise<TData> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': options?.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
  }
  if (!options?.anonymous) {
    const token = await validLibraryAccessToken()
    if (token) headers.Authorization = `Bearer ${token}`
  }

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
  value: ServerCreateRecordData[StandardRecordPropertyField],
  options?: { requireProperty?: boolean }
): LibraryProperty | undefined {
  const spec = standardPropertySpecs[field]
  const aliases = new Set(spec.aliases.map(normalizedPropertyName))
  const candidates = properties.filter((property) => aliases.has(normalizedPropertyName(property.name)))

  if (candidates.length === 0) {
    if (!standardFieldNeedsProperty(field, value)) return undefined
    // Skipped rather than refused when the caller is a `RestResource`. What it
    // holds is a whole `DatabaseRecord`, where `status` and `priority` are
    // always set — a record carries them whether or not anyone asked for one —
    // so their presence there is not evidence of a request the repository has
    // to honour. `assertRecordFieldsMappable` has already refused the fields
    // the caller really did supply, at the point the edit was made; by here
    // the only thing left to drop is a default nobody chose.
    if (options?.requireProperty === false) return undefined
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

/**
 * The canonical record fields, as this repository's Properties.
 *
 * `requireProperty: false` is for a caller holding a whole `DatabaseRecord`
 * rather than the fields someone typed — the `RestResource`, which is handed
 * the record Photon stored. A record always has a `status` and a `priority`,
 * so refusing a repository that defines neither would reject every create
 * against it, including the ones that never mentioned either field. The strict
 * check still runs, once, in `assertRecordFieldsMappable`, against what the
 * caller actually supplied.
 *
 * A Property that exists but has no matching option still throws in both
 * modes: that is a value going missing, not a default being dropped.
 */
function standardRecordPropertyData(
  properties: LibraryProperty[],
  data: ServerCreateRecordData | ServerUpdateRecordData,
  options?: { requireProperty?: boolean }
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
    const property = findStandardProperty(properties, field, value, options)
    if (!property) return []
    return [{ propertyId: property.id, value: standardPropertyValue(field, property, value) }]
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
    case 'Select':
      // Tagged, unlike the bare array MultiSelect sends just below. The API
      // reads the JSON without knowing which Property it is for, so a bare
      // string would land in its `String` arm and be rejected against a
      // Select Property. An array of strings is unambiguous enough to stay
      // bare; a single string is not. An empty option id clears the value.
      return { optionId: value.optionId ?? '' }
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
  anonymous?: boolean
  /** The repository's canonical Library id, where the caller knows it. */
  databaseId?: string
}

/**
 * Image formats the API stores. Refusing here keeps the editor's error
 * message specific instead of surfacing a bare 400 from the round trip.
 */
const UPLOADABLE_IMAGE_TYPES = new Set([
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
  'image/avif',
])

/**
 * Stores an image in the repository and returns the URL to embed in a body.
 *
 * The URL is a plain API address, not a presigned one, so it keeps working
 * after the signature that fetched it first would have expired.
 */
export async function uploadLibraryImage(
  target: { org: string; repo: string; operatorId?: string },
  file: File,
): Promise<string> {
  if (!UPLOADABLE_IMAGE_TYPES.has(file.type)) {
    throw new RecordApiError(
      `Unsupported image type: ${file.type || 'unknown'}`,
      415,
    )
  }

  const headers = await libraryRestHeaders(target.operatorId)
  headers['content-type'] = file.type
  const query = `?filename=${encodeURIComponent(file.name)}`
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/images${query}`,
    { method: 'POST', headers, body: file },
  )
  if (!response.ok) {
    throw new RecordApiError(`Library image upload failed: ${response.status}`, response.status)
  }

  const payload = await response.json() as { url?: unknown }
  if (typeof payload.url !== 'string' || !payload.url) {
    throw new RecordApiError(t('errors.imageUploadNoUrl'), response.status)
  }
  return payload.url
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
  let repoId = target.databaseId
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
      { operatorId: target.operatorId, anonymous: target.anonymous }
    )
    const repoData = payload.repo
    if (!repoData) {
      return { items: [], properties: [], repoName }
    }
    if (page === 1) {
      properties = repoData.properties.map(normalizeLibraryProperty)
      repoName = target.repoName ?? repoData.name
      repoId = repoData.id || repoId
    }
    items.push(...repoData.dataList.items)
    totalPages = Math.max(page, repoData.dataList.paginator?.totalPages ?? page)
    page += 1
  } while (page <= totalPages)

  return { items, properties, repoName, ...(repoId ? { repoId } : {}) }
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

export interface LibraryRepositoryProfile {
  id: string
  name: string
  username: string
  orgUsername: string
  description: string | null
  isPublic: boolean
}

interface LibraryRestRepoResponse {
  id: string
  name: string
  username: string
  description?: string | null
  is_public: boolean
  organization_id: string
  org_username: string
}

/**
 * Repository metadata, read over REST rather than GraphQL on purpose: the
 * public view has to tell "this repository is private" (403) from "no such
 * repository" (404), and the GraphQL transport collapses every field error
 * into one 400-with-message.
 */
export async function fetchLibraryRepositoryProfile(
  target: LibraryRepoTarget
): Promise<LibraryRepositoryProfile> {
  const org = encodeURIComponent(target.org)
  const repo = encodeURIComponent(target.repo)
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${org}/${repo}`,
    { headers: await libraryRestHeaders(target.operatorId, { anonymous: target.anonymous }) }
  )
  if (!response.ok) {
    throw new RecordApiError(
      `Library repository request failed: ${response.status}`,
      response.status
    )
  }

  const payload = await response.json() as LibraryRestRepoResponse
  if (!payload || typeof payload.id !== 'string' || typeof payload.is_public !== 'boolean') {
    throw new RecordApiError(
      'Library repository returned an invalid response',
      response.status,
      'invalid-response'
    )
  }
  return {
    id: payload.id,
    name: payload.name,
    username: payload.username,
    orgUsername: payload.org_username,
    description: payload.description ?? null,
    isPublic: payload.is_public,
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
  // Repository discovery is scoped to the signed-in user's organizations. Without a
  // token there is nothing of the caller's own to show, and listing whatever the API
  // hands back would surface other people's repositories on the home screen.
  const token = await validLibraryAccessToken()
  if (!token) return []

  const restRepos = await fetchLibraryRestRepositories(token)

  try {
    const payload = await requestLibraryGraphQL<LibraryMeOrganizationsResponse>(
      libraryMeOrganizationsQuery,
      {}
    )
    updateStoredAuthFromLibraryUser(payload.me)
    // The API already returns only what Library treats as an organization.
    // Narrowing it again by platform here dropped every tenant adopted from
    // Tachyon, because an adopted tenant keeps the platform it came from.
    const organizations = payload.me?.organizations ?? []

    return Promise.all(
      organizations.map(async (organization) => {
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

export async function fetchLibraryAccessibleTenants(): Promise<LibraryAccessibleTenant[]> {
  const payload = await requestLibraryGraphQL<LibraryAccessibleTenantsResponse>(
    libraryAccessibleTenantsQuery,
    {}
  )
  return payload.accessibleTenants ?? []
}

/**
 * Registers an existing Tachyon tenant as a Library organization and grants the
 * Library policies to everyone already in that tenant. Safe to call twice: the
 * API returns the existing organization instead of failing.
 */
export async function importLibraryTenant(
  tenantId: string
): Promise<CreatedLibraryOrganization> {
  const payload = await requestLibraryGraphQL<LibrarySeedTenantResponse>(
    librarySeedTenantMutation,
    { tenantId: tenantId.trim() }
  )
  return payload.seedLibraryTenant.organization
}

export async function createLibraryRepository(
  input: CreateLibraryRepositoryInput
): Promise<CreatedLibraryRepository> {
  const payload = await requestLibraryGraphQL<LibraryCreateRepositoryResponse>(
    libraryCreateRepositoryMutation,
    {
      input: {
        orgUsername: input.orgUsername.trim(),
        repoName: input.name.trim(),
        repoUsername: input.username.trim(),
        userId: configuredLibraryActor(),
        isPublic: input.isPublic,
        description: input.description?.trim() || null,
      },
    },
    { operatorId: input.operatorId },
  )
  return payload.createRepo
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

async function fetchLibraryRestRepositories(
  token: string
): Promise<LibraryRestRepository[]> {
  return requestLibraryRestRepositories({
    'x-platform-id': configuredPlatformId(),
    'x-operator-id': import.meta.env.VITE_LIBRARY_OPERATOR_ID ?? configuredPlatformId(),
    Authorization: `Bearer ${token}`,
  })
}

async function requestLibraryRestRepositories(
  headers: Record<string, string>
): Promise<LibraryRestRepository[]> {
  const baseUrls = [
    configuredLibraryApiBaseUrl(),
    'https://library-api.txcloud.app',
  ].filter((url, index, urls) => urls.indexOf(url) === index)

  for (const baseUrl of baseUrls) {
    try {
      const response = await fetch(`${baseUrl}/v1beta/repos`, { headers })
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
    if (typeof record.richText === 'string') return { richText: record.richText }
    if (typeof record.rich_text === 'string') return { richText: record.rich_text }
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
    typ: normalizeLibraryPropertyType(property.property_type),
    meta: null,
  }
}

async function fetchLibraryRestProperties(target: LibraryRepoTarget): Promise<LibraryProperty[]> {
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/properties`,
    { headers: await libraryRestHeaders(target.operatorId, { anonymous: target.anonymous }) }
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
      { operatorId: target.operatorId, anonymous: target.anonymous }
    )
    if (!Array.isArray(payload.properties)) {
      throw new RecordApiError(
        'Library GraphQL returned no Property definitions',
        200,
        'invalid-response'
      )
    }
    return payload.properties.map(normalizeLibraryProperty)
  } catch (error: unknown) {
    if (!shouldFallbackLibraryRequest(error, 'read')) throw error
    return fetchLibraryRestProperties(target)
  }
}

async function fetchLibraryRestRepoTableData(
  target: LibraryRepoTarget
): Promise<LibraryRepoTableData> {
  const pageSize = configuredLibraryPageSize()
  const headers = await libraryRestHeaders(target.operatorId, { anonymous: target.anonymous })
  const baseUrl = configuredLibraryApiBaseUrl()
  const propertiesResponsePromise = fetch(
    `${baseUrl}/v1beta/repos/${target.org}/${target.repo}/properties`,
    { headers }
  )
  // Asked for alongside the data because the id names the collection these
  // rows are cached in, and GraphQL — which returns it for free — is by
  // definition unavailable on this path. Rejections are swallowed: a missing
  // id costs the local cache, not the listing.
  const repoResponsePromise = target.databaseId
    ? null
    : fetch(`${baseUrl}/v1beta/repos/${target.org}/${target.repo}`, { headers })
        .catch(() => null)
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
  const repoId = target.databaseId ?? await readRestRepositoryId(repoResponsePromise)
  return {
    items,
    properties,
    repoName: target.repoName ?? target.repo,
    ...(repoId ? { repoId } : {}),
  }
}

/** The id from a `GET /repos/{org}/{repo}` that was allowed to fail. */
async function readRestRepositoryId(
  response: Promise<Response | null> | null
): Promise<string | undefined> {
  const settled = await response
  if (!settled?.ok) return undefined
  try {
    const payload = await settled.json() as { id?: unknown }
    return typeof payload.id === 'string' && payload.id ? payload.id : undefined
  } catch {
    return undefined
  }
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

/**
 * Create the record at `dataId`, or apply the payload to the one already there.
 *
 * `PUT .../data/{id}` is update-only and answers 404 for an id the server has
 * never seen, which is the wrong answer for a caller that assigned the id
 * itself. `.../upsert` is the route that creates instead. See
 * `apps/api/src/handler/data.rs`.
 */
async function upsertLibraryRestData(
  dataId: string,
  dataName: string,
  target: LibraryRepoTarget,
  properties: LibraryProperty[],
  propertyData: LibraryDataItem['propertyData']
): Promise<LibraryDataItem> {
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${dataId}/upsert`,
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
    throw new RecordApiError(`Library REST data upsert failed: ${response.status}`, response.status)
  }
  return restDataToLibraryDataItem(await response.json() as LibraryRestDataResponse)
}

async function deleteLibraryRestData(
  dataId: string,
  target: LibraryRepoTarget
): Promise<void> {
  const response = await fetch(
    `${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data/${dataId}`,
    { method: 'DELETE', headers: await libraryRestHeaders(target.operatorId) }
  )
  // A record that is already gone is the outcome the caller wanted. Raising
  // here would turn a retried delete into a rejection.
  if (!response.ok && response.status !== 404) {
    throw new RecordApiError(`Library REST data delete failed: ${response.status}`, response.status)
  }
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
  const headers = await libraryRestHeaders(target.operatorId, { anonymous: target.anonymous })
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
  if (!org || !repo) throw new RecordApiError(t('errors.apiNotConfigured'), 400)

  const resolvedTarget = { org, repo, operatorId: target?.operatorId, anonymous: target?.anonymous }
  try {
    const payload = await requestLibraryGraphQL<LibraryDataResponse>(
      libraryDataDetailQuery,
      { org, repo, dataId },
      { operatorId: target?.operatorId, anonymous: target?.anonymous }
    )
    if (!payload.data) throw new RecordApiError(t('errors.dataNotFound'), 404)
    if (!Array.isArray(payload.properties)) {
      throw new RecordApiError(
        'Library GraphQL returned no Property definitions',
        200,
        'invalid-response'
      )
    }
    return {
      item: payload.data,
      properties: payload.properties.map(normalizeLibraryProperty),
    }
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

/**
 * Cache one repository's records, in that repository's own collection.
 *
 * `ingest` rather than `upsert`: the Library API owns these rows, so storing
 * one is not a local edit and must not enter the push queue. Writing them as
 * operations meant every cached row became pending, and the pending set is
 * scope-wide — so the next `documents` save tried to push the whole cache at
 * the Engine.
 */
async function cacheLibraryRecords(
  repository: LibraryRecordsRepository,
  records: readonly DatabaseRecord[]
): Promise<void> {
  await ingestClientEngineRecords(
    libraryRecordsCollection(repository.databaseId),
    records.map((record) => ({ recordId: record.id, value: record }))
  )
}

/** The one repository this build is pinned to, where it is pinned to one. */
function configuredRepoTarget(): LibraryRepoTarget | undefined {
  const org = import.meta.env.VITE_LIBRARY_ORG
  const repo = import.meta.env.VITE_LIBRARY_REPO
  if (!org || !repo) return undefined
  return { org, repo, operatorId: import.meta.env.VITE_LIBRARY_OPERATOR_ID }
}

function repoTargetFor(repository: LibraryRecordsRepository): LibraryRepoTarget {
  return {
    org: repository.org,
    repo: repository.repo,
    operatorId: repository.operatorId,
    repoName: repository.repoName,
    databaseId: repository.databaseId,
  }
}

/**
 * Learn a set of repositories — for this session, and for the next start.
 *
 * The in-memory half is what `resolveCollection` reads when it is asked what
 * a `data:` collection is. The stored half is what an offline start reads:
 * the collections are named after repositories, and without the names there
 * is no way to reach cached rows that are sitting right there on disk.
 */
async function rememberRepositories(
  known: readonly LibraryRecordsRepository[]
): Promise<void> {
  const changed = rememberLibraryRepositories(known)
  if (changed.length === 0) return
  await ingestClientEngineRecords(
    LIBRARY_REPOSITORIES_COLLECTION,
    changed.map((repository) => ({ recordId: repository.databaseId, value: repository }))
  )
}

/**
 * The repository a target names, with its canonical id resolved.
 *
 * The id is what the collection is named for, so a target without one has no
 * local home. The registry answers without a request every time but the first
 * sight of a repository; that one costs a `GET /repos/{org}/{repo}` and is
 * allowed to fail, because only the local cache depends on it.
 */
async function repositoryFor(
  target: LibraryRepoTarget
): Promise<LibraryRecordsRepository | undefined> {
  const known = target.databaseId
    ? {
        databaseId: target.databaseId,
        org: target.org,
        repo: target.repo,
        operatorId: target.operatorId,
        repoName: target.repoName,
      }
    : libraryRepositoryByName(target.org, target.repo)
  if (known) {
    await rememberRepositories([known])
    return known
  }

  try {
    const profile = await fetchLibraryRepositoryProfile(target)
    const learned: LibraryRecordsRepository = {
      databaseId: profile.id,
      org: target.org,
      repo: target.repo,
      operatorId: target.operatorId,
      repoName: target.repoName ?? profile.name,
    }
    await rememberRepositories([learned])
    return learned
  } catch {
    return undefined
  }
}

/**
 * The repository that owns a record: the one whose collection holds it.
 *
 * This is the whole point of naming collections after repositories. A write
 * used to rebuild its destination from `orgUsername` on the value, falling
 * back to a cached copy of the same value and then to the build's environment;
 * three guesses at something the key already knew. Here the answer is
 * structural — the record is in exactly one collection, and that collection
 * *is* a repository.
 */
async function repositoryOwning(
  recordId: string
): Promise<{ repository: LibraryRecordsRepository; record: DatabaseRecord } | undefined> {
  for (const repository of knownLibraryRepositories()) {
    const held = (
      await listClientEngineRecords<DatabaseRecord>(
        libraryRecordsCollection(repository.databaseId)
      )
    ).find((record) => record.recordId === recordId)
    if (held) return { repository, record: held.value }
  }
  return undefined
}

/** Every record this device has cached, across the repositories it knows. */
async function cachedLibraryRecords(): Promise<DatabaseRecord[]> {
  const collections = knownLibraryRepositories().map((repository) =>
    libraryRecordsCollection(repository.databaseId)
  )
  // On the one start after the collections were renamed, and only then, the
  // rows are still under the name every repository used to share.
  if (legacyRecordsCollectionPending()) {
    collections.push(LEGACY_LIBRARY_RECORDS_COLLECTION)
  }

  const byId = new Map<string, DatabaseRecord>()
  for (const collection of collections) {
    for (const cached of await listClientEngineRecords<DatabaseRecord>(collection)) {
      byId.set(cached.recordId, cached.value)
    }
  }
  return [...byId.values()]
}

/**
 * Read each repository's records into its own collection.
 *
 * Reading several repositories is the home screen wanting every repository,
 * not one listing that has to work out what it covers: each collection is
 * fetched, cached and reconciled on its own, and its `list()` is complete by
 * construction.
 */
async function fetchRepositoryRecords(
  targets: readonly LibraryRepoTarget[]
): Promise<DatabaseRecord[]> {
  const fetched = await Promise.all(
    targets.map(async (target) => {
      const table = await fetchLibraryRepoTableData(target)
      const records = table.items.map((item) =>
        libraryDataToRecord(item, table.properties, table.repoName, {
          orgUsername: target.org,
          repoUsername: target.repo,
          operatorId: target.operatorId,
        })
      )
      const repository: LibraryRecordsRepository | undefined = table.repoId
        ? {
            databaseId: table.repoId,
            org: target.org,
            repo: target.repo,
            operatorId: target.operatorId,
            repoName: table.repoName,
          }
        : undefined
      return { repository, records }
    })
  )

  const repositories = fetched
    .map((entry) => entry.repository)
    .filter((repository): repository is LibraryRecordsRepository => repository != null)
  await rememberRepositories(repositories)
  // Only now can the old shared collection be split up: routing its rows needs
  // the repositories they belong to, and this is where those become known.
  await carryLegacyLibraryRecords(knownLibraryRepositories())

  for (const entry of fetched) {
    if (entry.repository) await cacheLibraryRecords(entry.repository, entry.records)
  }
  return fetched.flatMap((entry) => entry.records)
}

export async function fetchServerRecords(): Promise<DatabaseRecord[]> {
  const configured = configuredRepoTarget()
  if (configured) {
    try {
      return await fetchRepositoryRecords([configured])
    } catch (error) {
      const cached = await cachedLibraryRecords()
      if (cached.length > 0) return cached
      throw error
    }
  }

  if (await validLibraryAccessToken()) {
    try {
      const repositories = await fetchLibraryRepositories()
      return await fetchRepositoryRecords(
        repositories.flatMap((repository) =>
          repository.orgUsername
            ? [{
                org: repository.orgUsername,
                repo: repository.username,
                operatorId: repository.operatorId,
                repoName: `${repository.orgUsername} / ${repository.name || repository.username}`,
                databaseId: repository.id,
              }]
            : []
        )
      )
    } catch (error) {
      const cached = await cachedLibraryRecords()
      if (cached.length > 0) return cached
      throw error
    }
  }

  const records = await listClientEngineRecords<DatabaseRecord>('records')
  if (import.meta.env.MODE === 'test') {
    return records.map((record) => record.value)
  }
  return []
}

/**
 * Check a write against the repository's schema before it enters the queue.
 *
 * This is the same mapping the resource runs at push time, run early. Without
 * it a record whose status has no Property to live in is accepted on screen,
 * queued, and only rejected once the push reaches the server — and the message
 * that explains what is wrong with the repository's schema arrives detached
 * from the edit that caused it. `RecordPropertyMappingError` is a 422, so
 * Photon would roll the write back correctly; it is *where* the user hears
 * about it that this fixes.
 *
 * Best-effort on purpose. When the Properties cannot be read the write is
 * still queued: "the network is down" must not become "you may not edit this
 * record", which is the case this stage exists to serve.
 */
async function assertRecordFieldsMappable(
  target: LibraryRepoTarget,
  data: ServerCreateRecordData | ServerUpdateRecordData
): Promise<void> {
  let properties: LibraryProperty[]
  try {
    properties = await fetchLibraryRepoProperties(target)
  } catch {
    return
  }
  restPropertyData(properties, standardRecordPropertyData(properties, data))
}

/**
 * A record the engine put back after this module had already returned it.
 *
 * `record` is where the record now stands locally: `null` when the write that
 * was rolled back was the record's creation, or when the rollback restored it
 * to deleted.
 */
export interface RecordRollback {
  recordId: string
  record: DatabaseRecord | null
}

/**
 * Hear about writes the server refused only after they were reported queued.
 *
 * `settleRecordWrite` turns an *immediate* rejection into a thrown
 * `RecordApiError`, which is how the caller learns to undo what it drew. A
 * queued write has no rejection to throw yet — it is decided on a later sync
 * cycle, once the network is back, and by then the call that made it is long
 * gone. Photon rolls its own projection back and nothing downstream hears;
 * this is the seam through which it does.
 *
 * Scoped to the records collections, because that is what these callers hold.
 * A rollback in `documents` or `attachments` belongs to a different projection
 * and is not this listener's to reconcile.
 */
export function subscribeRecordRollbacks(
  listener: (rollbacks: readonly RecordRollback[]) => void
): () => void {
  return subscribeClientEngineRollbacks((changes) => {
    const rollbacks = changes
      .filter((change) => libraryRecordsDatabaseId(change.collection) != null)
      .map((change) => ({
        recordId: change.recordId,
        record: (change.record?.value as DatabaseRecord | undefined) ?? null,
      }))
    if (rollbacks.length > 0) listener(rollbacks)
  })
}

/**
 * What became of a write that was reported as queued.
 *
 * `rejected` is not here: it travels the rollback seam above, which reports
 * the rolled-back value the moment Photon reprojects it rather than waiting
 * for the cycle to end.
 *
 * `record` is where the record now stands. `null` under `conflict` means the
 * server has no such record; under `accepted` it means the record is no longer
 * under the id it was written with — the server minted its own and Photon
 * moved it — and nothing here knows the new one.
 */
export interface RecordSettlement {
  status: 'accepted' | 'conflict'
  recordId: string
  record: DatabaseRecord | null
}

/**
 * Hear what a queued write settled as, beyond the ones that were undone.
 *
 * A rejection is the loud case and `subscribeRecordRollbacks` has it. These
 * are the quiet ones, and they leave a projection built from the write's
 * return value just as wrong: a `conflict` puts the record back to the
 * server's value without rolling anything back, and an `accepted` create
 * carries the server-derived `identifier` that the queued write could only
 * guess at.
 */
export function subscribeRecordSettlements(
  listener: (settlement: RecordSettlement) => void
): () => void {
  return subscribeClientEngineSettlements((settlement) => {
    if (settlement.status === 'rejected') return
    if (libraryRecordsDatabaseId(settlement.collection) == null) return
    listener({
      status: settlement.status,
      recordId: settlement.recordId,
      record: (settlement.record?.value as DatabaseRecord | undefined) ?? null,
    })
  })
}

/**
 * Turn Photon's verdict into what this module's callers already expect.
 *
 * `queued` is deliberately not an error. The operation is durable, the record
 * is on screen, and the push will go out when the network comes back — that is
 * the whole reason records moved onto the engine.
 *
 * `conflict` is. Photon has put the record back to the server's value and kept
 * the local one on a conflict row, so returning it would hand the caller a
 * record that is not what it asked for while its mutation error was cleared —
 * the UI would show a successful save of a change that is not there. Until
 * there is a resolution flow to send the user to, saying so is the honest
 * answer, and the row is still on `listClientEngineConflicts` for one.
 */
function settleRecordWrite(
  outcome: ClientEngineWriteResult<DatabaseRecord>,
  fallback: DatabaseRecord
): DatabaseRecord {
  if (outcome.status === 'rejected') {
    throw new RecordApiError(
      outcome.reason ?? 'The server rejected this change',
      422,
      'http'
    )
  }
  if (outcome.status === 'conflict') {
    throw new RecordApiError(
      outcome.reason ?? 'This record changed elsewhere while you were editing it',
      409,
      'http'
    )
  }
  return outcome.record?.value ?? fallback
}

/**
 * Put a record the projection has never held where a patch can merge into it.
 *
 * Photon's `patch` merges into the local value, so an absent base would push a
 * record made of nothing but the changed fields. A caller can legitimately
 * reach an id the projection does not have — a deep link, a lazy collection
 * nothing has queried, a tool acting on an id it was handed — so read it once
 * and ingest it rather than refusing the edit.
 *
 * Returns `undefined` when the record cannot be read at all, which the caller
 * reports as the 404 it is.
 */
async function seedLibraryRecord(
  recordId: string,
  repository: LibraryRecordsRepository,
  target: LibraryRepoTarget
): Promise<DatabaseRecord | undefined> {
  let detail: { item: LibraryDataItem; properties: LibraryProperty[] }
  try {
    detail = await fetchLibraryDataDetail(recordId, target)
  } catch {
    return undefined
  }
  const record = libraryDataToRecord(
    detail.item,
    detail.properties,
    target.repoName ?? target.repo,
    {
      orgUsername: target.org,
      repoUsername: target.repo,
      operatorId: target.operatorId,
    }
  )
  await cacheLibraryRecords(repository, [record])
  return record
}

/**
 * Create a record by naming it first and telling the server second.
 *
 * The id is minted here, not by the API, which is what makes this work offline
 * and makes the created record navigable in the same tick. `data_` is not
 * decoration: library-api parses a `DataId` by its prefix and rejects anything
 * else. The write goes out as an `upsert` operation so Photon routes it to
 * `PUT .../data/{id}/upsert` — the create-or-update route — rather than
 * guessing create-vs-update from a projection that already holds the
 * optimistic value.
 *
 * A create names its own destination: the screen it was made from knows which
 * repository it is showing. The environment only stands in for a build pinned
 * to a single repository.
 */
export async function createServerRecord(data: ServerCreateRecordData): Promise<DatabaseRecord> {
  const requested: LibraryRepoTarget | undefined =
    data.orgUsername && data.repoUsername
      ? { org: data.orgUsername, repo: data.repoUsername, operatorId: data.operatorId }
      : undefined
  const destination = requested ?? configuredRepoTarget()
  const repository =
    destination && (libraryApiConfigured() || (await validLibraryAccessToken()))
      ? await repositoryFor({ ...destination, repoName: data.project ?? destination.repo })
      : undefined

  if (destination && repository) {
    const target: LibraryRepoTarget = {
      ...repoTargetFor(repository),
      repoName: data.project ?? repository.repoName ?? destination.repo,
    }
    await assertRecordFieldsMappable(target, data)

    const recordId = newClientEngineRecordId('data')
    const now = new Date().toISOString()
    const record: DatabaseRecord = {
      id: recordId,
      // The server derives this from an Id Property when the repository has
      // one, and otherwise echoes the record id back. The pull at the end of
      // the push replaces this with whatever it assigned.
      identifier: recordId,
      title: data.title,
      status: data.status ?? 'todo',
      priority: data.priority ?? 'none',
      assignee: data.assignee ?? null,
      labels: data.labels ?? [],
      project: data.project ?? target.repoName ?? target.repo,
      createdAt: now,
      updatedAt: now,
      description: data.description ?? '',
      orgUsername: target.org,
      repoUsername: target.repo,
      operatorId: target.operatorId,
    }

    return settleRecordWrite(
      await upsertAndPushClientEngineRecord<DatabaseRecord>(
        libraryRecordsCollection(repository.databaseId),
        recordId,
        record
      ),
      record
    )
  }

  const records = (await listClientEngineRecords<DatabaseRecord>('records')).map((record) => record.value)
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
  const storedRecord = await upsertClientEngineRecord('records', record.id, record)
  return storedRecord.value
}

/**
 * Edit a record optimistically, then push the edit.
 *
 * A `patch` operation, so only the fields that changed travel; the resource
 * re-reads the record and merges before it PUTs, because library-api's update
 * body carries the whole name and Property set.
 *
 * The repository comes from the collection holding the record, not from a
 * fallback chain over the value — that is what #285 moved up into the key.
 */
export async function updateServerRecord(
  recordId: string,
  data: ServerUpdateRecordData
): Promise<DatabaseRecord> {
  const owner = await repositoryOwning(recordId)
  const destination = owner ? repoTargetFor(owner.repository) : configuredRepoTarget()
  const repository =
    destination && (libraryApiConfigured() || (await validLibraryAccessToken()))
      ? owner?.repository ?? (await repositoryFor(destination))
      : undefined

  if (destination && repository) {
    const target: LibraryRepoTarget = {
      ...repoTargetFor(repository),
      repoName: data.project ?? owner?.record.project ?? repository.repoName ?? destination.repo,
    }
    const collection = libraryRecordsCollection(repository.databaseId)
    const existing =
      owner?.record ?? (await seedLibraryRecord(recordId, repository, target))
    if (!existing) {
      throw new RecordApiError(t('errors.recordNotFound'), 404)
    }

    await assertRecordFieldsMappable(target, data)

    const fields: Partial<DatabaseRecord> = {
      ...withoutUndefined(data),
      ...(data.assignee === undefined ? {} : { assignee: data.assignee }),
      ...(data.labels === undefined ? {} : { labels: data.labels }),
      updatedAt: new Date().toISOString(),
    }
    return settleRecordWrite(
      await patchAndPushClientEngineRecord<DatabaseRecord>(collection, recordId, fields),
      { ...existing, ...fields }
    )
  }

  const existing = (await listClientEngineRecords<DatabaseRecord>('records'))
    .find((record) => record.recordId === recordId)?.value
  if (!existing) {
    throw new RecordApiError(t('errors.recordNotFound'), 404)
  }

  const record: DatabaseRecord = {
    ...existing,
    ...withoutUndefined(data),
    assignee: data.assignee === undefined ? existing.assignee : data.assignee,
    labels: data.labels ?? existing.labels,
    updatedAt: new Date().toISOString(),
  }
  const storedRecord = await patchClientEngineRecord<DatabaseRecord>('records', recordId, record)
  if (!storedRecord) throw new RecordApiError(t('errors.recordNotFound'), 404)
  return storedRecord.value
}

/**
 * Delete a record through the engine.
 *
 * A `delete` operation rather than an API call plus a tombstone: the operation
 * *is* the DELETE, queued like any other write, so a delete made offline
 * survives and a delete the server refuses comes back.
 */
export async function deleteServerRecord(recordId: string): Promise<void> {
  const owner = await repositoryOwning(recordId)
  const destination = owner ? repoTargetFor(owner.repository) : configuredRepoTarget()
  const repository =
    destination && (libraryApiConfigured() || (await validLibraryAccessToken()))
      ? owner?.repository ?? (await repositoryFor(destination))
      : undefined

  if (destination && repository) {
    const collection = libraryRecordsCollection(repository.databaseId)
    if (!owner) {
      // Nothing local to delete. Seed it so the operation has a record to
      // remove, and so a rejection has something to roll back to.
      const seeded = await seedLibraryRecord(recordId, repository, repoTargetFor(repository))
      if (!seeded) return
    }
    const outcome = await deleteAndPushClientEngineRecord<DatabaseRecord>(collection, recordId)
    if (outcome.status === 'rejected') {
      throw new RecordApiError(
        outcome.reason ?? 'The server rejected this deletion',
        422,
        'http'
      )
    }
    if (outcome.status === 'conflict') {
      throw new RecordApiError(
        outcome.reason ?? 'This record changed elsewhere while you were deleting it',
        409,
        'http'
      )
    }
    return
  }

  await deleteClientEngineRecord('records', recordId)
}

/**
 * The records collection as a Photon `rest-backed` resource.
 *
 * Every method goes through the same base URL, auth headers and
 * `RecordApiError` the rest of this module uses, which is what makes the
 * adapter useful: `RecordApiError.status` is the one field Photon's
 * `decisionForError` reads, so a 400 becomes a rejection and a 409 a conflict
 * without anything here knowing about the engine.
 *
 * `upsert` is the reason this exists. Photon cannot tell a first write from an
 * edit — it writes the optimistic value into the projection before the push
 * runs, so both look identical from its side — and a resource without `upsert`
 * relies on the backend tolerating an update to an id it has never seen.
 * library-api does not: `PUT .../data/{id}` is update-only and 404s, which
 * `decisionForError` maps to `rejected`, dropping the user's new record. With
 * `upsert` present Photon routes upsert operations here and skips the guess.
 */
export interface LibraryRecordsResource extends RestResource<DatabaseRecord> {
  list(): Promise<RestListResult<DatabaseRecord>>
  create(value: DatabaseRecord): Promise<DatabaseRecord>
  /**
   * Required here, optional on `RestResource`. Photon leaves it optional
   * because a backend may not have PUT-style semantics; library-api does, so
   * a caller of this resource never has to check.
   */
  upsert(recordId: string, value: DatabaseRecord): Promise<DatabaseRecord>
  update(recordId: string, fields: Partial<DatabaseRecord>): Promise<DatabaseRecord>
}

/**
 * Hand `photonEngine` the builder for a repository's records collection.
 *
 * One resource per collection, closing over the one repository it is named
 * for. That is what makes `list()` unambiguous — it is a repository's data
 * list, not a search across every repository for rows that claim to belong
 * here — and what leaves a write with no destination to work out.
 *
 * The cast is the erasure Photon's `CollectionConfig` uses: `RestResource<never>`
 * is how it holds resources of collections whose value types it cannot name.
 */
setLibraryRecordsResourceFactory((repository) =>
  createLibraryRecordsResource(
    repoTargetFor(repository)
  ) as unknown as RestResource<never>
)

export function createLibraryRecordsResource(
  target: LibraryRepoTarget
): LibraryRecordsResource {
  const toDatabaseRecord = (
    item: LibraryDataItem,
    properties: LibraryProperty[]
  ): DatabaseRecord =>
    libraryDataToRecord(item, properties, target.repoName ?? target.repo, {
      orgUsername: target.org,
      repoUsername: target.repo,
      operatorId: target.operatorId,
    })

  return {
    async list(): Promise<RestListResult<DatabaseRecord>> {
      // GraphQL first, REST as the fallback — the same read the rest of this
      // module does, and not an inconsistency with the `rest-backed` mode.
      // Photon's contract for a resource is "the application's own HTTP calls,
      // wrapped"; it is the *writes* that have to be REST, because that is
      // where the operation log, the rollback and the conflict row live.
      //
      // Reading over REST alone is in fact wrong here. `GET .../properties`
      // returns a Property's id, name and type but not its Select options, so
      // an option id in a record cannot be resolved to the option — and every
      // record comes back with the default status and priority instead of the
      // ones it has. The GraphQL `properties` query carries the options.
      //
      // `complete` is what lets Photon delete records that are gone upstream,
      // so it may only be true for the whole collection. Both paths page until
      // the paginator runs out, so it is.
      const table = await fetchLibraryRepoTableData(target)
      return {
        items: table.items.map((item) =>
          libraryDataToRecord(item, table.properties, table.repoName, {
            orgUsername: target.org,
            repoUsername: target.repo,
            operatorId: target.operatorId,
          })
        ),
        complete: true,
      }
    },

    async create(value: DatabaseRecord): Promise<DatabaseRecord> {
      const properties = await fetchLibraryRepoProperties(target)
      const created = await createLibraryRestData(
        value,
        target,
        properties,
        standardRecordPropertyData(properties, value, { requireProperty: false })
      )
      // The id comes back different from the local one: `POST /data` mints its
      // own. Returning the item is what lets Photon record the alias.
      return toDatabaseRecord(created, properties)
    },

    async upsert(recordId: string, value: DatabaseRecord): Promise<DatabaseRecord> {
      const properties = await fetchLibraryRepoProperties(target)
      const stored = await upsertLibraryRestData(
        recordId,
        value.title,
        target,
        properties,
        standardRecordPropertyData(properties, value, { requireProperty: false })
      )
      return toDatabaseRecord(stored, properties)
    },

    async update(
      recordId: string,
      fields: Partial<DatabaseRecord>
    ): Promise<DatabaseRecord> {
      // Read first for the same reason `updateServerRecord` does: the PUT body
      // carries the whole name and the merged property set, neither of which a
      // patch of arbitrary fields contains. This is not a create-or-update
      // guess — a missing record still 404s here, and it must, so Photon can
      // reject the write and let the next pull reconcile.
      const existing = await fetchLibraryDataDetail(recordId, target)
      const propertyData = standardRecordPropertyData(
        existing.properties,
        fields,
        { requireProperty: false }
      ).reduce(
        (item, entry) => mergeLibraryDataProperty(item, entry.propertyId, entry.value),
        existing.item
      ).propertyData
      const updated = await updateLibraryRestData(
        recordId,
        fields.title ?? existing.item.name,
        target,
        existing.properties,
        propertyData
      )
      return toDatabaseRecord(updated, existing.properties)
    },

    async remove(recordId: string): Promise<void> {
      await deleteLibraryRestData(recordId, target)
    },

    /**
     * Where a server-assigned id becomes the record's identity.
     *
     * `create` returns the record under the id library-api minted, not the one
     * the client wrote optimistically, and this is the only place Photon learns
     * that — the returned `recordId` is what it stores as `aliasRecordId`. Keying
     * off anything but `item.id` would leave the alias pointing at the local id
     * and the next pull would insert the server's row as a second record.
     */
    toRecord(item: DatabaseRecord) {
      return { recordId: item.id, value: item }
    },
  }
}
