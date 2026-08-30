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
