/**
 * What each screen last showed, so that going back to it shows that again at
 * once.
 *
 * The Library API stays the authority for everything here, and every screen
 * still asks it on every visit. What changes is what a screen shows while it
 * asks: the answer it got last time, read from Photon, rather than a spinner.
 * The spinner is left for what this device has never seen.
 *
 * `ingest`, never an operation: these rows belong to the Library API, and
 * nothing here may enter the push queue. They are also kept out of the
 * `data:` collections, which hold `DatabaseRecord`s -- a projection of a row
 * that has already dropped every Property a table draws.
 *
 * Keyed by the signed-in user as well as by what was shown, because the store
 * outlives a sign-out and the next account must not open onto this one's rows.
 *
 * Photon stores ingested rows (from 0.5), so this carries a screen across a
 * reload and an app restart as well as across navigation. The collections are
 * lazy -- see `LAZY_LIBRARY_COLLECTIONS` -- so the first read of a session
 * waits for the store to load them, and `peek*` answers from then on.
 */

import { loadStoredAuthIdentity } from './auth'
import { getBodyProperty } from './libraryTable/bodyProperty'
import {
  getClientEngineRecord,
  ingestClientEngineRecords,
  listClientEngineRecords,
  peekClientEngineRecord,
} from './photonEngine/client'
import {
  LIBRARY_READ_DETAILS_COLLECTION as DETAILS_COLLECTION,
  LIBRARY_READ_TABLES_COLLECTION as TABLES_COLLECTION,
  LIBRARY_READ_WORKSPACE_COLLECTION as WORKSPACE_COLLECTION,
} from './photonEngine/libraryCollections'
import type {
  LibraryDataItem,
  LibraryOrganization,
  LibraryProperty,
  LibraryRepository,
} from './recordsApi'

/**
 * How many record pages to keep, and how many to let pile up before trimming.
 *
 * Tables and the workspace lists are bounded by what exists; record pages are
 * bounded by nothing but how long the device has been in use, and each holds a
 * whole body. The slack keeps the trim from running on every visit.
 */
const DETAILS_KEPT = 200
const DETAILS_TRIM_AT = 250
/** Check the count on the first page remembered in a session, then every so often. */
const DETAILS_TRIM_CHECK_EVERY = 25
let detailsRememberedThisSession = 0
/** Off only in tests that trim by hand at a moment of their choosing. */
let autoTrim = true

/**
 * Every write to the record pages, one at a time.
 *
 * Trimming lists the pages and then lists back the ones it keeps as the whole
 * collection; a page written in between would not be in that list, and the
 * complete listing would delete it. Queued with the other page writes, a trim
 * sees everything written before it and nothing is written during it.
 */
let detailWrites: Promise<unknown> = Promise.resolve()
function queueDetailWrite<T>(write: () => Promise<T>): Promise<T> {
  const queued = detailWrites.then(write)
  detailWrites = queued.catch(() => undefined)
  return queued
}

/** A repository, as far as naming its cached screens goes. */
export interface ReadCacheRepository {
  org: string
  repo: string
}

/** The first page of a repository's table, as it was last drawn. */
export interface CachedRepoTable {
  items: LibraryDataItem[]
  properties: LibraryProperty[]
  nextPage: number | null
  totalItems: number | null
}

/** One record's page, as it was last drawn. */
export interface CachedDataDetail {
  item: LibraryDataItem
  properties: LibraryProperty[]
  /**
   * False when the item is a listing row rather than the record itself.
   *
   * A listing carries a preview of a long body, not the body, so a page drawn
   * from one may show the title and the Properties but must not show -- let
   * alone save -- the body it holds.
   */
  complete: boolean
  /**
   * Every name this page is remembered under: its id, and the identifier a
   * route asked for it by. Forgetting the record has to reach all of them, or
   * the URL that was not the one deleted from keeps drawing it.
   */
  ids?: string[]
  /**
   * When it was remembered, by the wall clock. What trimming keeps the most
   * recent of -- the version Photon stamps an ingested row with is read from
   * its clock without advancing it, so it says nothing about recency.
   */
  rememberedAt?: number
  /**
   * Whose page this is, and in which repository -- the part of its key before
   * the record's name. Held in the value because a name may itself contain
   * the separator, so the key cannot be taken apart again.
   */
  owner?: string
}

/** The repositories and organizations the workspace shell last listed. */
export interface CachedWorkspace {
  repositories: LibraryRepository[]
  organizations: LibraryOrganization[]
}

function viewer(): string {
  return loadStoredAuthIdentity()?.userId ?? 'anonymous'
}

function tableKey(target: ReadCacheRepository): string {
  return `${viewer()}:${target.org}/${target.repo}`
}

function detailKey(target: ReadCacheRepository, dataId: string): string {
  return `${tableKey(target)}:${dataId}`
}

/**
 * Store without waiting and without failing.
 *
 * A screen remembers what it drew as a side effect of drawing it, and a store
 * that cannot be written must cost that screen nothing: the next visit simply
 * asks the Library API first, as every visit used to.
 */
function remember<T>(
  collection: string,
  items: readonly { recordId: string; value: T; deleted?: boolean }[]
): Promise<void> {
  return ingestClientEngineRecords(collection, items).catch((error: unknown) => {
    console.warn(`Failed to cache ${collection}`, error)
  })
}

async function read<T>(collection: string, recordId: string): Promise<T | null> {
  try {
    return (await getClientEngineRecord<T>(collection, recordId))?.value ?? null
  } catch {
    return null
  }
}

function peek<T>(collection: string, recordId: string): T | null {
  try {
    return peekClientEngineRecord<T>(collection, recordId)?.value ?? null
  } catch {
    return null
  }
}

/** The table as it was last drawn, if it can be had without waiting. */
export function peekRepoTable(target: ReadCacheRepository): CachedRepoTable | null {
  return peek<CachedRepoTable>(TABLES_COLLECTION, tableKey(target))
}

/** The table as it was last drawn, waiting for the store if it has to. */
export function readRepoTable(target: ReadCacheRepository): Promise<CachedRepoTable | null> {
  return read<CachedRepoTable>(TABLES_COLLECTION, tableKey(target))
}

export function rememberRepoTable(
  target: ReadCacheRepository,
  table: CachedRepoTable
): void {
  void remember(TABLES_COLLECTION, [{ recordId: tableKey(target), value: table }])
}

/**
 * The listing row for a record, as a page that is not yet the record.
 *
 * Only by id: an identifier is derived from the Properties, and resolving one
 * is the detail request's job.
 */
function rowAsDetail(
  table: CachedRepoTable | null,
  dataId: string
): CachedDataDetail | null {
  const item = table?.items.find((row) => row.id === dataId)
  return item ? { item, properties: table!.properties, complete: false } : null
}

/**
 * The record's page, if it can be had without waiting.
 *
 * The record itself where it has been opened before, and otherwise its row in
 * the table it was opened from -- which is the usual way in, and enough to
 * draw everything but the body.
 */
export function peekDataDetail(
  target: ReadCacheRepository,
  dataId: string
): CachedDataDetail | null {
  return (
    peek<CachedDataDetail>(DETAILS_COLLECTION, detailKey(target, dataId)) ??
    rowAsDetail(peekRepoTable(target), dataId)
  )
}

/** The record's page, waiting for the store if it has to. */
export async function readDataDetail(
  target: ReadCacheRepository,
  dataId: string
): Promise<CachedDataDetail | null> {
  return (
    (await read<CachedDataDetail>(DETAILS_COLLECTION, detailKey(target, dataId))) ??
    rowAsDetail(await readRepoTable(target), dataId)
  )
}

/**
 * Remember a record as the detail request returned it, or as a save left it.
 *
 * Under the name it was asked for as well as its id, because a route may
 * carry an identifier, and the next visit to that route asks by the same one.
 *
 * Its row in the cached table is brought up to date too. Otherwise going back
 * to the table after an edit would draw the row as it was before the edit,
 * and the listing would only put the edit back a round trip later -- which,
 * for that round trip, looks like the edit was lost. The row keeps its own
 * body, because a row's body is a preview and this one is the whole thing.
 */
export function rememberDataDetail(
  target: ReadCacheRepository,
  requestedId: string,
  detail: { item: LibraryDataItem; properties: LibraryProperty[] }
): Promise<void> {
  return queueDetailWrite(() => rememberDataDetailNow(target, requestedId, detail))
}

async function rememberDataDetailNow(
  target: ReadCacheRepository,
  requestedId: string,
  detail: { item: LibraryDataItem; properties: LibraryProperty[] }
): Promise<void> {
  // Every name the record is already remembered by, as well as these two:
  // opening it by its id must not drop the identifier it was opened by
  // before, or deleting it by id would leave that URL drawing it. Read, not
  // peeked -- straight after start the pages may not be loaded from disk yet.
  //
  // Only this record's names, though. An identifier is a Property value and
  // can be handed to another record; a page under this name that belongs to
  // someone else is that record's, and this name is taken off it.
  const ids = new Set([requestedId, detail.item.id])
  for (const known of [requestedId, detail.item.id]) {
    const page = await read<CachedDataDetail>(DETAILS_COLLECTION, detailKey(target, known))
    if (!page) continue
    if (page.item.id === detail.item.id) {
      for (const id of page.ids ?? []) ids.add(id)
    } else {
      await detachAlias(target, page, known)
    }
  }
  const value: CachedDataDetail = {
    ...detail,
    complete: true,
    ids: [...ids],
    rememberedAt: Date.now(),
    owner: tableKey(target),
  }
  await remember(
    DETAILS_COLLECTION,
    [...ids].map((dataId) => ({ recordId: detailKey(target, dataId), value }))
  )
  if (detailsRememberedThisSession++ % DETAILS_TRIM_CHECK_EVERY === 0 && autoTrim) {
    void trimDetails()
  }

  await updateCachedRow(target, detail)
}

/**
 * Bring the record's row in the cached table up to date.
 *
 * Read, not peeked: a record opened straight after start can come before the
 * table has been loaded from disk, and the row is there all the same.
 */
async function updateCachedRow(
  target: ReadCacheRepository,
  detail: { item: LibraryDataItem; properties: LibraryProperty[] }
): Promise<void> {
  const table = await readRepoTable(target)
  const row = table?.items.find((candidate) => candidate.id === detail.item.id)
  if (!table || !row) return
  const bodyId = getBodyProperty(detail.properties)?.id
  const updated: LibraryDataItem = {
    ...detail.item,
    propertyData: [
      ...detail.item.propertyData.filter((entry) => entry.propertyId !== bodyId),
      ...row.propertyData.filter((entry) => entry.propertyId === bodyId),
    ],
  }
  await remember(TABLES_COLLECTION, [{
    recordId: tableKey(target),
    value: {
      ...table,
      items: table.items.map((candidate) => (candidate.id === row.id ? updated : candidate)),
    },
  }])
}

/**
 * Take a name off the record it used to belong to.
 *
 * The record's other pages still list the name; rewritten without it, the
 * name now belongs to whichever record is remembered under it next, and
 * forgetting the old record no longer takes the new one's page with it.
 */
async function detachAlias(
  target: ReadCacheRepository,
  page: CachedDataDetail,
  alias: string
): Promise<void> {
  const ids = (page.ids ?? [page.item.id]).filter((id) => id !== alias)
  const value: CachedDataDetail = { ...page, ids }
  await remember(
    DETAILS_COLLECTION,
    ids.map((id) => ({ recordId: detailKey(target, id), value }))
  )
}

/**
 * Keep the most recently remembered record pages, and drop the rest --
 * a record at a time, all its names together.
 *
 * By when each was remembered (`rememberedAt`). A complete listing is the one way to remove rows that are not operations,
 * so the kept pages are listed again and everything else goes. Listing a page
 * Photon already holds unchanged writes nothing, so this costs the deletes.
 */
function trimDetails(): Promise<void> {
  return queueDetailWrite(trimDetailsNow)
}

async function trimDetailsNow(): Promise<void> {
  try {
    const pages = await listClientEngineRecords<CachedDataDetail>(DETAILS_COLLECTION)
    if (pages.length <= DETAILS_TRIM_AT) return
    // A record's names go or stay together. Kept apart, a surviving alias
    // outlives the canonical entry that lists it, and forgetting the record
    // by id -- which starts from that entry -- can no longer find it.
    const records = new Map<string, { pages: typeof pages; latest: number }>()
    for (const page of pages) {
      const record = `${page.value.owner ?? ''}\u0000${page.value.item.id}`
      const group = records.get(record) ?? { pages: [], latest: 0 }
      group.pages.push(page)
      group.latest = Math.max(group.latest, page.value.rememberedAt ?? 0)
      records.set(record, group)
    }
    const kept: typeof pages = []
    for (const group of [...records.values()].sort((a, b) => b.latest - a.latest)) {
      if (kept.length >= DETAILS_KEPT) break
      kept.push(...group.pages)
    }
    await ingestClientEngineRecords(
      DETAILS_COLLECTION,
      kept.map((page) => ({ recordId: page.recordId, value: page.value })),
      { complete: true }
    )
  } catch (error: unknown) {
    console.warn(`Failed to trim ${DETAILS_COLLECTION}`, error)
  }
}

/**
 * Forget a record that has been deleted, so that no screen draws it again.
 *
 * Resolves once no screen can: a caller about to navigate to the table the
 * record was in waits for this, or the table would open on the row.
 *
 * Its row goes too: the table would otherwise open on it next time and keep
 * it on screen until the listing came back without it.
 */
export function forgetData(target: ReadCacheRepository, dataId: string): Promise<void> {
  return queueDetailWrite(() => forgetDataNow(target, dataId))
}

async function forgetDataNow(target: ReadCacheRepository, dataId: string): Promise<void> {
  // Read, not peeked: the page may be on disk and not yet in memory, and its
  // other names are only known from it.
  const page = await read<CachedDataDetail>(DETAILS_COLLECTION, detailKey(target, dataId))
  const ids = new Set([dataId, ...(page?.ids ?? []), ...(page ? [page.item.id] : [])])
  await remember(
    DETAILS_COLLECTION,
    [...ids].map((id) => ({ recordId: detailKey(target, id), value: null, deleted: true }))
  )
  const table = await readRepoTable(target)
  if (table?.items.some((row) => ids.has(row.id))) {
    const items = table.items.filter((row) => !ids.has(row.id))
    const removed = table.items.length - items.length
    await remember(TABLES_COLLECTION, [{ recordId: tableKey(target), value: {
      ...table,
      items,
      totalItems: table.totalItems === null ? null : Math.max(0, table.totalItems - removed),
    } }])
  }
}

const WORKSPACE_RECORD = 'workspace'

function workspaceKey(): string {
  return `${viewer()}:${WORKSPACE_RECORD}`
}

/** The workspace shell's lists, waiting for the store if it has to. */
export function readWorkspace(): Promise<CachedWorkspace | null> {
  return read<CachedWorkspace>(WORKSPACE_COLLECTION, workspaceKey())
}

export function rememberWorkspace(workspace: CachedWorkspace): void {
  void remember(WORKSPACE_COLLECTION, [{ recordId: workspaceKey(), value: workspace }])
}

export const __testOnly = {
  trimDetails,
  DETAILS_KEPT,
  setAutoTrim(enabled: boolean): void {
    autoTrim = enabled
  },
}
