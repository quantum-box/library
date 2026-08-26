import { afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('./auth', async (importOriginal) => ({
  ...await importOriginal<typeof import('./auth')>(),
  getValidAuthTokens: vi.fn(async () => null),
}))

import {
  ApiKeysApiError,
  createApiKey,
  fetchApiKeys,
  isApiKeyPermissionError,
  revokeApiKey,
} from './apiKeysApi'

const target = { orgUsername: 'quantum-box', operatorId: 'tn_operator1' }

function graphqlResponse(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function requestOf(fetchMock: ReturnType<typeof vi.fn>, call = 0) {
  const init = fetchMock.mock.calls[call]?.[1] as RequestInit | undefined
  return {
    headers: init?.headers as Record<string, string>,
    body: JSON.parse(String(init?.body)) as {
      query: string
      variables: Record<string, unknown>
    },
  }
}

describe('apiKeysApi', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it('names the operator so a key can be verified against its organization', async () => {
    // /v1/graphql carries no organization in its path, so the operator has
    // to travel as a header. Without it the request is anonymous and the
    // listing comes back empty rather than refused.
    const fetchMock = vi.fn(async () => graphqlResponse({ data: { apiKeys: [] } }))
    vi.stubGlobal('fetch', fetchMock)

    await fetchApiKeys(target)

    expect(requestOf(fetchMock).headers['x-operator-id']).toBe('tn_operator1')
  })

  it('lists the keys the organization has issued', async () => {
    const fetchMock = vi.fn(async () =>
      graphqlResponse({
        data: {
          apiKeys: [
            { id: 'pk_1', name: 'ci', createdAt: '2026-08-26T00:00:00Z' },
          ],
        },
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const keys = await fetchApiKeys(target)

    expect(keys).toEqual([
      { id: 'pk_1', name: 'ci', createdAt: '2026-08-26T00:00:00Z' },
    ])
    expect(requestOf(fetchMock).body.variables).toEqual({
      orgUsername: 'quantum-box',
    })
  })

  it('returns the created key so its value can be shown once', async () => {
    const fetchMock = vi.fn(async () =>
      graphqlResponse({
        data: {
          createApiKey: {
            apiKey: {
              id: 'pk_2',
              name: 'batch',
              value: 'pk_secret',
              createdAt: '2026-08-26T00:00:00Z',
            },
          },
        },
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const created = await createApiKey(target, 'batch')

    expect(created.value).toBe('pk_secret')
    expect(requestOf(fetchMock).body.variables).toEqual({
      input: { organizationUsername: 'quantum-box', name: 'batch' },
    })
  })

  it('sends the key id the listing reported when revoking', async () => {
    const fetchMock = vi.fn(async () =>
      graphqlResponse({ data: { revokeApiKey: true } }),
    )
    vi.stubGlobal('fetch', fetchMock)

    await revokeApiKey(target, 'pk_1')

    expect(requestOf(fetchMock).body.variables).toEqual({
      input: { organizationUsername: 'quantum-box', apiKeyId: 'pk_1' },
    })
  })

  it('reports a refused action as a permission problem', async () => {
    // What a caller sees today when library:RevokeApiKey is not registered
    // upstream: the mutation resolves with FORBIDDEN rather than an HTTP
    // error, so the status code alone would read as success.
    const fetchMock = vi.fn(async () =>
      graphqlResponse({
        data: null,
        errors: [
          {
            message: 'Forbidden: action: library:RevokeApiKey',
            extensions: { code: 'FORBIDDEN' },
          },
        ],
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const error = await revokeApiKey(target, 'pk_1').catch((e: unknown) => e)

    expect(isApiKeyPermissionError(error)).toBe(true)
  })

  it('fails rather than reporting an empty list when the query errors', async () => {
    // Repository settings deliberately keeps partial data; here every field
    // is the whole answer, so a denied listing must not look like "no keys".
    const fetchMock = vi.fn(async () =>
      graphqlResponse({
        data: null,
        errors: [{ message: 'Something broke' }],
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const error = await fetchApiKeys(target).catch((e: unknown) => e)

    expect(error).toBeInstanceOf(ApiKeysApiError)
    expect((error as ApiKeysApiError).message).toBe('Something broke')
  })
})
