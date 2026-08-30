import { afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('./photonEngine/client', () => ({
  deleteClientEngineRecord: vi.fn(),
  listClientEngineRecords: vi.fn(async () => []),
  patchClientEngineRecord: vi.fn(),
  upsertClientEngineRecord: vi.fn(),
}))
import { appKitConfig } from '../app/kitConfig'
import { clearAuthTokens } from './auth'
import {
  createLibraryOrganization,
  createLibraryRepository,
  createServerRecord,
  fetchLibraryOrganizations,
  fetchLibraryRecords,
  fetchLibraryRepoTableData,
  fetchLibraryRepositories,
  fetchLibraryRepositoryProfile,
  libraryDataToRecord,
  RecordPropertyMappingError,
  toRecord,
  updateServerRecord,
  type LibraryDataItem,
  type LibraryProperty,
  type ServerRecord,
} from './recordsApi'

describe('recordsApi', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    vi.clearAllMocks()
    clearAuthTokens()
  })

  it('normalizes server record projections for the Yjs record cache', () => {
    const serverRecord: ServerRecord = {
      id: 'f3cc94d8-cc78-4fd3-a407-4793ea2f537c',
      identifier: 'PLT-1200',
      title: 'Persist record writes',
      description: 'Route frontend writes through the server.',
      status: 'in_progress',
      priority: 'high',
      assignee: '',
      labels: ['Feature', 'sync'],
      project: 'Photon Core',
      created_at: '2026-05-08 03:30:00',
      updated_at: '2026-05-08 03:31:00',
    }

    expect(toRecord(serverRecord)).toEqual({
      id: 'f3cc94d8-cc78-4fd3-a407-4793ea2f537c',
      identifier: 'PLT-1200',
      title: 'Persist record writes',
      description: 'Route frontend writes through the server.',
      status: 'in_progress',
      priority: 'high',
      assignee: null,
      labels: ['Feature', 'sync'],
      project: 'Photon Core',
      createdAt: '2026-05-08 03:30:00',
      updatedAt: '2026-05-08 03:31:00',
    })
  })

  it('keeps older server rows renderable during migration', () => {
    expect(
      toRecord({
        id: 'legacy-record',
        title: 'Legacy row',
        labels: '["legacy"]',
      })
    ).toMatchObject({
      id: 'legacy-record',
      identifier: 'legacy-record',
      status: 'backlog',
      priority: 'none',
      labels: ['legacy'],
      project: appKitConfig.records.defaultProject,
    })
  })

  it('projects Library repo data into the client record model', () => {
    const properties: LibraryProperty[] = [
      {
        id: 'prop-status',
        name: 'Status',
        typ: 'Select',
        meta: {
          options: [
            { id: 'opt-progress', key: 'progress', name: 'In Progress' },
          ],
        },
      },
      {
        id: 'prop-priority',
        name: 'Priority',
        typ: 'Select',
        meta: {
          options: [
            { id: 'opt-high', key: 'high', name: 'High' },
          ],
        },
      },
      {
        id: 'prop-tags',
        name: 'Tags',
        typ: 'MultiSelect',
        meta: {
          options: [
            { id: 'tag-api', key: 'api', name: 'API' },
            { id: 'tag-sync', key: 'sync', name: 'Sync' },
          ],
        },
      },
      { id: 'prop-owner', name: '担当', typ: 'String' },
      { id: 'prop-body', name: 'Markdown', typ: 'Markdown' },
    ]
    const item: LibraryDataItem = {
      id: 'data-1',
      name: 'GraphQL integration',
      createdAt: '2026-05-21T01:00:00.000Z',
      updatedAt: '2026-05-21T02:00:00.000Z',
      propertyData: [
        { propertyId: 'prop-status', value: { optionId: 'opt-progress' } },
        { propertyId: 'prop-priority', value: { optionId: 'opt-high' } },
        { propertyId: 'prop-tags', value: { optionIds: ['tag-api', 'tag-sync'] } },
        { propertyId: 'prop-owner', value: { string: 'Library Team' } },
        { propertyId: 'prop-body', value: { markdown: 'Repo data from Library API.' } },
      ],
    }

    expect(libraryDataToRecord(item, properties, 'Library Repo')).toEqual({
      id: 'data-1',
      identifier: 'data-1',
      title: 'GraphQL integration',
      status: 'in_progress',
      priority: 'high',
      assignee: 'Library Team',
      labels: ['API', 'Sync'],
      project: 'Library Repo',
      createdAt: '2026-05-21T01:00:00.000Z',
      updatedAt: '2026-05-21T02:00:00.000Z',
      description: 'Repo data from Library API.',
    })
  })

  it('fetches Library repo table data through GraphQL data-list + properties', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'quantum-box')
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    vi.stubEnv('VITE_LIBRARY_OPERATOR_ID', 'operator-1')
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({
      data: {
        repo: {
          id: 'repo-1',
          name: 'Docs',
          dataList: {
            items: [
              {
                id: 'data-1',
                name: 'First page',
                propertyData: [
                  { propertyId: 'prop-1', value: { string: 'Alpha' } },
                ],
              },
            ],
          },
          properties: [{ id: 'prop-1', name: 'Title', typ: 'String', meta: null }],
        },
      },
    }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })))

    await expect(fetchLibraryRepoTableData({
      org: 'quantum-box',
      repo: 'docs',
      operatorId: 'operator-1',
    })).resolves.toEqual({
      items: [
        {
          id: 'data-1',
          name: 'First page',
          propertyData: [{ propertyId: 'prop-1', value: { string: 'Alpha' } }],
        },
      ],
      properties: [{ id: 'prop-1', name: 'Title', typ: 'String', meta: null }],
      repoName: 'Docs',
    })
  })

  it('normalizes SCREAMING_SNAKE_CASE Property types from the live API', async () => {
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    vi.stubEnv('VITE_LIBRARY_OPERATOR_ID', 'operator-1')
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({
      data: {
        repo: {
          id: 'repo-1',
          name: 'Docs',
          dataList: { items: [] },
          properties: [
            { id: 'prop-id', name: 'id', typ: 'ID', meta: null },
            { id: 'prop-content', name: 'content', typ: 'MARKDOWN', meta: null },
            { id: 'prop-tags', name: 'tags', typ: 'MULTI_SELECT', meta: null },
          ],
        },
      },
    }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })))

    const payload = await fetchLibraryRepoTableData({
      org: 'quantum-box',
      repo: 'docs',
      operatorId: 'operator-1',
    })
    expect(payload.properties.map((property) => property.typ)).toEqual([
      'Id',
      'Markdown',
      'MultiSelect',
    ])
  })

  it('loads every GraphQL data-list page using the documented paginator', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    const fetchMock = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as {
        variables: { page: number }
      }
      const page = body.variables.page
      return Response.json({
        data: {
          repo: {
            id: 'repo-1',
            name: 'Docs',
            dataList: {
              items: [{ id: `data-${page}`, name: `Page ${page}`, propertyData: [] }],
              paginator: {
                currentPage: page,
                itemsPerPage: 100,
                totalItems: 2,
                totalPages: 2,
              },
            },
            properties: [],
          },
        },
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(fetchLibraryRepoTableData({ org: 'acme', repo: 'docs' })).resolves.toMatchObject({
      items: [
        { id: 'data-1', name: 'Page 1' },
        { id: 'data-2', name: 'Page 2' },
      ],
    })
    expect(fetchMock).toHaveBeenCalledTimes(2)
    expect(JSON.parse(String(fetchMock.mock.calls[1]?.[1]?.body)).variables.page).toBe(2)
  })

  it('maps canonical record fields to typed repository Properties on create', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'acme')
    vi.stubEnv('VITE_LIBRARY_REPO', 'work')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    const properties: LibraryProperty[] = [
      {
        id: 'status',
        name: 'Status',
        typ: 'Select',
        meta: { options: [{ id: 'status-progress', key: 'progress', name: 'In progress' }] },
      },
      {
        id: 'priority',
        name: 'Priority',
        typ: 'Select',
        meta: { options: [{ id: 'priority-high', key: 'high', name: 'High' }] },
      },
      { id: 'owner', name: 'Owner', typ: 'String' },
      {
        id: 'labels',
        name: 'Labels',
        typ: 'MultiSelect',
        meta: { options: [{ id: 'label-api', key: 'api', name: 'API' }] },
      },
      { id: 'body', name: 'Body', typ: 'Markdown' },
      { id: 'project', name: 'Project', typ: 'String' },
    ]
    const fetchMock = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as {
        query: string
        variables: { input?: { propertyData?: unknown[] } }
      }
      if (body.query.includes('LibraryClientProperties')) {
        return Response.json({ data: { properties } })
      }
      return Response.json({
        data: {
          addData: {
            id: 'data-new',
            name: 'Typed record',
            propertyData: [
              { propertyId: 'status', value: { optionId: 'status-progress' } },
              { propertyId: 'priority', value: { optionId: 'priority-high' } },
              { propertyId: 'owner', value: { string: 'Ada' } },
              { propertyId: 'labels', value: { optionIds: ['label-api'] } },
              { propertyId: 'body', value: { markdown: 'Canonical body' } },
              { propertyId: 'project', value: { string: 'Work' } },
            ],
          },
        },
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(createServerRecord({
      title: 'Typed record',
      status: 'in_progress',
      priority: 'high',
      assignee: 'Ada',
      labels: ['API'],
      description: 'Canonical body',
      project: 'Work',
    })).resolves.toMatchObject({
      id: 'data-new',
      status: 'in_progress',
      priority: 'high',
      assignee: 'Ada',
      labels: ['API'],
      description: 'Canonical body',
      project: 'Work',
    })

    const mutationCall = fetchMock.mock.calls.find(([, init]) =>
      String(init?.body).includes('LibraryClientAddData')
    )
    const mutationBody = JSON.parse(String(mutationCall?.[1]?.body)) as {
      variables: { input: { dataName: string; propertyData: unknown[] } }
    }
    expect(mutationBody.variables.input).toMatchObject({
      dataName: 'Typed record',
      propertyData: [
        { propertyId: 'status', value: { select: 'status-progress' } },
        { propertyId: 'priority', value: { select: 'priority-high' } },
        { propertyId: 'owner', value: { string: 'Ada' } },
        { propertyId: 'labels', value: { multiSelect: ['label-api'] } },
        { propertyId: 'body', value: { markdown: 'Canonical body' } },
        { propertyId: 'project', value: { string: 'Work' } },
      ],
    })
  })

  it('fails explicitly before create when a supplied standard field has no Property', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'acme')
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    const fetchMock = vi.fn(async () => Response.json({ data: { properties: [] } }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(createServerRecord({ title: 'Unsafe', status: 'todo' })).rejects.toBeInstanceOf(
      RecordPropertyMappingError
    )
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('sends the complete canonical Property payload and preserves untouched values', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'acme')
    vi.stubEnv('VITE_LIBRARY_REPO', 'work')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    const properties: LibraryProperty[] = [
      {
        id: 'status',
        name: 'Status',
        typ: 'Select',
        meta: {
          options: [
            { id: 'status-todo', key: 'todo', name: 'To do' },
            { id: 'status-done', key: 'done', name: 'Done' },
          ],
        },
      },
      { id: 'owner', name: 'Owner', typ: 'String' },
      { id: 'body', name: 'Body', typ: 'Markdown' },
    ]
    const fetchMock = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as { query: string }
      if (body.query.includes('LibraryClientDataDetail')) {
        return Response.json({
          data: {
            data: {
              id: 'data-1',
              name: 'Existing',
              propertyData: [
                { propertyId: 'status', value: { optionId: 'status-todo' } },
                { propertyId: 'owner', value: { string: 'Ada' } },
                { propertyId: 'body', value: { markdown: 'Keep this body' } },
                { propertyId: 'future-property', value: { string: 'preserve me' } },
              ],
            },
            properties,
          },
        })
      }
      return Response.json({
        data: {
          updateData: {
            id: 'data-1',
            name: 'Existing',
            propertyData: [{ propertyId: 'status', value: { optionId: 'status-done' } }],
          },
        },
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await expect(updateServerRecord('data-1', { status: 'done' })).resolves.toMatchObject({
      id: 'data-1',
      status: 'done',
      assignee: 'Ada',
      description: 'Keep this body',
    })
    const mutationCall = fetchMock.mock.calls.find(([, init]) =>
      String(init?.body).includes('LibraryClientUpdateData')
    )
    const mutationBody = JSON.parse(String(mutationCall?.[1]?.body)) as {
      variables: { input: { propertyData: Array<{ propertyId: string }> } }
    }
    expect(mutationBody.variables.input.propertyData).toEqual([
      { propertyId: 'status', value: { select: 'status-done' } },
      { propertyId: 'owner', value: { string: 'Ada' } },
      { propertyId: 'body', value: { markdown: 'Keep this body' } },
    ])
    expect(mutationBody.variables.input.propertyData).not.toContainEqual(
      expect.objectContaining({ propertyId: 'future-property' })
    )
  })

  it('fetches Library repo data through the GraphQL API contract', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'quantum-box')
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    vi.stubEnv('VITE_LIBRARY_OPERATOR_ID', 'operator-1')
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({
      data: {
        repo: {
          id: 'repo-1',
          name: 'Docs',
          dataList: {
            items: [
              {
                id: 'data-1',
                name: 'First page',
                createdAt: '2026-05-21T01:00:00.000Z',
                updatedAt: '2026-05-21T01:30:00.000Z',
                propertyData: [],
              },
            ],
          },
          properties: [],
        },
      },
    }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })))

    await expect(fetchLibraryRecords()).resolves.toMatchObject([
      {
        id: 'data-1',
        title: 'First page',
        project: 'Docs',
      },
    ])
    expect(fetch).toHaveBeenCalledWith(
      'https://library.example.test/v1/graphql',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'x-platform-id': 'platform-1',
          'x-operator-id': 'operator-1',
        }),
        body: expect.stringContaining('"org":"quantum-box"'),
      })
    )
    expect(fetch).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: expect.stringContaining('"repo":"docs"'),
      })
    )
  })

  it('fetches Library repositories for the sidebar from the GraphQL API', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'quantum-box')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({
      data: {
        organization: {
          id: 'org-1',
          name: 'Quantum Box',
          username: 'quantum-box',
          repos: [
            {
              id: 'repo-1',
              username: 'docs',
              name: 'Docs',
              description: 'Documentation repo',
            },
          ],
        },
      },
    })))

    await expect(fetchLibraryRepositories()).resolves.toEqual([
      {
        id: 'repo-1',
        username: 'docs',
        name: 'Docs',
        description: 'Documentation repo',
        orgUsername: 'quantum-box',
        operatorId: 'org-1',
        platformTenantId: 'platform-1',
      },
    ])
    expect(fetch).toHaveBeenCalledWith(
      'https://library.example.test/v1/graphql',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('"org":"quantum-box"'),
      })
    )
  })

  it('creates a Library organization through the authenticated GraphQL API', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({
      data: {
        createOrganization: {
          id: 'org-2',
          name: 'Acme Research',
          username: 'acme-research',
        },
      },
    })))

    await expect(createLibraryOrganization({
      name: ' Acme Research ',
      username: ' acme-research ',
    })).resolves.toEqual({
      id: 'org-2',
      name: 'Acme Research',
      username: 'acme-research',
    })
    expect(fetch).toHaveBeenCalledWith(
      'https://library.example.test/v1/graphql',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({
          Authorization: 'Bearer token',
          'x-platform-id': 'platform-1',
        }),
        body: expect.stringContaining('LibraryClientCreateOrganization'),
      })
    )
    expect(fetch).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: expect.stringContaining('"username":"acme-research"'),
      })
    )
  })

  it('creates a Library repository with the selected organization context', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({
      data: {
        createRepo: {
          id: 'repo-2',
          name: 'Research Library',
          username: 'research-library',
          description: 'Research notes',
          orgUsername: 'quantum-box',
          isPublic: false,
        },
      },
    })))

    await expect(createLibraryRepository({
      orgUsername: ' quantum-box ',
      operatorId: 'org-1',
      name: ' Research Library ',
      username: ' research-library ',
      description: ' Research notes ',
      isPublic: false,
    })).resolves.toMatchObject({
      id: 'repo-2',
      username: 'research-library',
      orgUsername: 'quantum-box',
    })
    expect(fetch).toHaveBeenCalledWith(
      'https://library.example.test/v1/graphql',
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: 'Bearer token',
          'x-operator-id': 'org-1',
        }),
        body: expect.stringContaining('LibraryClientCreateRepository'),
      }),
    )
    expect(fetch).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        body: expect.stringContaining('"userId":"user-1"'),
      }),
    )
  })

  it('falls back to the REST repository list when organization GraphQL repos are empty', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))

    vi.stubGlobal('fetch', vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const requestUrl = String(url)
      if (requestUrl.endsWith('/v1beta/repos')) {
        return Response.json([
          {
            id: 'repo-1',
            username: 'docs',
            name: 'Docs',
            description: 'Documentation repo',
            organization_id: 'org-1',
            org_username: 'library-docs',
          },
        ])
      }

      const body = typeof init?.body === 'string' ? init.body : ''
      if (body.includes('LibraryClientMeOrganizations')) {
        return Response.json({
          data: {
            me: {
              id: 'user-1',
              email: 'test@example.com',
              tenantIdList: ['org-1'],
              organizations: [
                {
                  id: 'org-1',
                  operatorName: 'quantum-box',
                  platformTenantId: 'platform-1',
                },
              ],
            },
          },
        })
      }

      return Response.json({
        data: {
          organization: {
            id: 'org-1',
            name: 'Quantum Box',
            username: 'quantum-box',
            repos: [],
          },
        },
      })
    }))

    await expect(fetchLibraryOrganizations()).resolves.toEqual([
      {
        id: 'org-1',
        operatorName: 'quantum-box',
        platformTenantId: 'platform-1',
        repos: [
          {
            id: 'repo-1',
            username: 'docs',
            name: 'Docs',
            description: 'Documentation repo',
            orgUsername: 'library-docs',
            operatorId: 'org-1',
            platformTenantId: 'platform-1',
          },
        ],
      },
    ])
    expect(fetch).toHaveBeenCalledWith(
      'https://library.example.test/v1beta/repos',
      expect.objectContaining({
        headers: expect.objectContaining({ Authorization: 'Bearer token' }),
      }),
    )
  })

  it('lists no organizations while signed out instead of every repository the API returns', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')

    vi.stubGlobal('fetch', vi.fn(async () => Response.json([
      {
        id: 'repo-1',
        username: 'docs',
        name: 'Docs',
        organization_id: 'someone-elses-org',
        org_username: 'someone-else',
      },
    ])))

    await expect(fetchLibraryOrganizations()).resolves.toEqual([])
    expect(fetch).not.toHaveBeenCalled()
  })

  it('reads a repository profile without the session token when asked anonymously', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({
      id: 'repo-1',
      name: 'Docs',
      username: 'docs',
      description: 'Documentation repo',
      is_public: true,
      organization_id: 'org-1',
      org_username: 'library-docs',
    })))

    await expect(fetchLibraryRepositoryProfile({
      org: 'library-docs',
      repo: 'docs',
      anonymous: true,
    })).resolves.toEqual({
      id: 'repo-1',
      name: 'Docs',
      username: 'docs',
      orgUsername: 'library-docs',
      description: 'Documentation repo',
      isPublic: true,
    })

    const [, init] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(vi.mocked(fetch).mock.calls[0][0])
      .toBe('https://library.example.test/v1beta/repos/library-docs/docs')
    expect(init.headers).not.toHaveProperty('Authorization')
  })

  it('surfaces the repository status so a private repo stays distinguishable from a missing one', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubGlobal('fetch', vi.fn(async () => new Response('forbidden', { status: 403 })))

    await expect(fetchLibraryRepositoryProfile({
      org: 'library-docs',
      repo: 'private',
      anonymous: true,
    })).rejects.toMatchObject({ status: 403 })
  })

  it('reads repository table data anonymously without attaching the session token', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({
      data: {
        repo: {
          name: 'Docs',
          properties: [],
          dataList: { items: [], paginator: { totalPages: 1 } },
        },
      },
    })))

    await fetchLibraryRepoTableData({ org: 'library-docs', repo: 'docs', anonymous: true })

    const [, init] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(init.headers).not.toHaveProperty('Authorization')
  })

  it('still attaches the session token for a normal repository read', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({
      id: 'repo-1',
      name: 'Docs',
      username: 'docs',
      description: null,
      is_public: false,
      organization_id: 'org-1',
      org_username: 'library-docs',
    })))

    await fetchLibraryRepositoryProfile({ org: 'library-docs', repo: 'docs' })

    const [, init] = vi.mocked(fetch).mock.calls[0] as [string, RequestInit]
    expect(init.headers).toMatchObject({ Authorization: 'Bearer token' })
  })
})
