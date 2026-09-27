import {
  configuredLibraryApiBaseUrl,
  libraryGraphqlHeaders,
} from './libraryGraphql'
import { t } from '../i18n'

export const repositoryPropertyTypes = [
  'STRING',
  'INTEGER',
  'MARKDOWN',
  'IMAGE',
  'SELECT',
  'MULTI_SELECT',
  'RELATION',
  'LOCATION',
  'DATE',
  'ID',
  'HTML',
  'RICH_TEXT',
  'BOOLEAN',
] as const

export type RepositoryPropertyType = (typeof repositoryPropertyTypes)[number]

export interface RepositoryPropertyOption {
  id?: string
  key: string
  name: string
}

export interface RepositoryPropertyMeta {
  __typename?: 'IdType' | 'JsonType' | 'MultiSelectType' | 'RelationType' | 'SelectType'
  autoGenerate?: boolean
  databaseId?: string
  json?: string
  options?: RepositoryPropertyOption[]
}

export interface RepositoryPropertyDefinition {
  id: string
  name: string
  displayName?: string
  /** Preserve future server-side Property types even when this client cannot edit them. */
  typ: RepositoryPropertyType | string
  meta?: RepositoryPropertyMeta | null
}

export interface RepositoryPolicy {
  userId: string
  role: string
}

export interface RepositorySettingsData {
  repository: {
    id: string
    name: string
    username: string
    description?: string | null
    isPublic: boolean
  }
  properties: RepositoryPropertyDefinition[]
  policies: RepositoryPolicy[]
}

export interface RepositorySettingsTarget {
  orgUsername: string
  repoUsername: string
  operatorId?: string
}

export interface RichTextTemplate {
  id: string
  name: string
  richText: string
}

export interface RichTextTemplateDraft {
  name: string
  richText: string
}

export interface RepositoryPropertyDraft {
  name: string
  displayName?: string
  type: RepositoryPropertyType
  options?: Array<{
    /** Existing server-issued option ID. Omit for a newly created option. */
    id?: string
    identifier: string
    label: string
  }>
  relationDatabaseId?: string
  autoGenerateId?: boolean
}

export interface RepositoryMetadataUpdate {
  /** Omit to leave the description unchanged. The current GraphQL resolver
   * cannot distinguish explicit null from an omitted Option value. */
  description?: string
  isPublic: boolean
}

export type RepositorySettingsErrorKind =
  | 'transport'
  | 'http'
  | 'graphql'
  | 'invalid-response'
  | 'permission'
  | 'not-found'
  | 'validation'

export class RepositorySettingsApiError extends Error {
  readonly status: number
  readonly kind: RepositorySettingsErrorKind

  constructor(message: string, status: number, kind: RepositorySettingsErrorKind) {
    super(message)
    this.name = 'RepositorySettingsApiError'
    this.status = status
    this.kind = kind
  }
}

interface GraphqlError {
  message?: string
  /** Which field failed, e.g. ['repo', 'policies']. */
  path?: (string | number)[]
  extensions?: {
    code?: string
    status?: number
  }
}

interface RepositorySettingsResponse {
  repo?: {
    id: string
    name: string
    username: string
    description?: string | null
    isPublic: boolean
    policies?: RepositoryPolicy[]
  } | null
  properties?: RepositoryPropertyDefinition[] | null
}

interface RepositoryPropertyMutationResponse {
  addProperty?: RepositoryPropertyDefinition | null
  updateProperty?: RepositoryPropertyDefinition | null
}

interface RepositoryPropertyDeleteResponse {
  deleteProperty?: string | null
}

interface RepositoryDeleteResponse {
  deleteRepo?: string | null
}

interface RepositoryUpdateResponse {
  updateRepo?: RepositorySettingsData['repository'] | null
}

interface RichTextTemplatesResponse {
  richTextTemplates?: RichTextTemplate[] | null
}

interface RichTextTemplateMutationResponse {
  saveRichTextTemplate?: RichTextTemplate | null
  updateRichTextTemplate?: RichTextTemplate | null
}

interface RichTextTemplateDeleteResponse {
  deleteRichTextTemplate?: boolean | null
}

const richTextTemplatesQuery = `
  query LibraryClientRichTextTemplates($orgUsername: String!, $repoUsername: String!) {
    richTextTemplates(orgUsername: $orgUsername, repoUsername: $repoUsername) {
      id
      name
      richText
    }
  }
`

const saveRichTextTemplateMutation = `
  mutation LibraryClientSaveRichTextTemplate($input: RichTextTemplateInput!) {
    saveRichTextTemplate(input: $input) { id name richText }
  }
`

const updateRichTextTemplateMutation = `
  mutation LibraryClientUpdateRichTextTemplate($templateId: String!, $input: RichTextTemplateInput!) {
    updateRichTextTemplate(templateId: $templateId, input: $input) { id name richText }
  }
`

const deleteRichTextTemplateMutation = `
  mutation LibraryClientDeleteRichTextTemplate($orgUsername: String!, $repoUsername: String!, $templateId: String!) {
    deleteRichTextTemplate(orgUsername: $orgUsername, repoUsername: $repoUsername, templateId: $templateId)
  }
`

const repositorySettingsQuery = `
  query LibraryClientRepositorySettings($orgUsername: String!, $repoUsername: String!) {
    repo(orgUsername: $orgUsername, repoUsername: $repoUsername) {
      id
      name
      username
      description
      isPublic
      policies {
        userId
        role
      }
    }
    properties(orgUsername: $orgUsername, repoUsername: $repoUsername) {
      id
      name
      displayName
      typ
      meta {
        ... on IdType { autoGenerate }
        ... on JsonType { json }
        ... on RelationType { databaseId }
        ... on SelectType { options { id key name } }
        ... on MultiSelectType { options { id key name } }
      }
    }
  }
`

const addRepositoryPropertyMutation = `
  mutation LibraryClientAddRepositoryProperty($input: PropertyInput!) {
    addProperty(input: $input) {
      id
      name
      displayName
      typ
      meta {
        ... on IdType { autoGenerate }
        ... on JsonType { json }
        ... on RelationType { databaseId }
        ... on SelectType { options { id key name } }
        ... on MultiSelectType { options { id key name } }
      }
    }
  }
`

const updateRepositoryPropertyMutation = `
  mutation LibraryClientUpdateRepositoryProperty($id: String!, $input: PropertyInput!) {
    updateProperty(id: $id, input: $input) {
      id
      name
      displayName
      typ
      meta {
        ... on IdType { autoGenerate }
        ... on JsonType { json }
        ... on RelationType { databaseId }
        ... on SelectType { options { id key name } }
        ... on MultiSelectType { options { id key name } }
      }
    }
  }
`

const deleteRepositoryPropertyMutation = `
  mutation LibraryClientDeleteRepositoryProperty(
    $orgUsername: String!
    $repoUsername: String!
    $id: String!
  ) {
    deleteProperty(
      orgUsername: $orgUsername
      repoUsername: $repoUsername
      propertyId: $id
    )
  }
`

const deleteRepositoryMutation = `
  mutation LibraryClientDeleteRepository($orgUsername: String!, $repoUsername: String!) {
    deleteRepo(orgUsername: $orgUsername, repoUsername: $repoUsername)
  }
`

const updateRepositoryMutation = `
  mutation LibraryClientUpdateRepository($input: UpdateRepoInput!) {
    updateRepo(input: $input) {
      id
      name
      username
      description
      isPublic
    }
  }
`

function graphqlErrorKind(error: GraphqlError): RepositorySettingsErrorKind {
  const code = error.extensions?.code?.toUpperCase() ?? ''
  const message = error.message?.toLowerCase() ?? ''
  const status = error.extensions?.status
  if (
    status === 401 ||
    status === 403 ||
    ['FORBIDDEN', 'UNAUTHORIZED', 'UNAUTHENTICATED', 'PERMISSION_DENIED'].includes(code) ||
    /forbidden|unauthori[sz]ed|permission denied|not permitted/.test(message)
  ) {
    return 'permission'
  }
  if (code === 'NOT_FOUND' || /not found|does not exist/.test(message)) return 'not-found'
  if (['BAD_USER_INPUT', 'VALIDATION_ERROR'].includes(code)) return 'validation'
  return 'graphql'
}

async function requestRepositoryGraphQL<TData>(
  query: string,
  variables: Record<string, unknown>,
  target: RepositorySettingsTarget,
): Promise<TData> {
  let response: Response
  try {
    response = await fetch(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
      method: 'POST',
      headers: await libraryGraphqlHeaders(target.operatorId),
      body: JSON.stringify({ query, variables }),
    })
  } catch (error: unknown) {
    const detail = error instanceof Error ? `: ${error.message}` : ''
    throw new RepositorySettingsApiError(
      `Repository settings transport unavailable${detail}`,
      0,
      'transport',
    )
  }

  if (!response.ok) {
    const kind = response.status === 401 || response.status === 403 ? 'permission' : 'http'
    throw new RepositorySettingsApiError(
      kind === 'permission'
        ? 'You do not have permission to manage this repository.'
        : `Repository settings request failed: ${response.status}`,
      response.status,
      kind,
    )
  }

  let payload: { data?: TData; errors?: GraphqlError[] }
  try {
    payload = await response.json() as typeof payload
  } catch {
    throw new RepositorySettingsApiError(
      'Repository settings returned an invalid JSON response.',
      response.status,
      'invalid-response',
    )
  }

  // GraphQL reports per-field failures alongside the data that did resolve.
  // Throwing whenever `errors` is non-empty threw that data away: a denied
  // `repo.policies` — a field a repo Owner cannot read, and one this client
  // already treats as optional — replaced the whole settings page with
  // "Permission required", taking Property management down with it.
  //
  // Only fail when nothing usable came back.
  if (payload.errors?.length && payload.data == null) {
    const firstError = payload.errors[0]
    const kind = graphqlErrorKind(firstError)
    throw new RepositorySettingsApiError(
      kind === 'permission'
        ? 'You do not have permission to manage this repository.'
        : firstError.message ?? 'Repository settings request failed.',
      firstError.extensions?.status ?? 400,
      kind,
    )
  }

  if (payload.errors?.length) {
    console.warn(
      'Repository settings resolved with partial errors',
      payload.errors.map((error) => ({
        path: error.path,
        message: error.message,
      })),
    )
  }

  if (payload.data == null) {
    throw new RepositorySettingsApiError(
      'Repository settings returned no data.',
      response.status,
      'invalid-response',
    )
  }

  return payload.data
}

function propertyInput(
  target: RepositorySettingsTarget,
  draft: RepositoryPropertyDraft,
  validateKey: boolean,
): Record<string, unknown> {
  const name = validateKey ? draft.name.trim() : draft.name
  if (validateKey && !/^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(name)) {
    throw new RepositorySettingsApiError(
      t('repoSettings.propertyKeyInvalid'),
      422,
      'validation',
    )
  }
  const displayName = draft.displayName?.trim()
  if (draft.displayName !== undefined && (!displayName || [...displayName].length > 255)) {
    throw new RepositorySettingsApiError(
      t('repoSettings.propertyDisplayNameInvalid'),
      422,
      'validation',
    )
  }

  let meta: Record<string, unknown> | undefined
  if (draft.type === 'SELECT' || draft.type === 'MULTI_SELECT') {
    const options = (draft.options ?? []).map((option) => {
      const id = option.id?.trim()
      return {
        ...(id ? { id } : {}),
        identifier: option.identifier.trim(),
        label: option.label.trim(),
      }
    })
    if (options.some((option) => !option.identifier || !option.label)) {
      throw new RepositorySettingsApiError(
        'Every select option needs an identifier and label.',
        422,
        'validation',
      )
    }
    meta = draft.type === 'SELECT' ? { select: options } : { multiSelect: options }
  } else if (draft.type === 'RELATION') {
    const databaseId = draft.relationDatabaseId?.trim()
    if (!databaseId) {
      throw new RepositorySettingsApiError(
        'Relation properties require a repository database ID.',
        422,
        'validation',
      )
    }
    meta = { relation: databaseId }
  } else if (draft.type === 'ID') {
    meta = { id: draft.autoGenerateId ?? false }
  }

  return {
    orgUsername: target.orgUsername,
    repoUsername: target.repoUsername,
    propertyName: name,
    ...(displayName !== undefined ? { displayName } : {}),
    propertyType: draft.type,
    ...(meta ? { meta } : {}),
  }
}

export function isRepositoryPermissionError(error: unknown): boolean {
  return error instanceof RepositorySettingsApiError && error.kind === 'permission'
}

export async function fetchRepositorySettings(
  target: RepositorySettingsTarget,
): Promise<RepositorySettingsData> {
  const payload = await requestRepositoryGraphQL<RepositorySettingsResponse>(
    repositorySettingsQuery,
    {
      orgUsername: target.orgUsername,
      repoUsername: target.repoUsername,
    },
    target,
  )
  if (!payload.repo) {
    throw new RepositorySettingsApiError(
      `${target.orgUsername}/${target.repoUsername} was not found.`,
      404,
      'not-found',
    )
  }
  if (!Array.isArray(payload.properties)) {
    throw new RepositorySettingsApiError(
      'Repository settings returned no Property definitions.',
      200,
      'invalid-response',
    )
  }
  return {
    repository: {
      id: payload.repo.id,
      name: payload.repo.name,
      username: payload.repo.username,
      description: payload.repo.description,
      isPublic: payload.repo.isPublic,
    },
    properties: payload.properties,
    policies: payload.repo.policies ?? [],
  }
}

export async function createRepositoryProperty(
  target: RepositorySettingsTarget,
  draft: RepositoryPropertyDraft,
): Promise<RepositoryPropertyDefinition> {
  if (draft.name.trim().startsWith('ext_')) {
    throw new RepositorySettingsApiError(
      'Property names starting with "ext_" are reserved for system extensions.',
      422,
      'validation',
    )
  }
  const payload = await requestRepositoryGraphQL<RepositoryPropertyMutationResponse>(
    addRepositoryPropertyMutation,
    { input: propertyInput(target, draft, true) },
    target,
  )
  if (!payload.addProperty) {
    throw new RepositorySettingsApiError(
      'Repository did not return the created Property.',
      200,
      'invalid-response',
    )
  }
  return payload.addProperty
}

export async function updateRepositoryProperty(
  target: RepositorySettingsTarget,
  propertyId: string,
  draft: RepositoryPropertyDraft,
): Promise<RepositoryPropertyDefinition> {
  if (!propertyId.trim()) {
    throw new RepositorySettingsApiError(t('errors.propertyIdRequired'), 422, 'validation')
  }
  const payload = await requestRepositoryGraphQL<RepositoryPropertyMutationResponse>(
    updateRepositoryPropertyMutation,
    {
      id: propertyId,
      input: propertyInput(target, draft, false),
    },
    target,
  )
  if (!payload.updateProperty) {
    throw new RepositorySettingsApiError(
      'Repository did not return the updated Property.',
      200,
      'invalid-response',
    )
  }
  return payload.updateProperty
}

export async function deleteRepositoryProperty(
  target: RepositorySettingsTarget,
  propertyId: string,
): Promise<void> {
  if (!propertyId.trim()) {
    throw new RepositorySettingsApiError(t('errors.propertyIdRequired'), 422, 'validation')
  }
  const payload = await requestRepositoryGraphQL<RepositoryPropertyDeleteResponse>(
    deleteRepositoryPropertyMutation,
    {
      orgUsername: target.orgUsername,
      repoUsername: target.repoUsername,
      id: propertyId,
    },
    target,
  )
  if (payload.deleteProperty !== propertyId) {
    throw new RepositorySettingsApiError(
      'Repository did not confirm the deleted Property.',
      200,
      'invalid-response',
    )
  }
}

export async function deleteRepository(
  target: RepositorySettingsTarget,
): Promise<void> {
  const payload = await requestRepositoryGraphQL<RepositoryDeleteResponse>(
    deleteRepositoryMutation,
    {
      orgUsername: target.orgUsername.trim(),
      repoUsername: target.repoUsername.trim(),
    },
    target,
  )
  if (payload.deleteRepo !== 'ok') {
    throw new RepositorySettingsApiError(
      t('errors.deleteRepositoryInvalidResponse'),
      200,
      'invalid-response',
    )
  }
}

export async function updateRepositorySettings(
  target: RepositorySettingsTarget,
  update: RepositoryMetadataUpdate,
): Promise<RepositorySettingsData['repository']> {
  const description = update.description?.trim()
  if (update.description !== undefined && !description) {
    throw new RepositorySettingsApiError(
      'The current API cannot remove an existing repository description. Replace it with text or leave it unchanged.',
      422,
      'validation',
    )
  }
  const payload = await requestRepositoryGraphQL<RepositoryUpdateResponse>(
    updateRepositoryMutation,
    {
      input: {
        orgUsername: target.orgUsername,
        repoUsername: target.repoUsername,
        ...(description ? { description } : {}),
        isPublic: update.isPublic,
      },
    },
    target,
  )
  if (!payload.updateRepo) {
    throw new RepositorySettingsApiError(
      'Repository did not return the updated settings.',
      200,
      'invalid-response',
    )
  }
  return payload.updateRepo
}

function richTextTemplateInput(
  target: RepositorySettingsTarget,
  draft: RichTextTemplateDraft,
): Record<string, unknown> {
  const name = draft.name.trim()
  if (!name || [...name].length > 255) {
    throw new RepositorySettingsApiError(
      'Template names must contain between 1 and 255 characters.',
      422,
      'validation',
    )
  }
  let parsed: unknown
  try {
    parsed = JSON.parse(draft.richText)
  } catch {
    throw new RepositorySettingsApiError(
      'The RichText body must be a valid block document.',
      422,
      'validation',
    )
  }
  if (!Array.isArray(parsed) || new TextEncoder().encode(draft.richText).length > 4 * 1024 * 1024) {
    throw new RepositorySettingsApiError(
      'The RichText body must be a block document under 4 MiB.',
      422,
      'validation',
    )
  }
  return {
    orgUsername: target.orgUsername,
    repoUsername: target.repoUsername,
    name,
    richText: draft.richText,
  }
}

export async function fetchRichTextTemplates(
  target: RepositorySettingsTarget,
): Promise<RichTextTemplate[]> {
  const payload = await requestRepositoryGraphQL<RichTextTemplatesResponse>(
    richTextTemplatesQuery,
    { orgUsername: target.orgUsername, repoUsername: target.repoUsername },
    target,
  )
  if (!Array.isArray(payload.richTextTemplates)) {
    throw new RepositorySettingsApiError(
      'Repository did not return RichText templates.',
      200,
      'invalid-response',
    )
  }
  return payload.richTextTemplates
}

export async function createRichTextTemplate(
  target: RepositorySettingsTarget,
  draft: RichTextTemplateDraft,
): Promise<RichTextTemplate> {
  const payload = await requestRepositoryGraphQL<RichTextTemplateMutationResponse>(
    saveRichTextTemplateMutation,
    { input: richTextTemplateInput(target, draft) },
    target,
  )
  if (!payload.saveRichTextTemplate) {
    throw new RepositorySettingsApiError(
      'Repository did not return the created RichText template.',
      200,
      'invalid-response',
    )
  }
  return payload.saveRichTextTemplate
}

export async function updateRichTextTemplate(
  target: RepositorySettingsTarget,
  templateId: string,
  draft: RichTextTemplateDraft,
): Promise<RichTextTemplate> {
  if (!templateId.trim()) {
    throw new RepositorySettingsApiError('A template ID is required.', 422, 'validation')
  }
  const payload = await requestRepositoryGraphQL<RichTextTemplateMutationResponse>(
    updateRichTextTemplateMutation,
    { templateId, input: richTextTemplateInput(target, draft) },
    target,
  )
  if (!payload.updateRichTextTemplate) {
    throw new RepositorySettingsApiError(
      'Repository did not return the updated RichText template.',
      200,
      'invalid-response',
    )
  }
  return payload.updateRichTextTemplate
}

export async function deleteRichTextTemplate(
  target: RepositorySettingsTarget,
  templateId: string,
): Promise<void> {
  if (!templateId.trim()) {
    throw new RepositorySettingsApiError('A template ID is required.', 422, 'validation')
  }
  const payload = await requestRepositoryGraphQL<RichTextTemplateDeleteResponse>(
    deleteRichTextTemplateMutation,
    {
      orgUsername: target.orgUsername,
      repoUsername: target.repoUsername,
      templateId,
    },
    target,
  )
  if (payload.deleteRichTextTemplate !== true) {
    throw new RepositorySettingsApiError(
      'Repository did not confirm the deleted RichText template.',
      200,
      'invalid-response',
    )
  }
}
