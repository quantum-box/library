/**
 * The read cache, against the real engine: what a screen remembers is what
 * the next visit to it can draw, synchronously once the store is open.
 */
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'

import { createPGliteStore } from '@quantum-box/photon/store-pglite'
import { loadPhotonKernel, setPhotonKernelSource } from '@quantum-box/photon/wasm'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { __testOnly as engine, listClientEngineRecords } from './photonEngine/client'
import { LIBRARY_READ_DETAILS_COLLECTION } from './photonEngine/libraryCollections'
import type { LibraryDataItem, LibraryProperty } from './recordsApi'
import {
  __testOnly as readCache,
  forgetData,
  peekDataDetail,
  peekRepoTable,
  readDataDetail,
  readRepoTable,
  readWorkspace,
  rememberDataDetail,
  rememberRepoTable,
  rememberWorkspace,
} from './libraryReadCache'

const auth = vi.hoisted(() => ({ userId: 'user-a' as string | null }))
vi.mock('./auth', () => ({
  loadStoredAuthIdentity: () => (auth.userId ? { userId: auth.userId } : null),
}))

vi.setConfig({ testTimeout: 20_000, hookTimeout: 30_000 })

const target = { org: 'acme', repo: 'docs' }

const properties: LibraryProperty[] = [
  { id: 'prop-status', name: 'Status', typ: 'String' },
  { id: 'prop-body', name: 'Body', typ: 'Markdown' },
]

function row(id: string, status: string, body: string): LibraryDataItem {
  return {
    id,
    name: `Item ${id}`,
    propertyData: [
      { propertyId: 'prop-status', value: { string: status } },
      { propertyId: 'prop-body', value: { markdown: body } },
    ],
  }
}

/** Let the fire-and-forget ingest land. */
async function settle(): Promise<void> {
  await readRepoTable(target)
}

beforeAll(async () => {
  const wasm = path.join(
    process.cwd(),
    'node_modules/@quantum-box/photon/crates/photon-engine/pkg/photon_engine_bg.wasm'
  )
  setPhotonKernelSource(await readFile(wasm))
})

beforeEach(async () => {
  auth.userId = 'user-a'
  await engine.reset()
  engine.configure({
    storage: await createPGliteStore(),
    kernel: await loadPhotonKernel(),
    skipLegacyMigration: true,
  })
})

afterEach(async () => {
  await engine.reset()
})

describe('repository tables', () => {
  it('answers a peek only once the store is open', async () => {
    expect(peekRepoTable(target)).toBeNull()

    rememberRepoTable(target, {
      items: [row('d1', 'todo', 'preview')],
      properties,
      nextPage: null,
      totalItems: 1,
    })
    await settle()

    expect(peekRepoTable(target)?.items.map((item) => item.id)).toEqual(['d1'])
    expect((await readRepoTable(target))?.totalItems).toBe(1)
  })

  it('keeps one account from opening onto another account’s rows', async () => {
    rememberRepoTable(target, { items: [row('d1', 'todo', '')], properties, nextPage: null, totalItems: 1 })
    await settle()

    auth.userId = 'user-b'
    expect(peekRepoTable(target)).toBeNull()
    expect(await readRepoTable(target)).toBeNull()
  })
})

describe('record pages', () => {
  it('falls back to the table row, marked as not the whole record', async () => {
    rememberRepoTable(target, { items: [row('d1', 'todo', 'pre')], properties, nextPage: null, totalItems: 1 })
    await settle()

    const detail = peekDataDetail(target, 'd1')
    expect(detail?.item.id).toBe('d1')
    expect(detail?.complete).toBe(false)
    expect((await readDataDetail(target, 'd1'))?.complete).toBe(false)
    expect(peekDataDetail(target, 'missing')).toBeNull()
  })

  it('prefers the record itself, under the id it was asked for too', async () => {
    rememberRepoTable(target, { items: [row('d1', 'todo', 'pre')], properties, nextPage: null, totalItems: 1 })
    rememberDataDetail(target, 'DOC-1', { item: row('d1', 'todo', 'the whole body'), properties })
    await settle()

    for (const id of ['d1', 'DOC-1']) {
      const detail = peekDataDetail(target, id)
      expect(detail?.complete).toBe(true)
      expect(detail?.item.propertyData[1]?.value).toEqual({ markdown: 'the whole body' })
    }
  })

  /**
   * Going back to the table after an edit must not draw the row as it was
   * before the edit -- but must not swap its body preview for the body either.
   */
  it('brings the cached row up to date, keeping the row’s own body', async () => {
    rememberRepoTable(target, { items: [row('d1', 'todo', 'pre')], properties, nextPage: null, totalItems: 1 })
    await settle()

    rememberDataDetail(target, 'd1', { item: row('d1', 'done', 'the whole body'), properties })
    await settle()

    expect(peekRepoTable(target)?.items[0]?.propertyData).toEqual([
      { propertyId: 'prop-status', value: { string: 'done' } },
      { propertyId: 'prop-body', value: { markdown: 'pre' } },
    ])
  })

  it('forgets a deleted record and its row', async () => {
    rememberRepoTable(target, {
      items: [row('d1', 'todo', ''), row('d2', 'todo', '')],
      properties,
      nextPage: null,
      totalItems: 2,
    })
    rememberDataDetail(target, 'd1', { item: row('d1', 'todo', 'body'), properties })
    await settle()

    forgetData(target, 'd1')
    await vi.waitFor(() => {
      expect(peekDataDetail(target, 'd1')).toBeNull()
    })
    await settle()
    expect(peekRepoTable(target)?.items.map((item) => item.id)).toEqual(['d2'])
    expect(peekRepoTable(target)?.totalItems).toBe(1)
  })
})

describe('forgetting a record opened by its identifier', () => {
  it('forgets it under every name, whichever one the deletion used', async () => {
    rememberRepoTable(target, { items: [row('d1', 'todo', ''), row('d2', 'todo', '')], properties, nextPage: null, totalItems: 2 })
    rememberDataDetail(target, 'DOC-1', { item: row('d1', 'todo', 'body'), properties })
    await settle()

    forgetData(target, 'DOC-1')
    await vi.waitFor(async () => {
      expect(await readDataDetail(target, 'd1')).toBeNull()
    })
    expect(await readDataDetail(target, 'DOC-1')).toBeNull()
    expect((await readRepoTable(target))?.items.map((item) => item.id)).toEqual(['d2'])
  })
})

describe('remembering a record under more than one name', () => {
  it('keeps the identifier it was opened by when it is opened again by id', async () => {
    rememberDataDetail(target, 'DOC-1', { item: row('d1', 'todo', 'body'), properties })
    await settle()
    await readDataDetail(target, 'd1')
    rememberDataDetail(target, 'd1', { item: row('d1', 'done', 'body'), properties })
    await settle()

    await forgetData(target, 'd1')
    expect(await readDataDetail(target, 'DOC-1')).toBeNull()
    expect(await readDataDetail(target, 'd1')).toBeNull()
  })
})

describe('trimming a record remembered under two names', () => {
  afterEach(() => {
    readCache.setAutoTrim(true)
  })

  it('keeps or drops its names together', async () => {
    // Trimmed by hand below, at the one moment that splits the record's names
    // under an entry-by-entry trim: its two entries straddle the cut.
    readCache.setAutoTrim(false)
    const pause = () => new Promise((resolve) => setTimeout(resolve, 5))
    for (let index = 0; index < 60; index += 1) {
      rememberDataDetail(target, `old${index}`, { item: row(`old${index}`, 'todo', ''), properties })
    }
    await settle()
    await pause()
    rememberDataDetail(target, 'DOC-1', { item: row('d1', 'todo', 'body'), properties })
    await settle()
    await pause()
    for (let index = 0; index < readCache.DETAILS_KEPT - 1; index += 1) {
      rememberDataDetail(target, `new${index}`, { item: row(`new${index}`, 'todo', ''), properties })
    }
    await settle()

    await readCache.trimDetails()

    const kept = new Set((await listClientEngineRecords(LIBRARY_READ_DETAILS_COLLECTION)).map((page) => page.recordId))
    const names = [...kept].filter((key) => key.endsWith(':DOC-1') || key.endsWith(':d1'))
    expect(names).toHaveLength(2)
    // The most recent kept, the oldest gone.
    expect([...kept].filter((key) => /:new\d+$/.test(key))).toHaveLength(readCache.DETAILS_KEPT - 1)
    expect([...kept].some((key) => /:old\d+$/.test(key))).toBe(false)
  })
})

describe('workspace lists', () => {
  it('round-trips per account', async () => {
    rememberWorkspace({
      repositories: [{ id: 'r1', username: 'docs', name: 'Docs', orgUsername: 'acme' }],
      organizations: [],
    })
    await settle()

    expect((await readWorkspace())?.repositories.map((repository) => repository.id)).toEqual(['r1'])
    auth.userId = 'user-b'
    expect(await readWorkspace()).toBeNull()
  })
})

/**
 * The reason the cache is in Photon rather than in memory: the first screen
 * after an app restart can draw what the last session saw.
 */
describe('across a restart', () => {
  it('draws what was remembered before the store was closed', async () => {
    const dataDir = await mkdtemp(path.join(tmpdir(), 'library-read-cache-'))
    const open = async () => {
      await engine.reset()
      engine.configure({
        storage: await createPGliteStore({ dataDir }),
        kernel: await loadPhotonKernel(),
        skipLegacyMigration: true,
      })
    }
    try {
      await open()
      rememberRepoTable(target, { items: [row('d1', 'todo', 'pre')], properties, nextPage: null, totalItems: 1 })
      rememberDataDetail(target, 'd1', { item: row('d1', 'todo', 'the whole body'), properties })
      await settle()

      await open()
      expect((await readRepoTable(target))?.items.map((item) => item.id)).toEqual(['d1'])
      expect(peekRepoTable(target)?.totalItems).toBe(1)
      expect((await readDataDetail(target, 'd1'))?.complete).toBe(true)
    } finally {
      await engine.reset()
      await rm(dataDir, { recursive: true, force: true })
    }
  })
})

describe('record page trimming', () => {
  it('keeps the most recently remembered pages and drops the rest', async () => {
    const count = readCache.DETAILS_KEPT + 60
    for (let index = 0; index < count; index += 1) {
      rememberDataDetail(target, `d${index}`, { item: row(`d${index}`, 'todo', ''), properties })
    }
    await settle()

    await readCache.trimDetails()

    const kept = await listClientEngineRecords(LIBRARY_READ_DETAILS_COLLECTION)
    expect(kept).toHaveLength(readCache.DETAILS_KEPT)
    expect(kept.some((page) => page.recordId.endsWith(`:d${count - 1}`))).toBe(true)
  })
})
