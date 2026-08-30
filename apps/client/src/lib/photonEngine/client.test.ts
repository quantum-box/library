/**
 * The local store's contract, against the real engine.
 *
 * A real WASM kernel and a real (in-memory) PGlite store, so what is exercised
 * is what ships. Only the data directory and the transport are swapped: one
 * would open IndexedDB, the other would talk to a server.
 */
import { readFile } from 'node:fs/promises'
import path from 'node:path'

import { createPGliteStore } from '@quantum-box/photon/store-pglite'
import { loadPhotonKernel, setPhotonKernelSource } from '@quantum-box/photon/wasm'
import type { SyncTransport } from '@quantum-box/photon'
import { beforeAll, beforeEach, afterEach, describe, expect, it } from 'vitest'

import {
  __testOnly as libraryCollections,
  libraryRecordsCollection,
  rememberLibraryRepositories,
  setLibraryRecordsResourceFactory,
} from './libraryCollections'
import {
  __testOnly,
  deleteClientEngineRecord,
  getClientEngineRecord,
  ingestClientEngineRecords,
  listClientEngineRecords,
  patchClientEngineRecord,
  syncClientEngineOperations,
  upsertClientEngineRecord,
} from './client'

interface Doc {
  title: string
  status?: string
}

/** Operations the transport was asked to push, in order. */
let pushed: { collection: string; recordId: string }[] = []

function recordingTransport(): SyncTransport {
  return {
    async push(request) {
      for (const operation of request.operations) {
        pushed.push({ collection: operation.key.collection, recordId: operation.key.record_id })
      }
      return { decisions: request.operations.map((o) => ({ kind: 'accepted' as const, operationId: o.id })) }
    },
    async pull(request) {
      return { kind: 'operations', operations: [], cursor: request.cursor }
    },
  }
}

beforeAll(async () => {
  // Vitest serves `import.meta.url` over http, so the loader cannot find the
  // asset on its own — hand it the bytes, as the package documents.
  // `exports` hides package.json, so resolve from the workspace root instead.
  const wasm = path.join(
    process.cwd(),
    'node_modules/@quantum-box/photon/crates/photon-engine/pkg/photon_engine_bg.wasm'
  )
  setPhotonKernelSource(await readFile(wasm))
})

beforeEach(async () => {
  pushed = []
  await __testOnly.reset()
  __testOnly.configure({
    storage: await createPGliteStore(),
    kernel: await loadPhotonKernel(),
    transport: recordingTransport(),
    skipLegacyMigration: true,
  })
})

afterEach(async () => {
  await __testOnly.reset()
  libraryCollections.reset()
})

describe('records', () => {
  it('round-trips an upsert through get and list', async () => {
    await upsertClientEngineRecord<Doc>('documents', 'd1', { title: 'first' })

    const one = await getClientEngineRecord<Doc>('documents', 'd1')
    expect(one?.value.title).toBe('first')
    expect(one?.recordId).toBe('d1')
    expect(one?.deleted).toBe(false)

    const all = await listClientEngineRecords<Doc>('documents')
    expect(all.map((record) => record.recordId)).toEqual(['d1'])
  })

  it('merges a patch into the existing value', async () => {
    await upsertClientEngineRecord<Doc>('documents', 'd1', { title: 'first' })
    const patched = await patchClientEngineRecord<Doc>('documents', 'd1', { status: 'done' })

    expect(patched?.value).toEqual({ title: 'first', status: 'done' })
  })

  it('returns null when patching a record that is not there', async () => {
    expect(await patchClientEngineRecord<Doc>('documents', 'missing', { title: 'x' })).toBeNull()
  })

  it('hides a deleted record from get and list', async () => {
    await upsertClientEngineRecord<Doc>('documents', 'd1', { title: 'first' })
    await deleteClientEngineRecord('documents', 'd1')

    expect(await getClientEngineRecord<Doc>('documents', 'd1')).toBeNull()
    expect(await listClientEngineRecords<Doc>('documents')).toEqual([])
  })

  it('keeps collections apart', async () => {
    await upsertClientEngineRecord<Doc>('documents', 'd1', { title: 'doc' })
    await upsertClientEngineRecord<Doc>('attachments', 'a1', { title: 'file' })

    expect((await listClientEngineRecords<Doc>('documents')).map((r) => r.recordId)).toEqual(['d1'])
    expect((await listClientEngineRecords<Doc>('attachments')).map((r) => r.recordId)).toEqual(['a1'])
  })
})

describe('ingest', () => {
  it('stores a record without queueing it for push', async () => {
    await ingestClientEngineRecords('library_data_records', [
      { recordId: 'r1', value: { title: 'from the Library API' } },
      { recordId: 'r2', value: { title: 'also from the API' } },
    ])

    // Readable like any other record...
    const cached = await listClientEngineRecords<Doc>('library_data_records')
    expect(cached.map((record) => record.recordId).sort()).toEqual(['r1', 'r2'])

    // ...but the Library API already owns them, so syncing must not push them
    // anywhere. Writing them as operations is what made a `documents` save
    // try to push the whole record cache at the Engine: pending is scope-wide.
    const summary = await syncClientEngineOperations()
    expect(summary.pushed).toBe(0)
    expect(pushed).toEqual([])
  })

  it('still pushes a genuine local write alongside ingested rows', async () => {
    await ingestClientEngineRecords('library_data_records', [
      { recordId: 'r1', value: { title: 'cached' } },
    ])
    await upsertClientEngineRecord<Doc>('documents', 'd1', { title: 'mine' })

    await syncClientEngineOperations()

    expect(pushed).toEqual([{ collection: 'documents', recordId: 'd1' }])
  })
})

/**
 * The wiring that makes a repository a collection.
 *
 * `createPhotonClient` is built with a fixed set of collections, and this app's
 * set is discovered from the Library API at runtime. `resolveCollection` is the
 * seam that closes the gap, and what it answers decides where a write goes —
 * the engine transport, or the repository's own REST resource.
 */
describe('repository collections', () => {
  const photonCore = {
    databaseId: 'repo-1',
    org: 'quantum-box',
    repo: 'photon-core',
  }

  /** Records the resource was asked to write, so routing is observable. */
  let restWrites: string[] = []

  function stubRepositoryResource() {
    setLibraryRecordsResourceFactory((repository) => ({
      list: async () => ({ items: [], complete: true }),
      create: async (value: { id: string }) => {
        restWrites.push(`create ${repository.databaseId}/${value.id}`)
        return value
      },
      upsert: async (recordId: string, value: unknown) => {
        restWrites.push(`upsert ${repository.databaseId}/${recordId}`)
        return value
      },
      update: async (recordId: string) => {
        restWrites.push(`update ${repository.databaseId}/${recordId}`)
        return { id: recordId }
      },
      remove: async (recordId: string) => {
        restWrites.push(`remove ${repository.databaseId}/${recordId}`)
      },
      toRecord: (item: { id: string }) => ({ recordId: item.id, value: item }),
    // The erasure Photon's own `CollectionConfig` uses for resources whose
    // value type it cannot name.
    }) as never)
  }

  beforeEach(() => {
    restWrites = []
  })

  it('pushes a repository\'s records to its resource, not to the engine', async () => {
    stubRepositoryResource()
    rememberLibraryRepositories([photonCore])

    const collection = libraryRecordsCollection(photonCore.databaseId)
    await upsertClientEngineRecord(collection, 'data-1', { id: 'data-1', title: 'mine' })
    await syncClientEngineOperations()

    expect(restWrites).toEqual(['upsert repo-1/data-1'])
    // The Library API owns these rows; the engine must never see them.
    expect(pushed).toEqual([])
  })

  it('keeps engine-native collections on the engine transport', async () => {
    stubRepositoryResource()
    rememberLibraryRepositories([photonCore])

    await upsertClientEngineRecord('documents', 'd1', { title: 'mine' })
    await syncClientEngineOperations()

    expect(pushed).toEqual([{ collection: 'documents', recordId: 'd1' }])
    expect(restWrites).toEqual([])
  })

  it('ingests a tombstone without asking the API to delete it again', async () => {
    stubRepositoryResource()
    rememberLibraryRepositories([photonCore])
    const collection = libraryRecordsCollection(photonCore.databaseId)

    await ingestClientEngineRecords(collection, [
      { recordId: 'data-1', value: { id: 'data-1' } },
    ])
    await ingestClientEngineRecords(collection, [
      { recordId: 'data-1', value: { id: 'data-1' }, deleted: true },
    ])

    expect(await listClientEngineRecords(collection)).toEqual([])
    await syncClientEngineOperations()
    expect(restWrites).toEqual([])
  })
})
