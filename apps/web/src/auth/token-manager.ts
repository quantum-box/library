import { InitiateAuthCommand } from '@aws-sdk/client-cognito-identity-provider'
import { cognitoClient, getCognitoConfig } from './config'

export interface AuthTokens {
  accessToken: string
  refreshToken: string
  expiresAt: number
  userId: string
  email: string
  username: string
}

export interface AuthTokenStorage {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
  removeItem(key: string): void
}

export type AuthChangeReason =
  | 'signed-in'
  | 'refreshed'
  | 'signed-out'
  | 'expired'
  | 'storage'

type AuthChangeListener = (reason: AuthChangeReason) => void

const AUTH_STORAGE_KEY = 'library_auth'

/** Refresh this far ahead of expiry so in-flight requests never race the clock. */
const REFRESH_SKEW_SECONDS = 5 * 60
/** Floor for the scheduled refresh so a short access-token lifetime cannot spin. */
const MINIMUM_REFRESH_DELAY_MS = 10_000
const MAXIMUM_TIMER_DELAY_MS = 2_147_483_647
const TRANSIENT_RETRY_BASE_MS = 30_000
const TRANSIENT_RETRY_CEILING_MS = 5 * 60 * 1000

/**
 * Cognito errors that mean the refresh token itself is finished. Everything
 * else - network failures, throttling, 5xx - leaves the session in place so a
 * blip cannot sign the user out.
 */
const UNRECOVERABLE_REFRESH_ERRORS = new Set([
  'NotAuthorizedException',
  'UserNotFoundException',
  'UserNotConfirmedException',
  'PasswordResetRequiredException',
  'ResourceNotFoundException',
  'InvalidParameterException',
])

let storageOverride: AuthTokenStorage | undefined
let refreshTimer: ReturnType<typeof setTimeout> | undefined
let refreshInFlight: Promise<AuthTokens | null> | null = null
let transientFailureCount = 0
let sessionRevision = 0
let watchInstalled = false

const listeners = new Set<AuthChangeListener>()

function tokenStorage(): AuthTokenStorage | null {
  if (storageOverride) return storageOverride
  if (typeof globalThis.localStorage === 'undefined') return null
  return globalThis.localStorage
}

function nowSeconds(): number {
  return Date.now() / 1000
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

function notify(reason: AuthChangeReason) {
  for (const listener of [...listeners]) {
    listener(reason)
  }
}

function cancelScheduledRefresh() {
  if (refreshTimer !== undefined) {
    clearTimeout(refreshTimer)
    refreshTimer = undefined
  }
}

/** Test seam; also the hook a native credential store would use. */
export function setAuthTokenStorage(storage?: AuthTokenStorage) {
  cancelScheduledRefresh()
  storageOverride = storage
  sessionRevision += 1
  refreshInFlight = null
  transientFailureCount = 0
  const tokens = loadStoredTokens()
  if (tokens) scheduleRefresh(tokens)
}

export function subscribeAuthChange(listener: AuthChangeListener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function loadStoredTokens(): AuthTokens | null {
  const storage = tokenStorage()
  if (!storage) return null

  try {
    const stored = storage.getItem(AUTH_STORAGE_KEY)
    if (!stored) return null
    const parsed: unknown = JSON.parse(stored)
    if (isAuthTokens(parsed)) return parsed
    storage.removeItem(AUTH_STORAGE_KEY)
  } catch {
    try {
      storage.removeItem(AUTH_STORAGE_KEY)
    } catch {
      // Storage can be unavailable in restricted browser contexts.
    }
  }

  return null
}

export function storeTokens(
  tokens: AuthTokens,
  reason: AuthChangeReason = 'signed-in',
) {
  const storage = tokenStorage()
  if (!storage) {
    throw new Error(
      'Authentication token storage is unavailable; configure an AuthTokenStorage adapter',
    )
  }

  storage.setItem(AUTH_STORAGE_KEY, JSON.stringify(tokens))
  sessionRevision += 1
  transientFailureCount = 0
  scheduleRefresh(tokens)
  notify(reason)
}

export function clearTokens(reason: AuthChangeReason = 'signed-out') {
  cancelScheduledRefresh()
  sessionRevision += 1
  refreshInFlight = null
  transientFailureCount = 0
  try {
    tokenStorage()?.removeItem(AUTH_STORAGE_KEY)
  } finally {
    notify(reason)
  }
}

function scheduleRefresh(tokens: AuthTokens) {
  cancelScheduledRefresh()
  if (typeof setTimeout !== 'function') return

  const secondsUntilExpiry = tokens.expiresAt - nowSeconds()

  if (!tokens.refreshToken) {
    if (secondsUntilExpiry <= 0) {
      clearTokens('expired')
      return
    }
    refreshTimer = setTimeout(
      () => {
        refreshTimer = undefined
        if (loadStoredTokens()?.accessToken === tokens.accessToken) {
          clearTokens('expired')
        }
      },
      Math.min(secondsUntilExpiry * 1000, MAXIMUM_TIMER_DELAY_MS),
    )
    return
  }

  // Never schedule at zero: an access-token lifetime shorter than the skew
  // would otherwise refresh in a tight loop until Cognito throttles us.
  const secondsUntilRefresh =
    secondsUntilExpiry > REFRESH_SKEW_SECONDS
      ? secondsUntilExpiry - REFRESH_SKEW_SECONDS
      : Math.max(secondsUntilExpiry * 0.5, 0)

  refreshTimer = setTimeout(
    () => {
      refreshTimer = undefined
      void refreshTokens().catch(() => undefined)
    },
    Math.min(
      Math.max(secondsUntilRefresh * 1000, MINIMUM_REFRESH_DELAY_MS),
      MAXIMUM_TIMER_DELAY_MS,
    ),
  )
}

function scheduleTransientRetry() {
  cancelScheduledRefresh()
  if (typeof setTimeout !== 'function') return

  const delay = Math.min(
    TRANSIENT_RETRY_BASE_MS * 2 ** Math.max(transientFailureCount - 1, 0),
    TRANSIENT_RETRY_CEILING_MS,
  )

  refreshTimer = setTimeout(() => {
    refreshTimer = undefined
    void refreshTokens().catch(() => undefined)
  }, delay)
}

function isUnrecoverableRefreshError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const name = (error as { name?: unknown }).name
  return typeof name === 'string' && UNRECOVERABLE_REFRESH_ERRORS.has(name)
}

/**
 * Exchanges a refresh token for a fresh access token. This deliberately talks
 * to Cognito only: the Library platform sign-in is a sign-in-time concern, and
 * making it part of every refresh turned any platform hiccup into a sign-out.
 */
export async function refreshAccessToken(
  refreshToken: string,
  previous: Pick<AuthTokens, 'userId' | 'email' | 'username'>,
): Promise<AuthTokens> {
  if (!refreshToken.trim()) {
    throw new Error('Cannot refresh Library authentication without a refresh token')
  }

  const response = await cognitoClient().send(
    new InitiateAuthCommand({
      AuthFlow: 'REFRESH_TOKEN_AUTH',
      ClientId: getCognitoConfig().clientId,
      AuthParameters: {
        REFRESH_TOKEN: refreshToken,
      },
    }),
  )

  const { AccessToken, ExpiresIn, RefreshToken } =
    response.AuthenticationResult ?? {}

  if (!AccessToken) {
    throw new Error('No access token from refresh')
  }

  return {
    accessToken: AccessToken,
    // Cognito omits the refresh token unless rotation is enabled.
    refreshToken: RefreshToken ?? refreshToken,
    expiresAt: Math.floor(nowSeconds() + (ExpiresIn ?? 3600)),
    userId: previous.userId,
    email: previous.email,
    username: previous.username,
  }
}

/**
 * Refreshes the stored session, collapsing concurrent callers onto a single
 * Cognito exchange so a page load cannot spend the same refresh token twice.
 */
export function refreshTokens(): Promise<AuthTokens | null> {
  if (refreshInFlight) return refreshInFlight

  const storedTokens = loadStoredTokens()
  if (!storedTokens) return Promise.resolve(null)

  if (!storedTokens.refreshToken) {
    if (storedTokens.expiresAt <= nowSeconds()) clearTokens('expired')
    return Promise.resolve(null)
  }

  const revisionAtStart = sessionRevision

  refreshInFlight = (async () => {
    try {
      const refreshed = await refreshAccessToken(storedTokens.refreshToken, {
        userId: storedTokens.userId,
        email: storedTokens.email,
        username: storedTokens.username,
      })

      // A sign-out or a different sign-in landed while we were in flight.
      if (sessionRevision !== revisionAtStart) return null

      storeTokens(refreshed, 'refreshed')
      return refreshed
    } catch (error) {
      if (sessionRevision === revisionAtStart) {
        if (isUnrecoverableRefreshError(error)) {
          clearTokens('expired')
        } else {
          // Keep the session: the refresh token is still good, the network is not.
          transientFailureCount += 1
          scheduleTransientRetry()
        }
      }
      throw error
    } finally {
      refreshInFlight = null
    }
  })()

  return refreshInFlight
}

/**
 * The single entry point for anything about to call the API. Refreshes ahead
 * of expiry instead of relying on a timer that a sleeping laptop never fires.
 */
export async function getValidAccessToken(): Promise<string | null> {
  const tokens = loadStoredTokens()
  if (!tokens) return null

  const secondsUntilExpiry = tokens.expiresAt - nowSeconds()
  if (secondsUntilExpiry > REFRESH_SKEW_SECONDS) return tokens.accessToken

  if (!tokens.refreshToken) {
    if (secondsUntilExpiry <= 0) {
      clearTokens('expired')
      return null
    }
    return tokens.accessToken
  }

  try {
    const refreshed = await refreshTokens()
    return refreshed?.accessToken ?? null
  } catch {
    // The refresh may have ended the session (rejected refresh token) or just
    // failed transiently; only in the latter case is the stale token still
    // worth sending for the rest of its life.
    const remaining = loadStoredTokens()
    if (!remaining) return null
    return remaining.expiresAt - nowSeconds() > 0 ? remaining.accessToken : null
  }
}

/**
 * Re-checks the session when the tab comes back and mirrors sign-in/sign-out
 * across tabs. Timers do not run while the machine sleeps, so waking up is the
 * moment an expired access token would otherwise reach the API as a 401.
 */
export function startAuthTokenWatch(): () => void {
  if (typeof window === 'undefined' || watchInstalled) return () => undefined
  watchInstalled = true

  const revalidate = () => {
    if (
      typeof document !== 'undefined' &&
      document.visibilityState === 'hidden'
    ) {
      return
    }
    void getValidAccessToken()
  }

  const onStorage = (event: StorageEvent) => {
    if (event.key !== null && event.key !== AUTH_STORAGE_KEY) return
    const tokens = loadStoredTokens()
    if (tokens) {
      scheduleRefresh(tokens)
    } else {
      cancelScheduledRefresh()
    }
    notify('storage')
  }

  window.addEventListener('focus', revalidate)
  window.addEventListener('online', revalidate)
  window.addEventListener('storage', onStorage)
  document.addEventListener('visibilitychange', revalidate)

  const stored = loadStoredTokens()
  if (stored) scheduleRefresh(stored)

  return () => {
    window.removeEventListener('focus', revalidate)
    window.removeEventListener('online', revalidate)
    window.removeEventListener('storage', onStorage)
    document.removeEventListener('visibilitychange', revalidate)
    cancelScheduledRefresh()
    watchInstalled = false
  }
}
