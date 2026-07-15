export interface AuthTokens {
  accessToken: string
  refreshToken: string
  expiresAt: number
  userId: string
  email: string
  username: string
}

export type AuthIdentity = Pick<AuthTokens, 'userId' | 'email' | 'username'>

export interface AuthTokenStorage {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
  removeItem(key: string): void
}

type AuthChangeReason = 'expired' | 'refreshed' | 'signed-in' | 'signed-out'

interface CognitoAuthenticationResult {
  AccessToken?: string
  RefreshToken?: string
  ExpiresIn?: number
}

interface CognitoUser {
  UserAttributes?: Array<{ Name?: string; Value?: string }>
}

interface LibraryUser {
  id: string
  email?: string | null
}

const authStorageKey = 'library_auth'
const authChangeEvent = 'library-auth-change'
const refreshEarlySeconds = 5 * 60
const maximumTimerDelay = 2_147_483_647

let storageOverride: AuthTokenStorage | undefined
let expiryTimer: ReturnType<typeof setTimeout> | undefined
let refreshPromise: Promise<AuthTokens | null> | null = null
let sessionRevision = 0

const signInWithPlatformMutation = `
  mutation LibraryClientSignInWithPlatform(
    $platformId: String!
    $accessToken: String!
    $allowSignUp: Boolean!
  ) {
    signIn: signInWithPlatform(
      platformId: $platformId
      accessToken: $accessToken
      allowSignUp: $allowSignUp
    ) {
      id
      email
    }
  }
`

function requireEnvironmentValue(name: string, value: string | undefined): string {
  const configuredValue = value?.trim()
  if (!configuredValue) {
    throw new Error(
      `Missing ${name}: configure it explicitly before using Library authentication`,
    )
  }
  return configuredValue
}

function configuredCognitoClientId(): string {
  return requireEnvironmentValue(
    'VITE_COGNITO_CLIENT_ID',
    import.meta.env.VITE_COGNITO_CLIENT_ID,
  )
}

function configuredCognitoRegion(): string {
  return import.meta.env.VITE_COGNITO_REGION?.trim() || 'ap-northeast-1'
}

function configuredLibraryApiBaseUrl(): string {
  return (
    import.meta.env.VITE_LIBRARY_API_BASE_URL?.trim() ||
    import.meta.env.VITE_BACKEND_API_URL?.trim() ||
    'http://localhost:50053'
  ).replace(/\/+$/, '')
}

function configuredPlatformId(): string {
  return (
    import.meta.env.VITE_LIBRARY_PLATFORM_ID?.trim() ||
    import.meta.env.VITE_PLATFORM_ID?.trim() ||
    'tn_01j702qf86pc2j35s0kv0gv3gy'
  )
}

function tokenStorage(): AuthTokenStorage | null {
  if (storageOverride) return storageOverride
  if (typeof globalThis.localStorage === 'undefined') return null
  return globalThis.localStorage
}

function emitAuthChange(reason: AuthChangeReason) {
  if (typeof window === 'undefined') return
  window.dispatchEvent(
    new CustomEvent(authChangeEvent, {
      detail: { reason },
    }),
  )
}

function cancelExpiryTimer() {
  if (expiryTimer !== undefined) {
    clearTimeout(expiryTimer)
    expiryTimer = undefined
  }
}

function isAuthTokens(value: unknown): value is AuthTokens {
  if (!value || typeof value !== 'object') return false
  const tokens = value as Partial<AuthTokens>
  return (
    typeof tokens.accessToken === 'string' &&
    tokens.accessToken.length > 0 &&
    typeof tokens.refreshToken === 'string' &&
    typeof tokens.expiresAt === 'number' &&
    Number.isFinite(tokens.expiresAt) &&
    typeof tokens.userId === 'string' &&
    typeof tokens.email === 'string' &&
    typeof tokens.username === 'string'
  )
}

function readStoredAuthTokens(): AuthTokens | null {
  const storage = tokenStorage()
  if (!storage) return null

  try {
    const stored = storage.getItem(authStorageKey)
    if (!stored) return null
    const parsed: unknown = JSON.parse(stored)
    if (isAuthTokens(parsed)) return parsed
    storage.removeItem(authStorageKey)
  } catch {
    try {
      storage.removeItem(authStorageKey)
    } catch {
      // Storage can be unavailable in restricted browser contexts.
    }
  }

  return null
}

function clearStoredAuthTokens(reason: AuthChangeReason) {
  cancelExpiryTimer()
  sessionRevision += 1
  try {
    tokenStorage()?.removeItem(authStorageKey)
  } finally {
    emitAuthChange(reason)
  }
}

function handleScheduledSession(expectedAccessToken: string) {
  const stored = readStoredAuthTokens()
  if (stored?.accessToken !== expectedAccessToken) return

  const secondsUntilExpiry = stored.expiresAt - Date.now() / 1000
  if (stored.refreshToken) {
    if (secondsUntilExpiry > refreshEarlySeconds) {
      scheduleSessionExpiry(stored)
    } else {
      void refreshStoredAuthTokens().catch(() => undefined)
    }
    return
  }

  if (secondsUntilExpiry <= 0) {
    clearStoredAuthTokens('expired')
  } else {
    scheduleSessionExpiry(stored)
  }
}

function scheduleSessionExpiry(tokens: AuthTokens) {
  cancelExpiryTimer()

  const secondsUntilExpiry = tokens.expiresAt - Date.now() / 1000
  if (secondsUntilExpiry <= 0) {
    queueMicrotask(() => handleScheduledSession(tokens.accessToken))
    return
  }

  const secondsUntilAction = tokens.refreshToken
    ? secondsUntilExpiry > refreshEarlySeconds
      ? secondsUntilExpiry - refreshEarlySeconds
      : Math.max(secondsUntilExpiry * 0.8, 1)
    : secondsUntilExpiry

  expiryTimer = setTimeout(
    () => {
      expiryTimer = undefined
      handleScheduledSession(tokens.accessToken)
    },
    Math.min(secondsUntilAction * 1000, maximumTimerDelay),
  )
}

function persistAuthTokens(tokens: AuthTokens, reason: AuthChangeReason) {
  if (!isAuthTokens(tokens)) {
    throw new Error('Cannot store an invalid Library authentication session')
  }

  const storage = tokenStorage()
  if (!storage) {
    throw new Error(
      'Authentication token storage is unavailable; configure an AuthTokenStorage adapter',
    )
  }

  storage.setItem(authStorageKey, JSON.stringify(tokens))
  sessionRevision += 1
  scheduleSessionExpiry(tokens)
  emitAuthChange(reason)
}

function errorMessage(payload: unknown): string | undefined {
  if (!payload || typeof payload !== 'object') return undefined
  const candidate = payload as {
    message?: unknown
    Message?: unknown
    __type?: unknown
  }
  if (typeof candidate.message === 'string' && candidate.message) {
    return candidate.message
  }
  if (typeof candidate.Message === 'string' && candidate.Message) {
    return candidate.Message
  }
  if (typeof candidate.__type === 'string' && candidate.__type) {
    return candidate.__type.split('#').at(-1)
  }
  return undefined
}

async function requestCognito<T>(
  target: string,
  body: Record<string, unknown>,
): Promise<T> {
  const response = await fetch(
    `https://cognito-idp.${configuredCognitoRegion()}.amazonaws.com/`,
    {
      method: 'POST',
      headers: {
        'content-type': 'application/x-amz-json-1.1',
        'x-amz-target': `AWSCognitoIdentityProviderService.${target}`,
      },
      body: JSON.stringify(body),
    },
  )
  const payload: unknown = await response.json().catch(() => ({}))
  if (!response.ok) {
    throw new Error(errorMessage(payload) ?? `Cognito ${target} failed (${response.status})`)
  }
  return payload as T
}

async function signInToLibrary(accessToken: string): Promise<LibraryUser> {
  const platformId = configuredPlatformId()
  const response = await fetch(`${configuredLibraryApiBaseUrl()}/v1/graphql`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      Authorization: `Bearer ${accessToken}`,
      'x-platform-id': platformId,
      'x-operator-id': platformId,
    },
    body: JSON.stringify({
      query: signInWithPlatformMutation,
      variables: {
        platformId,
        accessToken,
        allowSignUp: true,
      },
    }),
  })

  const payload = (await response.json().catch(() => null)) as {
    data?: { signIn?: LibraryUser | null }
    errors?: Array<{ message?: string }>
  } | null
  const graphqlError = payload?.errors?.find((error) => error.message)?.message

  if (!response.ok || graphqlError) {
    throw new Error(
      graphqlError ?? `Library signInWithPlatform failed (${response.status})`,
    )
  }

  const user = payload?.data?.signIn
  if (!user?.id) {
    throw new Error('Library signInWithPlatform returned no user')
  }
  return user
}

/**
 * Replaces the default localStorage adapter. This is an integration seam for a
 * future native credential store; localStorage is not secure native storage.
 */
export function setAuthTokenStorage(storage?: AuthTokenStorage) {
  cancelExpiryTimer()
  storageOverride = storage
  sessionRevision += 1
  const tokens = readStoredAuthTokens()
  if (tokens && tokens.expiresAt > Date.now() / 1000) {
    scheduleSessionExpiry(tokens)
  }
}

export function loadAuthTokens(): AuthTokens | null {
  const tokens = readStoredAuthTokens()
  if (!tokens) return null

  if (tokens.expiresAt <= Date.now() / 1000) {
    if (tokens.refreshToken) {
      void refreshStoredAuthTokens().catch(() => undefined)
    } else {
      clearStoredAuthTokens('expired')
    }
    return null
  }

  scheduleSessionExpiry(tokens)
  return tokens
}

/**
 * Returns only the persisted actor identity, including while an access token
 * is waiting to refresh. This is safe for cache namespacing, never for API
 * authorization.
 */
export function loadStoredAuthIdentity(): AuthIdentity | null {
  const tokens = readStoredAuthTokens()
  if (!tokens) return null
  return {
    userId: tokens.userId,
    email: tokens.email,
    username: tokens.username,
  }
}

export async function getValidAuthTokens(): Promise<AuthTokens | null> {
  const tokens = readStoredAuthTokens()
  if (!tokens) return null

  const secondsUntilExpiry = tokens.expiresAt - Date.now() / 1000
  if (tokens.refreshToken && secondsUntilExpiry <= refreshEarlySeconds) {
    try {
      return await refreshStoredAuthTokens()
    } catch {
      return null
    }
  }

  if (secondsUntilExpiry <= 0) {
    clearStoredAuthTokens('expired')
    return null
  }

  scheduleSessionExpiry(tokens)
  return tokens
}

export function storeAuthTokens(tokens: AuthTokens) {
  persistAuthTokens(tokens, 'signed-in')
}

export function clearAuthTokens() {
  clearStoredAuthTokens('signed-out')
}

export async function signInWithCredentials(
  username: string,
  password: string,
): Promise<AuthTokens> {
  const clientId = configuredCognitoClientId()
  const auth = await requestCognito<{
    AuthenticationResult?: CognitoAuthenticationResult
    ChallengeName?: string
  }>('InitiateAuth', {
    AuthFlow: 'USER_PASSWORD_AUTH',
    ClientId: clientId,
    AuthParameters: {
      USERNAME: username,
      PASSWORD: password,
    },
  })

  const accessToken = auth.AuthenticationResult?.AccessToken
  if (!accessToken) {
    if (auth.ChallengeName) {
      throw new Error(`Cognito sign-in requires unsupported challenge: ${auth.ChallengeName}`)
    }
    throw new Error('No access token returned from Cognito')
  }

  const cognitoUser = await requestCognito<CognitoUser>('GetUser', {
    AccessToken: accessToken,
  })
  const cognitoEmail =
    cognitoUser.UserAttributes?.find((attribute) => attribute.Name === 'email')
      ?.Value ?? ''
  const libraryUser = await signInToLibrary(accessToken)

  return {
    accessToken,
    refreshToken: auth.AuthenticationResult?.RefreshToken ?? '',
    expiresAt: Math.floor(
      Date.now() / 1000 + (auth.AuthenticationResult?.ExpiresIn ?? 3600),
    ),
    userId: libraryUser.id,
    email: cognitoEmail || libraryUser.email || '',
    username,
  }
}

export async function refreshAccessToken(
  refreshToken: string,
  username: string,
): Promise<AuthTokens> {
  if (!refreshToken.trim()) {
    throw new Error('Cannot refresh Library authentication without a refresh token')
  }

  const auth = await requestCognito<{
    AuthenticationResult?: CognitoAuthenticationResult
  }>('InitiateAuth', {
    AuthFlow: 'REFRESH_TOKEN_AUTH',
    ClientId: configuredCognitoClientId(),
    AuthParameters: {
      REFRESH_TOKEN: refreshToken,
    },
  })
  const accessToken = auth.AuthenticationResult?.AccessToken
  if (!accessToken) {
    throw new Error('No access token returned from Cognito refresh')
  }

  const libraryUser = await signInToLibrary(accessToken)
  return {
    accessToken,
    refreshToken: auth.AuthenticationResult?.RefreshToken ?? refreshToken,
    expiresAt: Math.floor(
      Date.now() / 1000 + (auth.AuthenticationResult?.ExpiresIn ?? 3600),
    ),
    userId: libraryUser.id,
    email: libraryUser.email ?? '',
    username,
  }
}

export function refreshStoredAuthTokens(): Promise<AuthTokens | null> {
  if (refreshPromise) return refreshPromise

  const storedTokens = readStoredAuthTokens()
  if (!storedTokens?.refreshToken) {
    if (storedTokens) clearStoredAuthTokens('expired')
    return Promise.resolve(null)
  }

  const revisionAtStart = sessionRevision
  refreshPromise = (async () => {
    try {
      const refreshedTokens = await refreshAccessToken(
        storedTokens.refreshToken,
        storedTokens.username,
      )
      if (sessionRevision !== revisionAtStart) return null

      const nextTokens = {
        ...refreshedTokens,
        email: refreshedTokens.email || storedTokens.email,
      }
      persistAuthTokens(nextTokens, 'refreshed')
      return nextTokens
    } catch (error) {
      if (sessionRevision === revisionAtStart) {
        clearStoredAuthTokens('expired')
      }
      throw error
    } finally {
      refreshPromise = null
    }
  })()

  return refreshPromise
}
