import { afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('./auth', async (importOriginal) => ({
  ...await importOriginal<typeof import('./auth')>(),
  getValidAuthTokens: vi.fn(async () => null),
}))

import {
  RepositorySettingsApiError,
  createRepositoryProperty,
  deleteRepositoryProperty,
  fetchRepositorySettings,
  updateRepositoryProperty,
  updateRepositorySettings,
} from './repositorySettingsApi'

const target = {
  orgUsername: 'quantum-box',
  repoUsername: 'library',
  operatorId: 'operator-1',
}

function graphqlResponse(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function requestBody(fetchMock: ReturnType<typeof vi.fn>, call = 0) {
  const init = fetchMock.mock.calls[call]?.[1] as RequestInit | undefined
  return JSON.parse(String(init?.body)) as {
    query: string
    variables: Record<string, unknown>
  }
}

describe('repositorySettingsApi', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    vi.clearAllMocks()
  })

  it('fetches repository metadata, visibility, policies, and Property definitions', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test/')
    vi.stubEnv('VITE_LIBRARY_ACCESS_TOKEN', 'access-token')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    const fetchMock = vi.fn(async (
      input: string | URL | Request,
      init?: RequestInit,
    ) => {
      void input
      void init
      return graphqlResponse({
        data: {
          repo: {
            id: 'repo-1',
            name: 'Library',
            username: 'library',
            description: 'Knowledge workspace',
            isPublic: false,
            policies: [{ userId: 'user-1', role: 'owner' }],
          },
          properties: [{
            id: 'property-1',
            name: 'Status',
            typ: 'SELECT',
            meta: {
              __typename: 'SelectType',
              options: [{ id: 'option-1', key: 'todo', name: 'Todo' }],
            },
          }],
        },
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(fetchRepositorySettings(target)).resolves.toEqual({
      repository: {
        id: 'repo-1',
        name: 'Library',
        username: 'library',
        description: 'Knowledge workspace',
        isPublic: false,
      },
      properties: [{
        id: 'property-1',
        name: 'Status',
        typ: 'SELECT',
        meta: {
          __typename: 'SelectType',
          options: [{ id: 'option-1', key: 'todo', name: 'Todo' }],
        },
      }],
      policies: [{ userId: 'user-1', role: 'owner' }],
    })

    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0]?.[0]).toBe('https://library.example.test/v1/graphql')
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit
    expect(init.headers).toMatchObject({
      Authorization: 'Bearer access-token',
      'x-platform-id': 'platform-1',
      'x-operator-id': 'operator-1',
    })
    expect(requestBody(fetchMock)).toMatchObject({
      query: expect.stringContaining('LibraryClientRepositorySettings'),
      variables: {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
      },
    })
  })

  it('surfaces HTTP permission failures without attempting another endpoint', async () => {
    const fetchMock = vi.fn(async () => new Response('forbidden', { status: 403 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(fetchRepositorySettings(target)).rejects.toMatchObject({
      kind: 'permission',
      status: 403,
      message: 'You do not have permission to manage this repository.',
    })
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('classifies GraphQL permission errors and does not use REST fallback', async () => {
    const fetchMock = vi.fn(async (
      input: string | URL | Request,
      init?: RequestInit,
    ) => {
      void input
      void init
      return graphqlResponse({
        errors: [{
          message: 'permission denied for repository',
          extensions: { code: 'FORBIDDEN', status: 403 },
        }],
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(updateRepositorySettings(target, {
      description: 'Private workspace',
      isPublic: false,
    })).rejects.toMatchObject({ kind: 'permission', status: 403 })
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetchMock.mock.calls[0]?.[0]).toMatch(/\/v1\/graphql$/)
  })

  it('creates a Select Property using the PropertyInput contract from apps/web', async () => {
    const fetchMock = vi.fn(async () => graphqlResponse({
      data: {
        addProperty: {
          id: 'property-status',
          name: 'Status',
          typ: 'SELECT',
          meta: { options: [{ id: 'option-todo', key: 'todo', name: 'Todo' }] },
        },
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(createRepositoryProperty(target, {
      name: ' Status ',
      type: 'SELECT',
      options: [{ identifier: 'todo', label: 'Todo' }],
    })).resolves.toMatchObject({ id: 'property-status', typ: 'SELECT' })

    expect(requestBody(fetchMock)).toMatchObject({
      query: expect.stringContaining('LibraryClientAddRepositoryProperty'),
      variables: {
        input: {
          orgUsername: 'quantum-box',
          repoUsername: 'library',
          propertyName: 'Status',
          propertyType: 'SELECT',
          meta: {
            select: [{ identifier: 'todo', label: 'Todo' }],
          },
        },
      },
    })
  })

  it('sends stable option IDs when updating a Select Property', async () => {
    const fetchMock = vi.fn(async () => graphqlResponse({
      data: {
        updateProperty: {
          id: 'property-status',
          name: 'Status',
          typ: 'SELECT',
          meta: { options: [{ id: 'op_existing', key: 'todo', name: 'To do' }] },
        },
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await updateRepositoryProperty(target, 'property-status', {
      name: 'Status',
      type: 'SELECT',
      options: [
        { id: ' op_existing ', identifier: 'todo', label: 'To do' },
        { identifier: 'done', label: 'Done' },
      ],
    })

    expect(requestBody(fetchMock).variables).toEqual({
      id: 'property-status',
      input: {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        propertyName: 'Status',
        propertyType: 'SELECT',
        meta: {
          select: [
            { id: 'op_existing', identifier: 'todo', label: 'To do' },
            { identifier: 'done', label: 'Done' },
          ],
        },
      },
    })
  })

  it('updates Property name and type with type-specific Relation metadata', async () => {
    const fetchMock = vi.fn(async () => graphqlResponse({
      data: {
        updateProperty: {
          id: 'property-owner',
          name: 'Parent',
          typ: 'RELATION',
          meta: { databaseId: 'database-2' },
        },
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await updateRepositoryProperty(target, 'property-owner', {
      name: 'Parent',
      type: 'RELATION',
      relationDatabaseId: 'database-2',
    })

    expect(requestBody(fetchMock).variables).toEqual({
      id: 'property-owner',
      input: {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        propertyName: 'Parent',
        propertyType: 'RELATION',
        meta: { relation: 'database-2' },
      },
    })
  })

  it('validates Relation metadata before issuing a mutation', async () => {
    const fetchMock = vi.fn()
    vi.stubGlobal('fetch', fetchMock)

    await expect(createRepositoryProperty(target, {
      name: 'Parent',
      type: 'RELATION',
    })).rejects.toEqual(expect.objectContaining<Partial<RepositorySettingsApiError>>({
      kind: 'validation',
      status: 422,
    }))
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('deletes a Property only after GraphQL confirms its id', async () => {
    const fetchMock = vi.fn(async () => graphqlResponse({
      data: { deleteProperty: 'property-old' },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(deleteRepositoryProperty(target, 'property-old')).resolves.toBeUndefined()
    expect(requestBody(fetchMock)).toMatchObject({
      query: expect.stringContaining('LibraryClientDeleteRepositoryProperty'),
      variables: {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        id: 'property-old',
      },
    })
  })

  it('updates description and visibility through UpdateRepoInput', async () => {
    const fetchMock = vi.fn(async () => graphqlResponse({
      data: {
        updateRepo: {
          id: 'repo-1',
          name: 'Library',
          username: 'library',
          description: 'Public knowledge workspace',
          isPublic: true,
        },
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(updateRepositorySettings(target, {
      description: ' Public knowledge workspace ',
      isPublic: true,
    })).resolves.toMatchObject({ description: 'Public knowledge workspace', isPublic: true })
    expect(requestBody(fetchMock).variables).toEqual({
      input: {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        description: 'Public knowledge workspace',
        isPublic: true,
      },
    })
  })

  it('fails closed instead of pretending that null can clear a description', async () => {
    const fetchMock = vi.fn()
    vi.stubGlobal('fetch', fetchMock)

    await expect(updateRepositorySettings(target, {
      description: '   ',
      isPublic: false,
    })).rejects.toMatchObject({
      kind: 'validation',
      status: 422,
      message: expect.stringContaining('cannot remove'),
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('omits description when only visibility changes', async () => {
    const fetchMock = vi.fn(async () => graphqlResponse({
      data: {
        updateRepo: {
          id: 'repo-1',
          name: 'Library',
          username: 'library',
          description: null,
          isPublic: true,
        },
      },
    }))
    vi.stubGlobal('fetch', fetchMock)

    await updateRepositorySettings(target, { isPublic: true })
    expect(requestBody(fetchMock).variables).toEqual({
      input: {
        orgUsername: 'quantum-box',
        repoUsername: 'library',
        isPublic: true,
      },
    })
  })
})
