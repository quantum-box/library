import { appKitConfig } from '../app/kitConfig.js'
import { type DatabaseRecord, type Priority, type Status } from '../data/mock'
import {
  getLibraryDataPropertyValue,
  propertyValueList,
  propertyValueText,
} from './libraryTable/libraryPropertyFormat'
import {
  deleteClientEngineRecord,
  listClientEngineRecords,
  patchClientEngineRecord,
  upsertClientEngineRecord,
} from './photonEngine/client'

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
    }
    properties: LibraryProperty[]
  } | null
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
  query LibraryClientRepoData($org: String!, $repo: String!, $pageSize: Int) {
    repo(orgUsername: $org, repoUsername: $repo) {
      id
      name
      dataList(pageSize: $pageSize) {
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

export class RecordApiError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'RecordApiError'
    this.status = status
  }
}

export function libraryApiConfigured(): boolean {
  return Boolean(import.meta.env.VITE_LIBRARY_ORG && import.meta.env.VITE_LIBRARY_REPO)
}

export function libraryOrgConfigured(): boolean {
  return Boolean(import.meta.env.VITE_LIBRARY_ORG)
}

function normalizeStatus(value: string | undefined): Status {
  if (!value) return 'backlog'
  const normalized = normalizedPropertyName(value)
  return statusAliases[normalized] ?? (statuses.includes(value as Status) ? (value as Status) : 'backlog')
}

function normalizePriority(value: string | undefined): Priority {
  if (!value) return 'none'
  const normalized = normalizedPropertyName(value)
  return priorityAliases[normalized] ?? (priorities.includes(value as Priority) ? (value as Priority) : 'none')
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

function libraryAccessToken(): string | undefined {
  if (import.meta.env.VITE_LIBRARY_ACCESS_TOKEN) {
    return import.meta.env.VITE_LIBRARY_ACCESS_TOKEN
  }

  if (typeof localStorage === 'undefined') {
    return undefined
  }

  try {
    const stored = localStorage.getItem('library_auth')
    if (!stored) return undefined
    const parsed = JSON.parse(stored) as { accessToken?: string }
    return parsed.accessToken
  } catch {
    return undefined
  }
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

function updateStoredAuthFromLibraryUser(user: { id: string; email?: string | null } | null | undefined) {
  if (!user || typeof localStorage === 'undefined') return

  try {
    const stored = localStorage.getItem('library_auth')
    if (!stored) return
    const parsed = JSON.parse(stored) as { userId?: string; email?: string | null }
    const next = {
      ...parsed,
      userId: user.id,
      email: user.email ?? parsed.email ?? '',
    }
    if (next.userId !== parsed.userId || next.email !== parsed.email) {
      localStorage.setItem('library_auth', JSON.stringify(next))
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
  const token = libraryAccessToken()
  if (token) headers.Authorization = `Bearer ${token}`

  const response = await fetch(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ query, variables }),
  })
  if (!response.ok) {
    throw new RecordApiError(`Library GraphQL request failed: ${response.status}`, response.status)
  }

  const payload = await response.json() as {
    data?: TData
    errors?: Array<{ message?: string }>
  }
  if (payload.errors?.length) {
    throw new RecordApiError(payload.errors[0]?.message ?? 'Library GraphQL request failed', 500)
  }
  return payload.data as TData
}

function normalizedPropertyName(value: string): string {
  return value.trim().toLowerCase().replace(/[\s_-]+/g, '')
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

export async function fetchLibraryRepoTableData(
  target: LibraryRepoTarget
): Promise<LibraryRepoTableData> {
  try {
    const payload = await requestLibraryGraphQL<LibraryRepoDataResponse>(
      libraryRepoDataQuery,
      {
        org: target.org,
        repo: target.repo,
        pageSize: Number(import.meta.env.VITE_LIBRARY_PAGE_SIZE ?? 100),
      },
      { operatorId: target.operatorId }
    )
    const repoData = payload.repo
    if (!repoData) {
      return { items: [], properties: [], repoName: target.repoName ?? target.repo }
    }
    return {
      items: repoData.dataList.items,
      properties: repoData.properties,
      repoName: target.repoName ?? repoData.name,
    }
  } catch {
    return fetchLibraryRestRepoTableData(target)
  }
}

export async function fetchLibraryRecords(target?: LibraryRepoTarget): Promise<DatabaseRecord[]> {
  const org = target?.org ?? import.meta.env.VITE_LIBRARY_ORG
  const repo = target?.repo ?? import.meta.env.VITE_LIBRARY_REPO
  if (!org || !repo) return []

  try {
    const payload = await requestLibraryGraphQL<LibraryRepoDataResponse>(
      libraryRepoDataQuery,
      {
        org,
        repo,
        pageSize: Number(import.meta.env.VITE_LIBRARY_PAGE_SIZE ?? 100),
      },
      { operatorId: target?.operatorId }
    )
    const repoData = payload.repo
    if (!repoData) return []
    return repoData.dataList.items.map((item) => libraryDataToRecord(
      item,
      repoData.properties,
      target?.repoName ?? repoData.name,
      { orgUsername: org, repoUsername: repo, operatorId: target?.operatorId }
    ))
  } catch {
    return fetchLibraryRestRecords({
      org,
      repo,
      operatorId: target?.operatorId,
      repoName: target?.repoName,
    })
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
  const repos = (payload.organization?.repos ?? []).map((repo) => ({
    ...repo,
    orgUsername: org,
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
    },
  ]
}

export async function fetchLibraryOrganizations(): Promise<LibraryOrganization[]> {
  const restRepos = await fetchLibraryRestRepositories()
  const token = libraryAccessToken()
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

async function fetchTachyonOperator(tenantId: string): Promise<TachyonOperatorResponse | null> {
  const token = libraryAccessToken()
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
  const token = libraryAccessToken()
  if (token) headers.Authorization = `Bearer ${token}`

  return requestLibraryRestRepositories(headers)
}

async function requestLibraryRestRepositories(
  headers?: Record<string, string>
): Promise<LibraryRestRepository[]> {
  const baseUrls = [
    configuredLibraryApiBaseUrl(),
    'https://library.api.n1.tachy.one',
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

function restPropertyValueText(value: LibraryRestPropertyData['value']): string | undefined {
  if (value == null) return undefined
  if (typeof value === 'string' || typeof value === 'number') return String(value)
  if (Array.isArray(value)) return value.join(', ')
  if (typeof value === 'object') {
    const candidate = Object.values(value)[0]
    if (typeof candidate === 'string' || typeof candidate === 'number') return String(candidate)
    if (Array.isArray(candidate)) return candidate.join(', ')
  }
  return undefined
}

function restDataToRecord(
  item: LibraryRestDataResponse,
  repoName: string,
  source?: { orgUsername?: string; repoUsername?: string; operatorId?: string }
): DatabaseRecord {
  const byName = new Map<string, string>()
  item.items.forEach((propertyData) => {
    const text = restPropertyValueText(propertyData.value)
    if (text) byName.set(normalizedPropertyName(propertyData.key), text)
  })
  const textValue = (...names: string[]) => {
    for (const name of names) {
      const value = byName.get(normalizedPropertyName(name))
      if (value) return value
    }
    return undefined
  }
  const listValue = (...names: string[]) => {
    const value = textValue(...names)
    return value ? value.split(',').map((item) => item.trim()).filter(Boolean) : []
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
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    description: textValue('description', 'body', 'content', 'markdown', 'html') ?? '',
    orgUsername: source?.orgUsername,
    repoUsername: source?.repoUsername,
    operatorId: source?.operatorId,
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
    if (typeof record.number === 'string' || typeof record.number === 'number') {
      return { number: String(record.number) }
    }
    if (typeof record.html === 'string') return { html: record.html }
    if (typeof record.markdown === 'string') return { markdown: record.markdown }
    if (typeof record.date === 'string') return { date: record.date }
    if (typeof record.url === 'string') return { url: record.url }
    if (typeof record.id === 'string') return { id: record.id }
    if (typeof record.optionId === 'string') return { optionId: record.optionId }
    if (typeof record.option_id === 'string') return { optionId: record.option_id }
    if (Array.isArray(record.optionIds)) {
      return { optionIds: record.optionIds.map((item) => String(item)) }
    }
    if (Array.isArray(record.option_ids)) {
      return { optionIds: record.option_ids.map((item) => String(item)) }
    }
    if (Array.isArray(record.dataIds)) {
      return { dataIds: record.dataIds.map((item) => String(item)) }
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

async function fetchLibraryRestRepoTableData(
  target: LibraryRepoTarget
): Promise<LibraryRepoTableData> {
  const limit = Number(import.meta.env.VITE_LIBRARY_PAGE_SIZE ?? 100)
  const headers = libraryRestHeaders(target.operatorId)
  const baseUrl = configuredLibraryApiBaseUrl()
  const [dataResponse, propertiesResponse] = await Promise.all([
    fetch(`${baseUrl}/v1beta/repos/${target.org}/${target.repo}/data-list?limit=${limit}`, { headers }),
    fetch(`${baseUrl}/v1beta/repos/${target.org}/${target.repo}/properties`, { headers }),
  ])
  if (!dataResponse.ok) {
    throw new RecordApiError(`Library REST data list failed: ${dataResponse.status}`, dataResponse.status)
  }
  const dataPayload = await dataResponse.json() as LibraryRestDataListResponse
  let properties: LibraryProperty[] = []
  if (propertiesResponse.ok) {
    const propertiesPayload = await propertiesResponse.json() as LibraryRestPropertyResponse[]
    properties = (Array.isArray(propertiesPayload) ? propertiesPayload : []).map(restPropertyToLibraryProperty)
  }
  return {
    items: (dataPayload.data ?? []).map(restDataToLibraryDataItem),
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

async function createLibraryRestRecord(
  data: ServerCreateRecordData,
  target: LibraryRepoTarget
): Promise<DatabaseRecord> {
  const response = await fetch(`${configuredLibraryApiBaseUrl()}/v1beta/repos/${target.org}/${target.repo}/data`, {
    method: 'POST',
    headers: libraryRestHeaders(target.operatorId),
    body: JSON.stringify({
      name: data.title,
      property_data: [],
    }),
  })
  if (!response.ok) throw new RecordApiError(`Library REST data create failed: ${response.status}`, response.status)
  const payload = await response.json() as LibraryRestDataResponse
  return restDataToRecord(
    payload,
    target.repoName ?? data.project ?? target.repo,
    { orgUsername: target.org, repoUsername: target.repo, operatorId: target.operatorId }
  )
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

async function fetchLibraryDataDetail(dataId: string, target?: Partial<LibraryRepoTarget>): Promise<{
  item: LibraryDataItem
  properties: LibraryProperty[]
}> {
  const org = target?.org ?? import.meta.env.VITE_LIBRARY_ORG
  const repo = target?.repo ?? import.meta.env.VITE_LIBRARY_REPO
  if (!org || !repo) throw new RecordApiError('Library API is not configured', 400)

  const payload = await requestLibraryGraphQL<LibraryDataResponse>(
    libraryDataDetailQuery,
    { org, repo, dataId },
    { operatorId: target?.operatorId }
  )
  if (!payload.data) throw new RecordApiError('Data not found', 404)
  return { item: payload.data, properties: payload.properties ?? [] }
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

  if (libraryAccessToken()) {
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

  let records = await listClientEngineRecords<DatabaseRecord>('records')
  if (import.meta.env.MODE === 'test') {
    return records.map((record) => record.value)
  }
  return []
}

export async function createServerRecord(data: ServerCreateRecordData): Promise<DatabaseRecord> {
  const targetOrg = data.orgUsername ?? import.meta.env.VITE_LIBRARY_ORG
  const targetRepo = data.repoUsername ?? import.meta.env.VITE_LIBRARY_REPO
  const targetOperatorId = data.operatorId ?? import.meta.env.VITE_LIBRARY_OPERATOR_ID
  if (targetOrg && targetRepo && (libraryApiConfigured() || libraryAccessToken())) {
    try {
      return await createLibraryRestRecord(data, {
        org: targetOrg,
        repo: targetRepo,
        operatorId: targetOperatorId,
        repoName: data.project ?? targetRepo,
      })
    } catch {
      // Fall back to GraphQL for deployments that have not enabled the REST write path.
    }

    const payload = await requestLibraryGraphQL<LibraryAddDataResponse>(
      libraryAddDataMutation,
      {
        input: {
          actor: configuredLibraryActor(),
          orgUsername: targetOrg,
          repoUsername: targetRepo,
          dataName: data.title,
          propertyData: [],
        },
      },
      { operatorId: targetOperatorId }
    )
    if (!payload.addData) throw new RecordApiError('Library API did not return created data', 500)

    const libraryRecords = await fetchLibraryRecords({
      org: targetOrg,
      repo: targetRepo,
      operatorId: targetOperatorId,
      repoName: data.project ?? targetRepo,
    })
    await Promise.all(
      libraryRecords.map((record) => upsertClientEngineRecord(libraryRecordsCollection, record.id, record))
    )
    return libraryRecords.find((record) => record.id === payload.addData?.id) ??
      libraryDataToRecord(payload.addData, [], data.project ?? targetRepo ?? 'Library', {
        orgUsername: targetOrg,
        repoUsername: targetRepo,
        operatorId: targetOperatorId,
      })
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
  if (targetOrg && targetRepo && (libraryApiConfigured() || libraryAccessToken())) {
    const existing = await fetchLibraryDataDetail(recordId, {
      org: targetOrg,
      repo: targetRepo,
      operatorId: targetOperatorId,
    })
    const nextName = data.title ?? existing.item.name
    const payload = await requestLibraryGraphQL<LibraryUpdateDataResponse>(
      libraryUpdateDataMutation,
      {
        input: {
          actor: configuredLibraryActor(),
          orgUsername: targetOrg,
          repoUsername: targetRepo,
          dataId: recordId,
          dataName: nextName,
          propertyData: existing.item.propertyData,
        },
      },
      { operatorId: targetOperatorId }
    )
    if (!payload.updateData) throw new RecordApiError('Library API did not return updated data', 500)
    const record = libraryDataToRecord(payload.updateData, existing.properties, data.project ?? targetRepo ?? 'Library', {
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
  if (targetOrg && targetRepo && (libraryApiConfigured() || libraryAccessToken())) {
    const payload = await requestLibraryGraphQL<LibraryDeleteDataResponse>(
      libraryDeleteDataMutation,
      { org: targetOrg, repo: targetRepo, dataId: recordId },
      { operatorId: targetOperatorId }
    )
    if (!payload.deleteData) throw new RecordApiError('Library API did not delete data', 500)
    await deleteClientEngineRecord(libraryRecordsCollection, recordId)
    return
  }

  await deleteClientEngineRecord(activeRecordsCollection(), recordId)
}
