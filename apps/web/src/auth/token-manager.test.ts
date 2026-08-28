import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const send = vi.fn()

vi.mock('./config', () => ({
  getCognitoConfig: () => ({
    clientId: 'cognito-client-id',
    region: 'ap-northeast-1',
    hostedUiDomain: '',
  }),
  cognitoClient: () => ({ send }),
}))

import {
  clearTokens,
  getValidAccessToken,
  loadStoredTokens,
  refreshTokens,
  setAuthTokenStorage,
  storeTokens,
  subscribeAuthChange,
  type AuthTokenStorage,
  type AuthTokens,
} from './token-manager'

const now = 1_750_000_000_000

function memoryStorage(): AuthTokenStorage {
  const entries = new Map<string, string>()
  return {
    getItem: (key) => entries.get(key) ?? null,
    setItem: (key, value) => {
      entries.set(key, value)
    },
    removeItem: (key) => {
      entries.delete(key)
    },
  }
}

function tokens(overrides: Partial<AuthTokens> = {}): AuthTokens {
  return {
    accessToken: 'stored-access-token',
    refreshToken: 'stored-refresh-token',
    expiresAt: now / 1000 + 3600,
    userId: 'library-user-1',
    email: 'person@example.test',
    username: 'person',
    ...overrides,
  }
}

function cognitoError(name: string): Error {
  const error = new Error(name)
  error.name = name
  return error
}

describe('auth token manager', () => {
  beforeEach(() => {
    vi.spyOn(Date, 'now').mockReturnValue(now)
    send.mockReset()
    setAuthTokenStorage(memoryStorage())
  })

  afterEach(() => {
    clearTokens()
    setAuthTokenStorage()
    vi.restoreAllMocks()
  })

  it('uses the stored access token while it is comfortably valid', async () => {
    storeTokens(tokens())

    await expect(getValidAccessToken()).resolves.toBe('stored-access-token')
    expect(send).not.toHaveBeenCalled()
  })

  it('refreshes ahead of expiry and keeps the stored identity', async () => {
    send.mockResolvedValue({
      AuthenticationResult: { AccessToken: 'fresh-access-token', ExpiresIn: 3600 },
    })
    storeTokens(tokens({ expiresAt: now / 1000 + 60 }))

    await expect(getValidAccessToken()).resolves.toBe('fresh-access-token')
    expect(loadStoredTokens()).toMatchObject({
      accessToken: 'fresh-access-token',
      // Cognito omits the refresh token unless rotation is enabled.
      refreshToken: 'stored-refresh-token',
      userId: 'library-user-1',
      email: 'person@example.test',
      username: 'person',
      expiresAt: now / 1000 + 3600,
    })
  })

  it('collapses concurrent refreshes onto one Cognito exchange', async () => {
    send.mockResolvedValue({
      AuthenticationResult: { AccessToken: 'fresh-access-token', ExpiresIn: 3600 },
    })
    storeTokens(tokens({ expiresAt: now / 1000 + 60 }))

    const [first, second, third] = await Promise.all([
      getValidAccessToken(),
      getValidAccessToken(),
      refreshTokens(),
    ])

    expect(send).toHaveBeenCalledTimes(1)
    expect(first).toBe('fresh-access-token')
    expect(second).toBe('fresh-access-token')
    expect(third?.accessToken).toBe('fresh-access-token')
  })

  it('keeps the session when a refresh fails for a transient reason', async () => {
    send.mockRejectedValue(new TypeError('Failed to fetch'))
    storeTokens(tokens({ expiresAt: now / 1000 + 60 }))

    // The stale-but-unexpired token is still worth sending.
    await expect(getValidAccessToken()).resolves.toBe('stored-access-token')
    expect(loadStoredTokens()).not.toBeNull()
  })

  it('signs out only when Cognito rejects the refresh token itself', async () => {
    send.mockRejectedValue(cognitoError('NotAuthorizedException'))
    storeTokens(tokens({ expiresAt: now / 1000 + 60 }))

    await expect(getValidAccessToken()).resolves.toBeNull()
    expect(loadStoredTokens()).toBeNull()
  })

  it('notifies subscribers on sign-in, refresh and sign-out', async () => {
    const reasons: string[] = []
    const unsubscribe = subscribeAuthChange((reason) => reasons.push(reason))
    send.mockResolvedValue({
      AuthenticationResult: { AccessToken: 'fresh-access-token', ExpiresIn: 3600 },
    })

    storeTokens(tokens({ expiresAt: now / 1000 + 60 }))
    await getValidAccessToken()
    clearTokens('signed-out')
    unsubscribe()

    expect(reasons).toEqual(['signed-in', 'refreshed', 'signed-out'])
  })

  it('drops an expired session that has no refresh token', async () => {
    storeTokens(tokens({ refreshToken: '', expiresAt: now / 1000 - 1 }))

    await expect(getValidAccessToken()).resolves.toBeNull()
    expect(loadStoredTokens()).toBeNull()
    expect(send).not.toHaveBeenCalled()
  })

  it('discards a malformed stored session instead of throwing', () => {
    const storage = memoryStorage()
    storage.setItem('library_auth', '{"accessToken":')
    setAuthTokenStorage(storage)

    expect(loadStoredTokens()).toBeNull()
  })
})
