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
import { beforeAll, beforeEach, afterEach, describe, expect, it, vi } from 'vitest'

import {
  __testOnly as libraryCollections,
  libraryRecordsCollection,
  rememberLibraryRepositories,
  setLibraryRecordsResourceFactory,
} from './libraryCollections'
import {
  __testOnly,
  deleteAndPushClientEngineRecord,
  deleteClientEngineRecord,
  getClientEngineRecord,
  listClientEngineConflicts,
  patchAndPushClientEngineRecord,
  upsertAndPushClientEngineRecord,
  ingestClientEngineRecords,
  listClientEngineRecords,
  patchClientEngineRecord,
  syncClientEngineOperations,
  upsertClientEngineRecord,
} from './client'

/**
 * Generous, because every test here builds a real WASM kernel and a real
 * PGlite database in `beforeEach` and tears them down after. The defaults are
 * sized for pure-JS unit tests and this file times out under them whenever the
 * machine is doing anything else.
 */
vi.setConfig({ testTimeout: 20_000, hookTimeout: 30_000 })

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

/**
 * What a `rest-backed` collection buys beyond routing.
 *
 * The suite above proves a write reaches the repository's resource. This one
 * is about what happens to it afterwards: queued when there is no answer,
 * rolled back when the answer is no, held on a conflict row when the answer is
 * "someone else got there first". That is the whole reason records went
 * through the operation log rather than straight at library-api.
 */
describe('rest-backed write outcomes', () => {
  const photonCore = {
    databaseId: 'repo-1',
    org: 'quantum-box',
    repo: 'photon-core',
  }
  const collection = libraryRecordsCollection(photonCore.databaseId)

  /**
   * The stub's server. Real state, not a fixed list: `list()` claims
   * `complete`, and a complete snapshot that omits a record the push just
   * accepted tombstones it locally. That is the engine behaving correctly, and
   * a stub that always listed nothing would delete every record one cycle
   * after creating it.
   */
  let server: Map<string, Doc & { id: string }>
  /**
   * How the stub answers next. Mutable rather than re-registered, because
   * `resolveCollection` is consulted once per collection and its answer is
   * kept — handing over a second factory mid-test changes nothing.
   */
  let failures: Record<string, Error>
  let offline: boolean

  /** An HTTP-ish failure: `status` is the one field Photon's mapping reads. */
  class Status extends Error {
    readonly status: number

    constructor(status: number, message = 'nope') {
      super(message)
      this.status = status
    }
  }

  beforeEach(() => {
    server = new Map()
    failures = {}
    offline = false
    const reject = () => {
      if (offline) throw new TypeError('Failed to fetch')
    }
    setLibraryRecordsResourceFactory(() => ({
      list: async () => {
        reject()
        return { items: [...server.values()], complete: true }
      },
      create: async (value: Doc) => {
        reject()
        const stored = { ...value, id: 'server-assigned' }
        server.set(stored.id, stored)
        return stored
      },
      upsert: async (recordId: string, value: Doc) => {
        reject()
        if (failures[recordId]) throw failures[recordId]
        const stored = { ...value, id: recordId }
        server.set(recordId, stored)
        return stored
      },
      update: async (recordId: string, fields: Partial<Doc>) => {
        reject()
        if (failures[recordId]) throw failures[recordId]
        const stored = { ...server.get(recordId), ...fields, id: recordId } as Doc & { id: string }
        server.set(recordId, stored)
        return stored
      },
      remove: async (recordId: string) => {
        reject()
        if (failures[recordId]) throw failures[recordId]
        server.delete(recordId)
      },
      toRecord: (item: Doc & { id: string }) => ({ recordId: item.id, value: item }),
    }) as never)
    rememberLibraryRepositories([photonCore])
  })

  it('reports a write the server accepted', async () => {
    const outcome = await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', {
      title: 'first',
    })

    expect(outcome.status).toBe('accepted')
    expect(outcome.record?.value.title).toBe('first')
    expect(pushed).toEqual([])
  })

  it('keeps an offline write, and sends it once the resource answers', async () => {
    offline = true

    const outcome = await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', {
      title: 'written on a plane',
    })

    // Queued, not failed. The record is on screen and the operation is durable.
    expect(outcome.status).toBe('queued')
    expect(outcome.record?.value.title).toBe('written on a plane')
    expect((await getClientEngineRecord<Doc>(collection, 'r1'))?.value.title).toBe(
      'written on a plane'
    )

    // Back online: the same operation goes out, with no help from the caller.
    offline = false
    await syncClientEngineOperations()
    expect(server.get('r1')?.title).toBe('written on a plane')
  })

  it('rolls a rejected create back off the screen', async () => {
    failures = { r1: new Status(400, 'name is required') }

    const outcome = await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', { title: '' })

    expect(outcome.status).toBe('rejected')
    expect(outcome.reason).toContain('name is required')
    // The record was never on the server, so rolling back removes it entirely
    // rather than restoring a previous value.
    expect(outcome.record).toBeNull()
    expect(await getClientEngineRecord<Doc>(collection, 'r1')).toBeNull()
  })

  it('restores the previous value when an edit is rejected', async () => {
    await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', { title: 'saved' })

    failures = { r1: new Status(400) }
    const outcome = await patchAndPushClientEngineRecord<Doc>(collection, 'r1', {
      title: 'not allowed',
    })

    expect(outcome.status).toBe('rejected')
    expect(outcome.record?.value.title).toBe('saved')
  })

  it('raises a conflict row rather than losing the edit', async () => {
    await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', { title: 'mine' })

    failures = { r1: new Status(409, 'edited elsewhere') }
    const outcome = await patchAndPushClientEngineRecord<Doc>(collection, 'r1', {
      title: 'mine, edited',
    })

    expect(outcome.status).toBe('conflict')

    // The edit is not lost, but it is not left on screen either: the conflict
    // row holds it as `localValue`, and the projection goes back to what the
    // server has. That split is the point — the work survives somewhere it can
    // be resolved from, rather than silently winning a race it lost.
    const conflicts = await listClientEngineConflicts(collection)
    expect(conflicts).toHaveLength(1)
    expect(conflicts[0]?.key.record_id).toBe('r1')
    expect((conflicts[0]?.localValue as Doc).title).toBe('mine, edited')
    expect(conflicts[0]?.reason).toContain('edited elsewhere')
    expect(outcome.record?.value.title).toBe('mine')
  })

  it('does not roll back a write the server never answered', async () => {
    await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', { title: 'saved' })

    // 503 is not a verdict. Treating it as one would delete a record the user
    // will be able to save again in a moment.
    failures = { r1: new Status(503) }
    const outcome = await patchAndPushClientEngineRecord<Doc>(collection, 'r1', {
      title: 'still mine',
    })

    expect(outcome.status).toBe('queued')
    expect(outcome.record?.value.title).toBe('still mine')
  })

  it('deletes through the resource', async () => {
    await upsertAndPushClientEngineRecord<Doc>(collection, 'r1', { title: 'doomed' })

    const outcome = await deleteAndPushClientEngineRecord(collection, 'r1')

    expect(outcome.status).toBe('accepted')
    expect(server.has('r1')).toBe(false)
    expect(await getClientEngineRecord<Doc>(collection, 'r1')).toBeNull()
  })
})
