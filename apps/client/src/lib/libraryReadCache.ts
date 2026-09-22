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
 * How long this lasts is Photon's to decide. In `@quantum-box/photon` 0.3,
 * `ingest` keeps rows in the projection only, so today this carries a screen
 * across navigation but not across a reload. It is written against the store
 * rather than around it so that it carries across a reload as soon as
 * ingested rows are durable, with nothing here changing.
 */

import { loadStoredAuthIdentity } from './auth'
import { getBodyProperty } from './libraryTable/bodyProperty'
import {
  getClientEngineRecord,
  ingestClientEngineRecords,
  peekClientEngineRecord,
} from './photonEngine/client'
import type {
  LibraryDataItem,
  LibraryOrganization,
  LibraryProperty,
  LibraryRepository,
} from './recordsApi'

const TABLES_COLLECTION = 'library_read_tables'
const DETAILS_COLLECTION = 'library_read_details'
const WORKSPACE_COLLECTION = 'library_read_workspace'

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
): void {
  void ingestClientEngineRecords(collection, items).catch((error: unknown) => {
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
  remember(TABLES_COLLECTION, [{ recordId: tableKey(target), value: table }])
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
): void {
  const value: CachedDataDetail = { ...detail, complete: true }
  const ids = new Set([requestedId, detail.item.id])
  remember(
    DETAILS_COLLECTION,
    [...ids].map((dataId) => ({ recordId: detailKey(target, dataId), value }))
  )

  const table = peekRepoTable(target)
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
  rememberRepoTable(target, {
    ...table,
    items: table.items.map((candidate) => (candidate.id === row.id ? updated : candidate)),
  })
}

/**
 * Forget a record that has been deleted, so that no screen draws it again.
 *
 * Its row goes too: the table would otherwise open on it next time and keep
 * it on screen until the listing came back without it.
 */
export function forgetData(target: ReadCacheRepository, dataId: string): void {
  remember(DETAILS_COLLECTION, [
    { recordId: detailKey(target, dataId), value: null, deleted: true },
  ])
  const table = peekRepoTable(target)
  if (table?.items.some((row) => row.id === dataId)) {
    rememberRepoTable(target, {
      ...table,
      items: table.items.filter((row) => row.id !== dataId),
      totalItems: table.totalItems === null ? null : Math.max(0, table.totalItems - 1),
    })
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
  remember(WORKSPACE_COLLECTION, [{ recordId: workspaceKey(), value: workspace }])
}
