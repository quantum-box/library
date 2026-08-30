/**
 * The local-first store, on top of `@quantum-box/photon`.
 *
 * This file used to be a 1,552-line reimplementation of that package: its own
 * PGlite schema, operation log, projection, cursor handling, push/pull cycle
 * and decision handling, plus four runtime paths. All of it now comes from the
 * published engine; what is left here is the wiring that is genuinely this
 * app's — which collections exist, where the data lives, and how a request is
 * authorized.
 *
 * The exported functions keep the shapes their callers already use, so
 * `recordsApi`, `docsApi`, `attachmentsApi` and the sync dashboard did not have
 * to change alongside the swap.
 */

import {
  createEngineTransport,
  createPhotonClient,
  type LocalStore,
  type PhotonClient,
  type PhotonRecord,
  type SyncTransport,
} from '@quantum-box/photon'
import type { PhotonKernelModule } from '@quantum-box/photon'
import { createPGliteStore } from '@quantum-box/photon/store-pglite'
import { loadPhotonKernel } from '@quantum-box/photon/wasm'

import { appKitConfig } from '../../app/kitConfig'
import { getValidAuthTokens } from '../auth'
import {
  LIBRARY_REPOSITORIES_COLLECTION,
  type LibraryRecordsRepository,
  rememberLibraryRepositories,
  resolveLibraryCollection,
} from './libraryCollections'
import {
  LEGACY_ENGINE_DATA_DIR,
  isCarriedCollection,
  migrateLegacyEngineData,
} from './legacyMigration'

export type PhotonEngineMutationKind = 'upsert' | 'patch' | 'delete'

export interface PhotonEngineRecord<T = unknown> {
  scope: string
  collection: string
  recordId: string
  value: T
  deleted: boolean
  updatedAt: string
}

export interface ClientEngineDebugState {
  scope: string
  records: number
  cursor: {
    remote: string
    position: number
    updatedAtMs: number
  } | null
  operations: {
    pending: number
    accepted: number
    rejected: number
    conflict: number
    total: number
  }
  recentOperations: Array<{
    operationId: string
    collection: string
    recordId: string
    status: string
    localSequence: number
    createdAt: string
    kind: string
    remoteSequence: number | null
    error: unknown | null
  }>
}

const engineScope = appKitConfig.workspace.scope
const engineActorId = `${appKitConfig.tenant.id}:${appKitConfig.workspace.id}:${appKitConfig.app.id}-client`

/**
 * A new data directory, not the one the old implementation used.
 *
 * The two schemas share table names but not columns — the old
 * `photon_engine_records` declares `value_json` and `updated_at` NOT NULL and
 * the engine writes neither, so its first insert into a pre-existing table
 * fails, and `CREATE TABLE IF NOT EXISTS` does not repair that. Opening a
 * fresh directory and carrying the data across leaves the old one intact to
 * roll back to. See `legacyMigration.ts`.
 */
const engineDataDir = `${LEGACY_ENGINE_DATA_DIR}-v2`

/** Timeout for one durable sync cycle, matching what `docsApi` expected. */
const SYNC_TIMEOUT_MS = 5_000

let clientPromise: Promise<PhotonClient> | null = null

/**
 * The carry-over, running alongside the client rather than in front of it.
 *
 * `migrateLegacyEngineData` never rejects, so awaiting this adds no failure
 * mode — only the wait, and only where the wait is owed. See `engineFor`.
 */
let legacyCarryOver: Promise<void> | null = null

/**
 * Test seam. A test supplies an in-memory store and its own transport rather
 * than opening IndexedDB and talking to a server; nothing else about the
 * client changes, so the code under test is the code that ships.
 */
interface EngineOverrides {
  storage?: LocalStore
  kernel?: PhotonKernelModule
  transport?: SyncTransport
  skipLegacyMigration?: boolean
}
let overrides: EngineOverrides | null = null

async function engine(): Promise<PhotonClient> {
  clientPromise ??= build().catch((error: unknown) => {
    // Do not memoize a failure: a transient WASM fetch or a locked data
    // directory should stay retryable.
    clientPromise = null
    throw error
  })
  return clientPromise
}

/**
 * The client, ready for this collection.
 *
 * The carry-over used to be awaited inside `build`, which meant the first read
 * or write of *any* collection queued behind it — `records` included, which
 * the carry-over never touches and which the first screen is drawn from. On a
 * browser with no old store to carry that was a `MIGRATION_TIMEOUT_MS` stall
 * bolted onto the front of hydration and of every create, and the Library API
 * request it was attached to had already returned.
 *
 * So only `documents` and `attachments` wait: they are what the carry-over
 * writes, and a read that overtook it would report a document the user still
 * has as missing.
 */
async function engineFor(collection: string): Promise<PhotonClient> {
  const client = await engine()
  if (isCarriedCollection(collection)) await legacyCarryOver
  return client
}

async function build(): Promise<PhotonClient> {
  const [storage, kernel] = await Promise.all([
    overrides?.storage ?? createPGliteStore({ dataDir: engineDataDir }),
    overrides?.kernel ?? loadPhotonKernel(),
  ])

  const client = await createPhotonClient({
    scope: engineScope,
    actorId: engineActorId,
    storage,
    kernel,
    transport:
      overrides?.transport ??
      createEngineTransport({
      baseUrl: appKitConfig.server.apiBaseUrl ?? '',
      pushPath: appKitConfig.engine.pushPath,
      pullPath: appKitConfig.engine.pullPath,
      timeoutMs: SYNC_TIMEOUT_MS,
      // The engine has no business knowing which identity provider this app
      // uses; it asks for headers when it is about to send a request. The old
      // implementation sent none, so every push was unauthenticated.
      headers: async (): Promise<Record<string, string>> => {
        const tokens = await getValidAuthTokens()
        if (!tokens?.accessToken) return {}
        return { authorization: `Bearer ${tokens.accessToken}` }
      },
      }),
    // Records are partitioned one collection per repository, and the set of
    // repositories is only known at runtime — so the collections cannot be
    // named when the client is built. The resolver is asked for each one as it
    // is encountered instead. See `libraryCollections`.
    resolveCollection: resolveLibraryCollection,
    // Sync runs when a caller asks for it, as it did before.
    sync: { autoStart: false },
  })

  // Before the client is handed out, and so before anything can ask
  // `resolveCollection` about a `data:` collection: the resolver is consulted
  // once per name and its answer is kept, so a repository learned later than
  // its own collection would be learned too late.
  await restoreKnownRepositories(client)

  legacyCarryOver = overrides?.skipLegacyMigration
    ? Promise.resolve()
    // `migrateLegacyEngineData` reports its own failures and resolves anyway;
    // the catch keeps that true here, where the promise is held and may never
    // be awaited, rather than resting on a promise made in another file.
    : migrateLegacyEngineData(client).catch(() => undefined)
  return client
}

/**
 * Re-learn the repositories a previous session recorded.
 *
 * This is the whole reason the repository list is stored locally: it names the
 * collections, and an offline start cannot ask the Library API what they are.
 * Without it a cold offline load would find no collection to read and show an
 * empty workspace while holding every record on disk.
 */
async function restoreKnownRepositories(client: PhotonClient): Promise<void> {
  await client.hydrateCollection(LIBRARY_REPOSITORIES_COLLECTION)
  const query = client.query<LibraryRecordsRepository>({
    collection: LIBRARY_REPOSITORIES_COLLECTION,
  })
  try {
    rememberLibraryRepositories(
      query
        .getSnapshot()
        .data.filter((record) => record.deletedAt == null)
        .map((record) => record.value)
    )
  } finally {
    query.destroy()
  }
}

function toEngineRecord<T>(record: PhotonRecord<T>): PhotonEngineRecord<T> {
  return {
    scope: engineScope,
    collection: record.key.collection,
    recordId: record.key.record_id,
    value: record.value,
    deleted: record.deletedAt != null,
    updatedAt: String(record.version.wall_time_ms),
  }
}

export async function listClientEngineRecords<T>(
  collection: string
): Promise<PhotonEngineRecord<T>[]> {
  const client = await engineFor(collection)
  await client.hydrateCollection(collection)
  const query = client.query<T>({ collection })
  try {
    return query
      .getSnapshot()
      .data.filter((record) => record.deletedAt == null)
      .map((record) => toEngineRecord<T>(record))
  } finally {
    query.destroy()
  }
}

export async function getClientEngineRecord<T>(
  collection: string,
  recordId: string
): Promise<PhotonEngineRecord<T> | null> {
  const client = await engineFor(collection)
  await client.hydrateCollection(collection)
  const query = client.liveRecord<T>(collection, recordId)
  try {
    const record = query.getSnapshot().data
    if (!record || record.deletedAt != null) return null
    return toEngineRecord<T>(record)
  } finally {
    query.destroy()
  }
}

export async function upsertClientEngineRecord<T>(
  collection: string,
  recordId: string,
  value: T
): Promise<PhotonEngineRecord<T>> {
  const client = await engineFor(collection)
  const handle = client.upsert<T>(collection, recordId, value)
  const stored = (await handle.local) ?? handle.optimistic
  if (!stored) throw new Error(`upsert produced no record for ${collection}/${recordId}`)
  return toEngineRecord<T>(stored)
}

export async function patchClientEngineRecord<T>(
  collection: string,
  recordId: string,
  fields: Partial<T>
): Promise<PhotonEngineRecord<T> | null> {
  const client = await engineFor(collection)
  const existing = await getClientEngineRecord<T>(collection, recordId)
  if (!existing) return null
  const handle = client.patch<T>(collection, recordId, fields)
  const stored = (await handle.local) ?? handle.optimistic
  return stored ? toEngineRecord<T>(stored) : null
}

export async function deleteClientEngineRecord(
  collection: string,
  recordId: string
): Promise<void> {
  const client = await engineFor(collection)
  await client.remove(collection, recordId).local
}

/**
 * Store records fetched from an authority elsewhere, without writing
 * operations for them.
 *
 * The Library API owns `records`, so caching one is not a local edit and must
 * not enter the push queue. The old implementation used `upsert` here, which
 * meant every cached row became a pending operation that the next `documents`
 * save tried to push to the Engine — the pending set is scope-wide, not
 * per-collection.
 *
 * `deleted` is how a row leaves such a collection. `remove()` would queue a
 * delete for the authority to carry out, which is backwards when the authority
 * is the one that has just carried it out.
 */
export async function ingestClientEngineRecords<T>(
  collection: string,
  items: readonly { recordId: string; value: T; deleted?: boolean }[]
): Promise<void> {
  const client = await engineFor(collection)
  client.ingest<T>(collection, items)
}

/**
 * Run one durable sync cycle: push what is queued, then pull.
 *
 * Takes no arguments any more. The base URL, the auth header and the request
 * timeout are the transport's, set once where the client is built, rather than
 * re-supplied by each caller.
 */
export async function syncClientEngineOperations(): Promise<{
  pushed: number
  accepted: number
}> {
  const client = await engine()
  const summary = await client.sync.syncNow('manual')
  return { pushed: summary.pushed, accepted: summary.pushed - summary.rejected }
}

export async function getClientEngineDebugState(): Promise<ClientEngineDebugState> {
  const client = await engine()
  const status = client.sync.getStatus()
  const conflicts = client.conflicts()
  const pending = client.pendingCount()

  return {
    scope: engineScope,
    records: 0,
    cursor:
      status.cursor == null
        ? null
        : { remote: 'photon-server', position: status.cursor, updatedAtMs: 0 },
    operations: {
      pending,
      accepted: 0,
      rejected: 0,
      conflict: conflicts.length,
      total: pending + conflicts.length,
    },
    recentOperations: conflicts.map((conflict) => ({
      operationId: conflict.operationId,
      collection: conflict.key.collection,
      recordId: conflict.key.record_id,
      status: 'conflict',
      localSequence: 0,
      createdAt: new Date(conflict.createdAtMs).toISOString(),
      kind: 'conflict',
      remoteSequence: null,
      error: conflict.reason,
    })),
  }
}

export const __testOnly = {
  /** Drop the memoized client so a test can build a fresh one. */
  async reset(): Promise<void> {
    const existing = clientPromise
    clientPromise = null
    legacyCarryOver = null
    overrides = null
    if (!existing) return
    await existing.then((client) => client.close()).catch(() => undefined)
  },
  /** Build the next client from these instead of IndexedDB and the network. */
  configure(next: EngineOverrides): void {
    overrides = next
  },
  engineScope,
  engineActorId,
  engineDataDir,
}
