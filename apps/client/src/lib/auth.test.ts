import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearAuthTokens,
  loadAuthTokens,
  loadStoredAuthIdentity,
  refreshAccessToken,
  refreshStoredAuthTokens,
  setAuthTokenStorage,
  signInWithCredentials,
  storeAuthTokens,
  type AuthTokenStorage,
  type AuthTokens,
} from './auth'

const now = 1_750_000_000_000

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: vi.fn().mockResolvedValue(body),
  } as unknown as Response
}

function requestBody(call: [RequestInfo | URL, RequestInit?]) {
  return JSON.parse(String(call[1]?.body)) as Record<string, unknown>
}

function configuredEnvironment() {
  vi.stubEnv('VITE_COGNITO_CLIENT_ID', 'cognito-client-id')
  vi.stubEnv('VITE_COGNITO_REGION', 'us-west-2')
  vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test/')
  vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
}

function storedTokens(overrides: Partial<AuthTokens> = {}): AuthTokens {
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

describe('Library authentication', () => {
  beforeEach(() => {
    setAuthTokenStorage()
    localStorage.clear()
    configuredEnvironment()
    vi.spyOn(Date, 'now').mockReturnValue(now)
  })

  afterEach(() => {
    clearAuthTokens()
    setAuthTokenStorage()
    vi.useRealTimers()
    vi.unstubAllGlobals()
    vi.unstubAllEnvs()
    vi.restoreAllMocks()
  })

  it('requires an explicitly configured Cognito client id', async () => {
    const fetchMock = vi.fn<typeof fetch>()
    vi.stubGlobal('fetch', fetchMock)
    vi.stubEnv('VITE_COGNITO_CLIENT_ID', '   ')

    await expect(signInWithCredentials('person', 'secret')).rejects.toThrow(
      'Missing VITE_COGNITO_CLIENT_ID',
    )
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('signs in through Cognito and then registers the session with Library', async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        jsonResponse({
          AuthenticationResult: {
            AccessToken: 'cognito-access-token',
            RefreshToken: 'cognito-refresh-token',
            ExpiresIn: 900,
          },
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          UserAttributes: [
            { Name: 'sub', Value: 'cognito-user' },
            { Name: 'email', Value: 'person@example.test' },
          ],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          data: {
            signIn: {
              id: 'library-user',
              email: 'library-person@example.test',
            },
          },
        }),
      )
    vi.stubGlobal('fetch', fetchMock)

    await expect(signInWithCredentials('person', 'secret')).resolves.toEqual({
      accessToken: 'cognito-access-token',
      refreshToken: 'cognito-refresh-token',
      expiresAt: now / 1000 + 900,
      userId: 'library-user',
      email: 'person@example.test',
      username: 'person',
    })

    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      'https://cognito-idp.us-west-2.amazonaws.com/',
    )
    expect(requestBody(fetchMock.mock.calls[0]!)).toEqual({
      AuthFlow: 'USER_PASSWORD_AUTH',
      ClientId: 'cognito-client-id',
      AuthParameters: {
        USERNAME: 'person',
        PASSWORD: 'secret',
      },
    })
    expect(requestBody(fetchMock.mock.calls[1]!)).toEqual({
      AccessToken: 'cognito-access-token',
    })

    const [libraryUrl, libraryRequest] = fetchMock.mock.calls[2]!
    expect(libraryUrl).toBe('https://library.example.test/v1/graphql')
    expect(libraryRequest?.headers).toMatchObject({
      Authorization: 'Bearer cognito-access-token',
      'x-platform-id': 'platform-1',
      'x-operator-id': 'platform-1',
    })
    const libraryBody = requestBody(fetchMock.mock.calls[2]!)
    expect(libraryBody.query).toContain('signInWithPlatform')
    expect(libraryBody.variables).toEqual({
      platformId: 'platform-1',
      accessToken: 'cognito-access-token',
      allowSignUp: true,
    })
  })

  it('refreshes Cognito and re-establishes the Library platform session', async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        jsonResponse({
          AuthenticationResult: {
            AccessToken: 'refreshed-access-token',
            ExpiresIn: 1800,
          },
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          data: {
            signIn: {
              id: 'library-user-after-refresh',
              email: 'refreshed@example.test',
            },
          },
        }),
      )
    vi.stubGlobal('fetch', fetchMock)

    await expect(
      refreshAccessToken('original-refresh-token', 'person'),
    ).resolves.toEqual({
      accessToken: 'refreshed-access-token',
      refreshToken: 'original-refresh-token',
      expiresAt: now / 1000 + 1800,
      userId: 'library-user-after-refresh',
      email: 'refreshed@example.test',
      username: 'person',
    })
    expect(requestBody(fetchMock.mock.calls[0]!)).toEqual({
      AuthFlow: 'REFRESH_TOKEN_AUTH',
      ClientId: 'cognito-client-id',
      AuthParameters: {
        REFRESH_TOKEN: 'original-refresh-token',
      },
    })
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('refreshes an expired stored session and preserves its email fallback', async () => {
    localStorage.setItem(
      'library_auth',
      JSON.stringify(storedTokens({ expiresAt: now / 1000 - 1 })),
    )
    const authChange = vi.fn()
    window.addEventListener('library-auth-change', authChange)
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        jsonResponse({
          AuthenticationResult: {
            AccessToken: 'refreshed-access-token',
            ExpiresIn: 3600,
          },
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          data: { signIn: { id: 'library-user-refreshed', email: null } },
        }),
      )
    vi.stubGlobal('fetch', fetchMock)

    expect(loadAuthTokens()).toBeNull()
    await expect(refreshStoredAuthTokens()).resolves.toMatchObject({
      accessToken: 'refreshed-access-token',
      refreshToken: 'stored-refresh-token',
      userId: 'library-user-refreshed',
      email: 'person@example.test',
    })
    expect(loadAuthTokens()).toMatchObject({
      accessToken: 'refreshed-access-token',
      email: 'person@example.test',
    })
    expect(authChange).toHaveBeenCalledTimes(1)
    expect((authChange.mock.calls[0]?.[0] as CustomEvent).detail).toEqual({
      reason: 'refreshed',
    })
    window.removeEventListener('library-auth-change', authChange)
  })

  it('clears the centralized session when refresh is rejected', async () => {
    localStorage.setItem(
      'library_auth',
      JSON.stringify(storedTokens({ expiresAt: now / 1000 - 1 })),
    )
    const authChange = vi.fn()
    window.addEventListener('library-auth-change', authChange)
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        jsonResponse(
          { __type: 'NotAuthorizedException', message: 'Refresh Token has expired' },
          400,
        ),
      )
    vi.stubGlobal('fetch', fetchMock)

    await expect(refreshStoredAuthTokens()).rejects.toThrow(
      'Refresh Token has expired',
    )
    expect(localStorage.getItem('library_auth')).toBeNull()
    expect((authChange.mock.calls[0]?.[0] as CustomEvent).detail).toEqual({
      reason: 'expired',
    })
    window.removeEventListener('library-auth-change', authChange)
  })

  it('expires a stored access token centrally when no refresh token exists', async () => {
    vi.restoreAllMocks()
    vi.useFakeTimers()
    vi.setSystemTime(now)
    const authChange = vi.fn()
    window.addEventListener('library-auth-change', authChange)

    storeAuthTokens(
      storedTokens({
        refreshToken: '',
        expiresAt: now / 1000 + 10,
      }),
    )
    await vi.advanceTimersByTimeAsync(10_000)

    expect(localStorage.getItem('library_auth')).toBeNull()
    expect(
      authChange.mock.calls.some(
        ([event]) => (event as CustomEvent).detail?.reason === 'expired',
      ),
    ).toBe(true)
    window.removeEventListener('library-auth-change', authChange)
  })

  it('keeps only the actor identity available while an expired token refreshes', () => {
    localStorage.setItem(
      'library_auth',
      JSON.stringify(storedTokens({ expiresAt: now / 1000 - 1 })),
    )

    expect(loadStoredAuthIdentity()).toEqual({
      userId: 'library-user-1',
      email: 'person@example.test',
      username: 'person',
    })
  })

  it('allows the localStorage adapter to be replaced without claiming native security', () => {
    const values = new Map<string, string>()
    const storage: AuthTokenStorage = {
      getItem: (key) => values.get(key) ?? null,
      setItem: (key, value) => values.set(key, value),
      removeItem: (key) => values.delete(key),
    }
    setAuthTokenStorage(storage)

    const tokens = storedTokens()
    storeAuthTokens(tokens)

    expect(loadAuthTokens()).toEqual(tokens)
    expect(values.has('library_auth')).toBe(true)
    clearAuthTokens()
    expect(values.has('library_auth')).toBe(false)
  })
})
