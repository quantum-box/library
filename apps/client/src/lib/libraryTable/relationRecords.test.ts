import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createLibraryRelationRecordLoader } from './relationRecords'

const mocks = vi.hoisted(() => ({
  fetchLibraryRepositories: vi.fn(),
  fetchLibraryRepoTableData: vi.fn(),
  fetchLibraryDataDetail: vi.fn(),
}))

vi.mock('../recordsApi', () => mocks)

describe('createLibraryRelationRecordLoader', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.fetchLibraryRepositories.mockResolvedValue([
      {
        id: 'database-target',
        username: 'people',
        name: 'People',
        orgUsername: 'example',
        operatorId: 'operator-example',
      },
    ])
    mocks.fetchLibraryRepoTableData.mockResolvedValue({
      items: [{ id: 'data-1', name: 'Aoi Example', propertyData: [] }],
      properties: [],
      repoName: 'People',
      hasMore: true,
      nextPage: 2,
    })
  })

  it('resolves the canonical database id and keeps pagination', async () => {
    const loader = createLibraryRelationRecordLoader()
    await expect(loader.loadPage('database-target')).resolves.toEqual({
      items: [{ id: 'data-1', name: 'Aoi Example' }],
      hasMore: true,
      nextPage: 2,
      repositoryLabel: 'example / People',
    })
    expect(mocks.fetchLibraryRepoTableData).toHaveBeenCalledWith({
      org: 'example',
      repo: 'people',
      operatorId: 'operator-example',
      databaseId: 'database-target',
      repoName: 'People',
    }, 1)
  })

  it('loads selected labels directly and preserves an unavailable id', async () => {
    mocks.fetchLibraryDataDetail
      .mockResolvedValueOnce({ item: { id: 'data-2', name: 'Haru Example' }, properties: [] })
      .mockRejectedValueOnce(Object.assign(new Error('not found'), { status: 404 }))
    const loader = createLibraryRelationRecordLoader()

    await expect(loader.loadSelected('database-target', ['data-2', 'removed', 'data-2']))
      .resolves.toEqual([
        { id: 'data-2', name: 'Haru Example' },
        { id: 'removed', name: 'removed', unavailable: true },
      ])
  })

  it('rejects a target repository the caller cannot see', async () => {
    const loader = createLibraryRelationRecordLoader()
    await expect(loader.loadPage('other-database'))
      .rejects.toThrow('Relation target unavailable')
  })

  it('deduplicates concurrent detail requests for the same selected record', async () => {
    mocks.fetchLibraryDataDetail.mockResolvedValue({
      item: { id: 'data-2', name: 'Haru Example' },
      properties: [],
    })
    const loader = createLibraryRelationRecordLoader()

    await Promise.all([
      loader.loadSelected('database-target', ['data-2']),
      loader.loadSelected('database-target', ['data-2']),
    ])

    expect(mocks.fetchLibraryDataDetail).toHaveBeenCalledTimes(1)
  })

  it('propagates transient detail failures and permits a retry', async () => {
    mocks.fetchLibraryDataDetail
      .mockRejectedValueOnce(Object.assign(new Error('temporary failure'), { status: 503 }))
      .mockResolvedValueOnce({
        item: { id: 'data-2', name: 'Haru Example' },
        properties: [],
      })
    const loader = createLibraryRelationRecordLoader()

    await expect(loader.loadSelected('database-target', ['data-2']))
      .rejects.toThrow('temporary failure')
    await expect(loader.loadSelected('database-target', ['data-2']))
      .resolves.toEqual([{ id: 'data-2', name: 'Haru Example' }])
    expect(mocks.fetchLibraryDataDetail).toHaveBeenCalledTimes(2)
  })

  it('retries repository discovery after a failed request', async () => {
    mocks.fetchLibraryRepositories
      .mockRejectedValueOnce(new Error('temporary failure'))
      .mockResolvedValueOnce([
        {
          id: 'database-target',
          username: 'people',
          name: 'People',
          orgUsername: 'example',
        },
      ])
    const loader = createLibraryRelationRecordLoader()

    await expect(loader.loadPage('database-target')).rejects.toThrow('temporary failure')
    await expect(loader.loadPage('database-target')).resolves.toMatchObject({
      repositoryLabel: 'example / People',
    })
    expect(mocks.fetchLibraryRepositories).toHaveBeenCalledTimes(2)
  })
})
