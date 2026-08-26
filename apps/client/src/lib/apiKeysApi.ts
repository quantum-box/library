import {
  configuredLibraryApiBaseUrl,
  libraryGraphqlHeaders,
  type LibraryGraphqlError,
} from './libraryGraphql'

/** A key as it can be listed: everything except the secret itself. */
export interface ApiKeySummary {
  id: string
  name: string
  createdAt: string
}

/** A key as it comes back from creation, the one time `value` is readable. */
export interface CreatedApiKey extends ApiKeySummary {
  value: string
}

/**
 * Keys belong to an organization, not to a repository, so callers name the
 * organization even when they reached this from a repository page.
 */
export interface ApiKeyTarget {
  orgUsername: string
  operatorId?: string
}

export type ApiKeysErrorKind =
  | 'transport'
  | 'http'
  | 'invalid-response'
  | 'permission'
  | 'not-found'
  | 'graphql'

export class ApiKeysApiError extends Error {
  readonly status: number
  readonly kind: ApiKeysErrorKind

  constructor(message: string, status: number, kind: ApiKeysErrorKind) {
    super(message)
    this.name = 'ApiKeysApiError'
    this.status = status
    this.kind = kind
  }
}

export function isApiKeyPermissionError(error: unknown): boolean {
  return error instanceof ApiKeysApiError && error.kind === 'permission'
}

const LIST_QUERY = `
  query ClientApiKeys($orgUsername: String!) {
    apiKeys(orgUsername: $orgUsername) {
      id
      name
      createdAt
    }
  }
`

const CREATE_MUTATION = `
  mutation ClientCreateApiKey($input: CreateApiKeyInput!) {
    createApiKey(input: $input) {
      apiKey {
        id
        name
        value
        createdAt
      }
    }
  }
`

const REVOKE_MUTATION = `
  mutation ClientRevokeApiKey($input: RevokeApiKeyInput!) {
    revokeApiKey(input: $input)
  }
`

export async function fetchApiKeys(
  target: ApiKeyTarget,
): Promise<ApiKeySummary[]> {
  const data = await requestApiKeysGraphQL<{ apiKeys: ApiKeySummary[] }>(
    LIST_QUERY,
    { orgUsername: target.orgUsername },
    target,
  )
  return data.apiKeys
}

export async function createApiKey(
  target: ApiKeyTarget,
  name: string,
): Promise<CreatedApiKey> {
  const data = await requestApiKeysGraphQL<{
    createApiKey: { apiKey: CreatedApiKey }
  }>(
    CREATE_MUTATION,
    { input: { organizationUsername: target.orgUsername, name } },
    target,
  )
  return data.createApiKey.apiKey
}

export async function revokeApiKey(
  target: ApiKeyTarget,
  apiKeyId: string,
): Promise<void> {
  await requestApiKeysGraphQL<{ revokeApiKey: boolean }>(
    REVOKE_MUTATION,
    { input: { organizationUsername: target.orgUsername, apiKeyId } },
    target,
  )
}

function graphqlErrorKind(error: LibraryGraphqlError): ApiKeysErrorKind {
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
  return 'graphql'
}

/**
 * Unlike repository settings, every field here is the whole answer: a
 * partial result is not useful, so any error fails the call.
 */
async function requestApiKeysGraphQL<TData>(
  query: string,
  variables: Record<string, unknown>,
  target: ApiKeyTarget,
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
    throw new ApiKeysApiError(`API key transport unavailable${detail}`, 0, 'transport')
  }

  if (!response.ok) {
    const kind = response.status === 401 || response.status === 403 ? 'permission' : 'http'
    throw new ApiKeysApiError(
      kind === 'permission'
        ? 'You do not have permission to manage API keys for this organization.'
        : `API key request failed: ${response.status}`,
      response.status,
      kind,
    )
  }

  let payload: { data?: TData; errors?: LibraryGraphqlError[] }
  try {
    payload = (await response.json()) as typeof payload
  } catch {
    throw new ApiKeysApiError(
      'API key request returned an invalid JSON response.',
      response.status,
      'invalid-response',
    )
  }

  if (payload.errors?.length) {
    const firstError = payload.errors[0]
    const kind = graphqlErrorKind(firstError)
    throw new ApiKeysApiError(
      kind === 'permission'
        ? 'You do not have permission to manage API keys for this organization.'
        : firstError.message ?? 'API key request failed.',
      firstError.extensions?.status ?? 400,
      kind,
    )
  }

  if (payload.data == null) {
    throw new ApiKeysApiError(
      'API key request returned no data.',
      response.status,
      'invalid-response',
    )
  }

  return payload.data
}
