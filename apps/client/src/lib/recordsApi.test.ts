import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * The engine is stubbed, so these tests exercise the Library API mapping and
 * the destination of a write, not Photon. The push-and-report helpers default
 * to `queued`, the outcome that leaves the caller with the record it wrote;
 * the verdicts belong in `photonEngine/client.test.ts`, against a real engine.
 */
vi.mock('./photonEngine/client', () => ({
  deleteAndPushClientEngineRecord: vi.fn(async () => ({ status: 'queued', record: null })),
  deleteClientEngineRecord: vi.fn(),
  ingestClientEngineRecords: vi.fn(async () => undefined),
  listClientEngineRecords: vi.fn(async () => []),
  newClientEngineRecordId: vi.fn((prefix?: string) => `${prefix ? `${prefix}_` : ''}stub-id`),
  patchAndPushClientEngineRecord: vi.fn(async () => ({ status: 'queued', record: null })),
  patchClientEngineRecord: vi.fn(),
  subscribeClientEngineRollbacks: vi.fn(() => () => undefined),
  upsertAndPushClientEngineRecord: vi.fn(async () => ({ status: 'queued', record: null })),
  upsertClientEngineRecord: vi.fn(),
}))
import { appKitConfig } from '../app/kitConfig'
import { clearAuthTokens } from './auth'
import * as photonEngine from './photonEngine/client'
import {
  __testOnly as libraryCollections,
  rememberLibraryRepositories,
} from './photonEngine/libraryCollections'
import {
  createLibraryOrganization,
  createLibraryRecordsResource,
  createLibraryRepository,
  createServerRecord,
  deleteServerRecord,
  fetchLibraryOrganizations,
  fetchLibraryRepoTableData,
  fetchLibraryRepositories,
  fetchServerRecords,
  fetchLibraryRepositoryProfile,
  libraryDataToRecord,
  RecordPropertyMappingError,
  toRecord,
  updateServerRecord,
  type LibraryDataItem,
  type LibraryProperty,
  type ServerRecord,
} from './recordsApi'
import type { DatabaseRecord } from '../data/mock'

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
      // The repository's canonical id comes back with the table: it is what
      // names the collection these rows are cached in.
      repoId: 'repo-1',
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

  /**
   * The create path after Stage 3b: the client names the record, checks it
   * against the repository's schema, and hands it to Photon. The write itself
   * is the engine's — which is what makes it survive being offline — so what
   * is asserted here is the handoff, not an HTTP body. The body belongs to
   * `createLibraryRecordsResource`, and is asserted there.
   */
  it('names a record itself and hands the write to the repository collection', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'acme')
    vi.stubEnv('VITE_LIBRARY_REPO', 'work')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    rememberLibraryRepositories([{ databaseId: 'repo-acme', org: 'acme', repo: 'work' }])
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
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({ data: { properties } })))

    const created = await createServerRecord({
      title: 'Typed record',
      status: 'in_progress',
      priority: 'high',
      assignee: 'Ada',
      labels: ['API'],
      description: 'Canonical body',
      project: 'Work',
    })

    // `data_` is not decoration: library-api parses a DataId by its prefix and
    // rejects anything else, so a client-minted id has to carry it.
    expect(created.id).toMatch(/^data_/)
    expect(created).toMatchObject({
      status: 'in_progress',
      priority: 'high',
      assignee: 'Ada',
      labels: ['API'],
      description: 'Canonical body',
      project: 'Work',
      orgUsername: 'acme',
      repoUsername: 'work',
    })
    expect(photonEngine.upsertAndPushClientEngineRecord).toHaveBeenCalledWith(
      'data:repo-acme',
      created.id,
      expect.objectContaining({ id: created.id, title: 'Typed record' })
    )

    libraryCollections.reset()
  })

  /**
   * "Before create" now means "before the operation is queued". The check runs
   * up front precisely so the user hears about a repository whose schema has
   * nowhere to put a status at the moment they set one — rather than after the
   * push reaches the server and the write is rolled back out from under them.
   */
  it('fails explicitly before queueing when a supplied standard field has no Property', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'acme')
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    rememberLibraryRepositories([{ databaseId: 'repo-docs', org: 'acme', repo: 'docs' }])
    const fetchMock = vi.fn(async () => Response.json({ data: { properties: [] } }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(createServerRecord({ title: 'Unsafe', status: 'todo' })).rejects.toBeInstanceOf(
      RecordPropertyMappingError
    )
    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(photonEngine.upsertAndPushClientEngineRecord).not.toHaveBeenCalled()

    libraryCollections.reset()
  })

  /**
   * The mirror image, and the one a whole-record write path gets wrong by
   * default. A `DatabaseRecord` always carries a status and a priority, so the
   * resource cannot read their presence as a request — only what the caller
   * passed is that. A repository defining neither Property must still accept a
   * record that never mentioned one.
   */
  it('creates against a repository with no Status Property when none was asked for', async () => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'acme')
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    rememberLibraryRepositories([{ databaseId: 'repo-docs', org: 'acme', repo: 'docs' }])
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({ data: { properties: [] } })))

    const created = await createServerRecord({ title: 'No status here' })

    // The record still gets the UI's defaults...
    expect(created).toMatchObject({ status: 'todo', priority: 'none' })
    // ...and the write went out rather than being refused for them.
    expect(photonEngine.upsertAndPushClientEngineRecord).toHaveBeenCalled()

    libraryCollections.reset()
  })

  /**
   * And the resource itself must drop those defaults rather than refuse them,
   * because what it is handed is the stored record, not the caller's input.
   */
  it('drops a default the repository has no Property for instead of refusing the write', async () => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    const bodies: unknown[] = []
    vi.stubGlobal('fetch', vi.fn(async (url: string, init: RequestInit = {}) => {
      if (String(url).endsWith('/v1/graphql')) return Response.json({ data: { properties: [] } })
      bodies.push(JSON.parse(String(init.body)))
      return Response.json({ id: 'data-1', name: 'No status here', items: [] })
    }))

    const resource = createLibraryRecordsResource({ org: 'acme', repo: 'docs' })
    await expect(
      resource.upsert('data-1', {
        id: 'data-1',
        identifier: 'data-1',
        title: 'No status here',
        status: 'todo',
        priority: 'none',
        assignee: null,
        labels: [],
        project: 'docs',
        createdAt: '2026-08-30T00:00:00.000Z',
        updatedAt: '2026-08-30T00:00:00.000Z',
        description: '',
      })
    ).resolves.toMatchObject({ id: 'data-1' })

    expect(bodies).toEqual([{ name: 'No status here', property_data: [] }])
  })

  it('fetches a pinned build\'s repo data through the GraphQL API contract', async () => {
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

    await expect(fetchServerRecords()).resolves.toMatchObject([
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

describe('createLibraryRecordsResource', () => {
  const target = { org: 'quantum-box', repo: 'docs', operatorId: 'operator-1', repoName: 'Docs' }

  // `standardRecordPropertyData` refuses to guess, so a record carrying
  // `status` or `priority` needs a matching Property or it throws. These are
  // `String` to keep the routing tests below about routing; the Select
  // roundtrip has its own fixture.
  const properties = [
    { id: 'prop-status', name: 'status', typ: 'String', meta: null },
    { id: 'prop-priority', name: 'priority', typ: 'String', meta: null },
  ]

  // What a real repository looks like: the standard fields are Select, so
  // every REST write has to encode an option id.
  const selectProperties = [
    {
      id: 'prop-status',
      name: 'status',
      typ: 'Select',
      meta: { options: [{ id: 'opt-todo', key: 'todo', name: 'Todo' }] },
    },
    {
      id: 'prop-priority',
      name: 'priority',
      typ: 'Select',
      meta: { options: [{ id: 'opt-none', key: 'none', name: 'None' }] },
    },
  ]

  function record(overrides: Partial<DatabaseRecord> = {}): DatabaseRecord {
    return {
      id: 'local-1',
      identifier: 'PLT-1',
      title: 'Route the first write',
      status: 'todo',
      priority: 'none',
      assignee: null,
      labels: [],
      project: 'Docs',
      createdAt: '2026-08-30T00:00:00.000Z',
      updatedAt: '2026-08-30T00:00:00.000Z',
      description: '',
      ...overrides,
    }
  }

  /**
   * One mock for every call the resource makes, routed the way the real API
   * routes: Properties and the record detail come over GraphQL, the writes over
   * REST. Anything the test did not anticipate 404s, so a method that reaches
   * for an endpoint it should not fails loudly.
   */
  function stubLibrary(
    rest: (url: string, init: RequestInit) => Response | undefined,
    schema: typeof properties | typeof selectProperties = properties
  ) {
    const calls: Array<{ url: string; method: string; body: unknown }> = []
    vi.stubGlobal('fetch', vi.fn(async (url: string, init: RequestInit = {}) => {
      const method = init.method ?? 'GET'
      const body = init.body ? JSON.parse(String(init.body)) as Record<string, unknown> : undefined
      calls.push({ url, method, body })
      if (url.endsWith('/v1/graphql')) {
        const query = String((body as { query?: string } | undefined)?.query ?? '')
        if (query.includes('LibraryClientDataDetail')) {
          return Response.json({ data: {
            data: {
              id: 'data-1',
              name: 'Route the first write',
              propertyData: [{ propertyId: 'prop-status', value: { string: 'todo' } }],
            },
            properties: schema,
          } })
        }
        return Response.json({ data: { properties: schema } })
      }
      return rest(url, init) ?? new Response('not found', { status: 404 })
    }))
    return calls
  }

  beforeEach(() => {
    vi.stubEnv('VITE_LIBRARY_ORG', 'quantum-box')
    vi.stubEnv('VITE_LIBRARY_REPO', 'docs')
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    vi.stubEnv('VITE_LIBRARY_PLATFORM_ID', 'platform-1')
    vi.stubEnv('VITE_LIBRARY_OPERATOR_ID', 'operator-1')
  })

  /**
   * The case Photon PR #71 exists for. The client cannot tell its first write
   * from an edit, so it sends an upsert; if that went to the update-only
   * `PUT .../data/{id}` the server would answer 404, `decisionForError` would
   * map it to `rejected`, and the record the user just made would vanish.
   */
  it('sends a first write to the upsert route rather than the update-only one', async () => {
    const calls = stubLibrary((url, init) => {
      if (url.endsWith('/data/local-1/upsert') && init.method === 'PUT') {
        return Response.json({
          id: 'local-1',
          name: 'Route the first write',
          items: [{ property_id: 'prop-status', value: 'todo' }],
        })
      }
      return undefined
    })

    const resource = createLibraryRecordsResource(target)
    const stored = await resource.upsert('local-1', record())

    expect(stored.id).toBe('local-1')
    expect(stored.title).toBe('Route the first write')
    const write = calls.find((call) => call.method === 'PUT')
    expect(write?.url).toBe(
      'https://library.example.test/v1beta/repos/quantum-box/docs/data/local-1/upsert'
    )
    expect(calls.some((call) => call.url.endsWith('/data/local-1'))).toBe(false)
  })

  it('still sends a later edit through the update-only route', async () => {
    const calls = stubLibrary((url, init) => {
      if (url.endsWith('/data/data-1') && init.method === 'PUT') {
        return Response.json({
          id: 'data-1',
          name: 'Edited',
          items: [{ property_id: 'prop-status', value: 'done' }],
        })
      }
      return undefined
    })

    const resource = createLibraryRecordsResource(target)
    const updated = await resource.update('data-1', { title: 'Edited', status: 'done' })

    expect(updated.id).toBe('data-1')
    expect(updated.title).toBe('Edited')
    const write = calls.find((call) => call.method === 'PUT')
    expect(write?.url).toBe(
      'https://library.example.test/v1beta/repos/quantum-box/docs/data/data-1'
    )
    expect(write?.body).toMatchObject({ name: 'Edited' })
  })

  /**
   * An upsert against a record the repository does not have must still fail.
   * The endpoint creates records, it does not conjure repositories, and Photon
   * needs the status to decide between a rejection and a retry.
   */
  it('carries the HTTP status so Photon can decide what the failure means', async () => {
    stubLibrary((url, init) => {
      if (url.endsWith('/data/local-1/upsert') && init.method === 'PUT') {
        return new Response('bad request', { status: 400 })
      }
      return undefined
    })

    const resource = createLibraryRecordsResource(target)
    await expect(resource.upsert('local-1', record())).rejects.toMatchObject({
      name: 'RecordApiError',
      status: 400,
    })
  })

  /**
   * `create` is the path where library-api mints its own id, and `toRecord` is
   * the only place Photon learns it — the returned recordId becomes the
   * `aliasRecordId`. Keying off the local id instead would make the next pull
   * insert the server's row as a second record.
   */
  it('maps a server-assigned id through toRecord so it can become the alias', async () => {
    stubLibrary((url, init) => {
      if (url.endsWith('/repos/quantum-box/docs/data') && init.method === 'POST') {
        return Response.json({
          id: 'data_01k9server',
          name: 'Route the first write',
          items: [{ property_id: 'prop-status', value: 'todo' }],
        })
      }
      return undefined
    })

    const resource = createLibraryRecordsResource(target)
    const created = await resource.create(record({ id: 'local-1' }))

    expect(created.id).toBe('data_01k9server')
    expect(resource.toRecord(created)).toEqual({
      recordId: 'data_01k9server',
      value: created,
    })
  })

  /**
   * The gap this fixture exists for: standard repositories type status and
   * priority as Select, so a REST-backed write that cannot encode one cannot
   * write anything at all. `{ optionId }` is tagged rather than a bare string
   * because the API reads the JSON without knowing the target Property, and a
   * bare string means `String` there.
   */
  it('encodes a Select as a tagged option id on create', async () => {
    const calls = stubLibrary((url, init) => {
      if (url.endsWith('/repos/quantum-box/docs/data') && init.method === 'POST') {
        return Response.json({
          id: 'data_01k9server',
          name: 'Route the first write',
          items: [
            { property_id: 'prop-status', value: { select: 'opt-todo' } },
            { property_id: 'prop-priority', value: { select: 'opt-none' } },
          ],
        })
      }
      return undefined
    }, selectProperties)

    const created = await createLibraryRecordsResource(target).create(record({ id: 'local-1' }))

    // Not just any POST — the GraphQL Property fetch is one too.
    const write = calls.find(
      (call) => call.method === 'POST' && call.url.endsWith('/repos/quantum-box/docs/data')
    )
    expect(write?.body).toMatchObject({
      property_data: [
        { property_id: 'prop-status', value: { optionId: 'opt-todo' } },
        { property_id: 'prop-priority', value: { optionId: 'opt-none' } },
      ],
    })
    // And the option ids come back as the record fields they stand for, so the
    // write survives a read.
    expect(created.status).toBe('todo')
    expect(created.priority).toBe('none')
  })

  it('encodes a Select the same way on upsert', async () => {
    const calls = stubLibrary((url, init) => {
      if (url.endsWith('/data/local-1/upsert') && init.method === 'PUT') {
        return Response.json({
          id: 'local-1',
          name: 'Route the first write',
          items: [{ property_id: 'prop-status', value: { select: 'opt-todo' } }],
        })
      }
      return undefined
    }, selectProperties)

    const stored = await createLibraryRecordsResource(target).upsert('local-1', record())

    const write = calls.find((call) => call.method === 'PUT')
    expect(write?.body).toMatchObject({
      property_data: [
        { property_id: 'prop-status', value: { optionId: 'opt-todo' } },
        { property_id: 'prop-priority', value: { optionId: 'opt-none' } },
      ],
    })
    expect(stored.status).toBe('todo')
  })

  it('treats a delete of an already-gone record as done', async () => {
    stubLibrary((url, init) => {
      if (url.endsWith('/data/data-1') && init.method === 'DELETE') {
        return new Response('gone', { status: 404 })
      }
      return undefined
    })

    const resource = createLibraryRecordsResource(target)
    await expect(resource.remove('data-1')).resolves.toBeUndefined()
  })
})
})

/**
 * Where a write goes, now that the collection says so.
 *
 * A record used to be routed by reading `orgUsername` off its own value, then
 * off a cached copy of that value, then off the build's environment. All three
 * are guesses at something the key already knows: a record lives in exactly
 * one collection, and a collection *is* a repository.
 */
describe('the destination of a write', () => {
  const photonCore = { databaseId: 'repo-1', org: 'quantum-box', repo: 'photon-core' }
  const libraryRepo = { databaseId: 'repo-2', org: 'quantum-box', repo: 'library' }

  /** The value lies about its repository, so only the key can be right. */
  const heldByLibraryRepo = {
    scope: 'test',
    collection: 'data:repo-2',
    recordId: 'data-9',
    deleted: false,
    updatedAt: '0',
    value: {
      id: 'data-9',
      title: 'Held by the library repo',
      orgUsername: 'stale-org',
      repoUsername: 'stale-repo',
    },
  }

  function signIn() {
    localStorage.setItem('library_auth', JSON.stringify({
      accessToken: 'token',
      refreshToken: '',
      expiresAt: Math.floor(Date.now() / 1000 + 3600),
      userId: 'user-1',
      email: 'test@example.com',
      username: 'test',
    }))
  }

  beforeEach(() => {
    vi.stubEnv('VITE_LIBRARY_API_BASE_URL', 'https://library.example.test')
    // Deliberately no VITE_LIBRARY_ORG / VITE_LIBRARY_REPO: if the collection
    // does not decide, nothing else can.
    signIn()
    rememberLibraryRepositories([photonCore, libraryRepo])
    vi.mocked(photonEngine.listClientEngineRecords).mockImplementation(
      async (collection: string) =>
        collection === 'data:repo-2' ? [heldByLibraryRepo] : []
    )
  })

  afterEach(() => {
    libraryCollections.reset()
    vi.mocked(photonEngine.listClientEngineRecords).mockReset().mockResolvedValue([])
  })

  it('updates against the repository whose collection holds the record', async () => {
    const fetchMock = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as { query: string }
      if (body.query.includes('LibraryClientDataDetail')) {
        return Response.json({
          data: {
            data: { id: 'data-9', name: 'Held by the library repo', propertyData: [] },
            properties: [{ id: 'status', name: 'Status', typ: 'Select', meta: { options: [
              { id: 'status-done', key: 'done', name: 'Done' },
            ] } }],
          },
        })
      }
      return Response.json({
        data: { updateData: { id: 'data-9', name: 'Held by the library repo', propertyData: [] } },
      })
    })
    vi.stubGlobal('fetch', fetchMock)

    await updateServerRecord('data-9', { status: 'done' })

    // The collection decides, and the collection alone: the value claims
    // `stale-org` / `stale-repo`, and neither reaches the write.
    expect(photonEngine.patchAndPushClientEngineRecord).toHaveBeenCalledWith(
      'data:repo-2',
      'data-9',
      expect.objectContaining({ status: 'done' })
    )
  })

  it('deletes against that same repository', async () => {
    const sent: string[] = []
    const fetchMock = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      sent.push(String(init?.body))
      return Response.json({ data: { deleteData: 'data-9' } })
    })
    vi.stubGlobal('fetch', fetchMock)

    await deleteServerRecord('data-9')

    // A `remove` operation rather than an API call plus a tombstone: the
    // operation *is* the DELETE now, so it queues offline like any other
    // write and comes back if the server refuses it.
    expect(photonEngine.deleteAndPushClientEngineRecord).toHaveBeenCalledWith(
      'data:repo-2',
      'data-9'
    )
    expect(sent).toEqual([])
  })

  /**
   * A conflict is not a save.
   *
   * Photon has already put the record back to the server's value and kept the
   * user's on a conflict row, so returning it would clear `RecordsContext`'s
   * mutation error and show a successful save of a change that is not there.
   */
  it('reports a conflicting edit as an error rather than a save', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => Response.json({ data: { properties: [] } })))
    vi.mocked(photonEngine.patchAndPushClientEngineRecord).mockResolvedValueOnce({
      status: 'conflict',
      record: null,
      conflictId: 'c1',
      reason: 'edited elsewhere',
    })

    await expect(updateServerRecord('data-9', { title: 'mine' })).rejects.toMatchObject({
      status: 409,
    })
  })

  it('does nothing over the API for a record no collection holds', async () => {
    const fetchMock = vi.fn(async () => Response.json({ data: {} }))
    vi.stubGlobal('fetch', fetchMock)

    await deleteServerRecord('never-fetched')

    // The old chain would have fallen through to VITE_LIBRARY_ORG here and
    // deleted an id in whichever repository the build happened to name.
    expect(fetchMock).not.toHaveBeenCalled()
    expect(photonEngine.deleteClientEngineRecord).toHaveBeenCalledWith('records', 'never-fetched')
  })
})
