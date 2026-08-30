/**
 * Renaming a collection orphans whatever a browser already holds under the old
 * name. The Library API can hand it all back, so the carry-over is a comfort
 * rather than a rescue — but a first load with no network is exactly when the
 * comfort is worth having, and that is the case these cover.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest'

interface Cached {
  recordId: string
  value: { id: string; orgUsername?: string; repoUsername?: string }
}

const engine = vi.hoisted(() => ({
  ingestClientEngineRecords: vi.fn<
    (collection: string, items: readonly { recordId: string }[]) => Promise<void>
  >(async () => undefined),
  listClientEngineRecords: vi.fn<(collection: string) => Promise<unknown[]>>(async () => []),
}))

vi.mock('./client', () => engine)

import { carryLegacyLibraryRecords, legacyRecordsCollectionPending } from './recordsCollectionMigration'

const repositories = [
  { databaseId: 'repo-1', org: 'quantum-box', repo: 'photon-core' },
  { databaseId: 'repo-2', org: 'quantum-box', repo: 'library' },
]

function cached(recordId: string, org?: string, repo?: string): Cached {
  return { recordId, value: { id: recordId, orgUsername: org, repoUsername: repo } }
}

beforeEach(() => {
  localStorage.clear()
  engine.ingestClientEngineRecords.mockClear()
  engine.listClientEngineRecords.mockReset().mockResolvedValue([])
})

describe('carryLegacyLibraryRecords', () => {
  it('sends each cached record to the collection of the repository that owns it', async () => {
    engine.listClientEngineRecords.mockResolvedValue([
      cached('data-1', 'quantum-box', 'photon-core'),
      cached('data-2', 'quantum-box', 'library'),
      cached('data-3', 'quantum-box', 'photon-core'),
    ])

    await carryLegacyLibraryRecords(repositories)

    expect(engine.listClientEngineRecords).toHaveBeenCalledWith('library_data_records')
    const written = engine.ingestClientEngineRecords.mock.calls.map(([collection, items]) => [
      collection,
      items.map((item) => item.recordId),
    ])
    expect(written).toEqual([
      ['data:repo-1', ['data-1', 'data-3']],
      ['data:repo-2', ['data-2']],
    ])
  })

  it('leaves a record it cannot route where it is, and lets the API return it', async () => {
    engine.listClientEngineRecords.mockResolvedValue([
      cached('data-1', 'quantum-box', 'photon-core'),
      cached('orphan', 'someone-else', 'gone'),
      cached('unlabelled'),
    ])

    await carryLegacyLibraryRecords(repositories)

    const carried = engine.ingestClientEngineRecords.mock.calls.flatMap(([, items]) =>
      items.map((item) => item.recordId)
    )
    expect(carried).toEqual(['data-1'])
  })

  it('runs once, and says so afterwards', async () => {
    engine.listClientEngineRecords.mockResolvedValue([
      cached('data-1', 'quantum-box', 'photon-core'),
    ])
    expect(legacyRecordsCollectionPending()).toBe(true)

    await carryLegacyLibraryRecords(repositories)
    expect(legacyRecordsCollectionPending()).toBe(false)

    engine.ingestClientEngineRecords.mockClear()
    await carryLegacyLibraryRecords(repositories)
    expect(engine.ingestClientEngineRecords).not.toHaveBeenCalled()
  })

  it('waits for a repository to route to rather than declaring itself done', async () => {
    await carryLegacyLibraryRecords([])

    expect(engine.listClientEngineRecords).not.toHaveBeenCalled()
    // Nothing could have been carried, so the old collection is still the only
    // place those rows live and the offline fallback must keep reading it.
    expect(legacyRecordsCollectionPending()).toBe(true)
  })

  it('gives up quietly when the old collection will not open', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    engine.listClientEngineRecords.mockRejectedValue(new Error('store is locked'))

    await expect(carryLegacyLibraryRecords(repositories)).resolves.toBeUndefined()

    // Not marked done: the rows are still only under the old name, and the next
    // load should try again.
    expect(legacyRecordsCollectionPending()).toBe(true)
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })
})
