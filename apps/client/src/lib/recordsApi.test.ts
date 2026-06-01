import { afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('./photonEngine/client', () => ({
  deleteClientEngineRecord: vi.fn(),
  listClientEngineRecords: vi.fn(async () => []),
  patchClientEngineRecord: vi.fn(),
  upsertClientEngineRecord: vi.fn(),
}))
import { appKitConfig } from '../app/kitConfig'
import {
  fetchLibraryOrganizations,
  fetchLibraryRecords,
  fetchLibraryRepoTableData,
  fetchLibraryRepositories,
  libraryDataToRecord,
  toRecord,
  type LibraryDataItem,
  type LibraryProperty,
  type ServerRecord,
} from './recordsApi'

describe('recordsApi', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.unstubAllGlobals()
    localStorage.clear()
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
  })
})
