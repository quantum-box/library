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
  newId,
  type AckResult,
  type Conflict,
  type LocalStore,
  type MutationHandle,
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
    // Sync runs when a caller asks for it, as it did before — except while
    // something is queued, which `followQueue` handles.
    sync: { autoStart: false },
  })

  // Stop as soon as the queue drains. Started in `followQueue`, and the pair
  // is what keeps the loop's cost proportional to there being unsent work.
  client.sync.subscribe(() => {
    if (client.pendingCount() === 0) client.sync.stop()
  })

  // A queued write's verdict arrives with nobody waiting on it: `pushMutation`
  // has already reported `queued` and its caller has moved on. When that
  // verdict is a rejection Photon puts its own projection back, and this is
  // the only announcement of it. See `subscribeClientEngineRollbacks`.
  client.subscribeChanges((changeSet) => {
    if (changeSet.origin !== 'rollback' || rollbackListeners.size === 0) return

    const changes = changeSet.changes
      // Photon emits the release of the pending marker as a rollback change of
      // its own, carrying the value the write had already put on screen. It is
      // the reprojection that follows which holds the rolled-back value, so a
      // change that leaves value and deletion alone has nothing to reconcile.
      .filter(
        (change) =>
          change.previous?.value !== change.next?.value ||
          change.previous?.deletedAt !== change.next?.deletedAt
      )
      .map((change) => ({
        collection: change.collection,
        recordId: change.recordId,
        record:
          change.next && change.next.deletedAt == null
            ? toEngineRecord(change.next)
            : null,
      }))
    if (changes.length === 0) return

    for (const listener of rollbackListeners) listener(changes)
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

/**
 * One record the engine moved on its own, after the write that moved it had
 * already returned.
 */
export interface ClientEngineRecordChange<T = unknown> {
  collection: string
  recordId: string
  /** Where the record now stands locally, or `null` when it no longer does. */
  record: PhotonEngineRecord<T> | null
}

type ClientEngineRollbackListener = (
  changes: readonly ClientEngineRecordChange[]
) => void

/**
 * Held here rather than on the client, because a listener outlives one.
 *
 * The client is built lazily and rebuilt after a `__testOnly.reset`, and a
 * subscriber has no way to know when either happens — subscribing through the
 * client would mean either forcing a build (a WASM fetch and an IndexedDB
 * open) at subscribe time, or losing the subscription on the next rebuild.
 * `build` attaches the one fan-out that feeds this set.
 */
const rollbackListeners = new Set<ClientEngineRollbackListener>()

/**
 * Watch for writes the server refused after they were reported as `queued`.
 *
 * Every projection built on top of the engine — the Yjs document records are
 * drawn from, above all — is written by the caller of a write, from that
 * write's return value. That works while the verdict arrives inside the call.
 * A queued write has no verdict to return: it is decided on a later sync
 * cycle, and Photon's rollback then reaches its own projection and stops
 * there, leaving the refused create, edit or delete on screen for good.
 *
 * Only rollbacks. `remote` changes are the pull's business and reconciling
 * those is what hydration already does; a rollback is the one decision nothing
 * else re-reads, because the caller was told the write had been kept.
 */
export function subscribeClientEngineRollbacks(
  listener: ClientEngineRollbackListener
): () => void {
  rollbackListeners.add(listener)
  return () => {
    rollbackListeners.delete(listener)
  }
}

/**
 * A verdict that arrived after the write reporting it had already returned.
 *
 * The counterpart of `ClientEngineWriteResult` for a write that was reported
 * `queued`: same three statuses, and `record` read once the cycle carrying the
 * verdict was over rather than in the middle of it.
 */
export interface ClientEngineSettlement<T = unknown> {
  status: 'accepted' | 'rejected' | 'conflict'
  collection: string
  recordId: string
  /** Where the record now stands locally, or `null` when it no longer does. */
  record: PhotonEngineRecord<T> | null
  reason?: string
  conflictId?: string
}

type ClientEngineSettlementListener = (
  settlement: ClientEngineSettlement
) => void

/** Held for the same reason `rollbackListeners` is. */
const settlementListeners = new Set<ClientEngineSettlementListener>()

/**
 * Watch every late verdict, not only the ones that roll a write back.
 *
 * `subscribeClientEngineRollbacks` reports what Photon undid, which covers a
 * rejection and nothing else. Two other late verdicts leave a projection built
 * from the write's return value just as stale:
 *
 * - `conflict`. The local value goes onto a conflict row and the projection
 *   returns to the server's, so what the caller drew is no longer what the
 *   engine holds — but nothing was rolled back, so no rollback is emitted.
 * - `accepted`. A create is pushed with a client-minted id and the server
 *   fills in what it derives from it; `createServerRecord` says as much of
 *   `identifier`, and for a queued create that value arrives with the pull,
 *   long after the placeholder was handed back.
 *
 * The record is read after the sync cycle finishes, because in both cases it
 * is that cycle's *pull* that carries the value worth reporting.
 */
export function subscribeClientEngineSettlements(
  listener: ClientEngineSettlementListener
): () => void {
  settlementListeners.add(listener)
  return () => {
    settlementListeners.delete(listener)
  }
}

/**
 * Resolve once the sync loop is out of a cycle.
 *
 * A verdict is delivered from inside `syncNow`, before the pull that goes with
 * it has been applied — reading the record there would report the value the
 * cycle is about to replace. `phase` leaving `syncing` is the loop saying the
 * cycle is done.
 *
 * Bounded, because a listener that never fires would strand the announcement:
 * a stalled cycle is worth reporting late, not never.
 */
function syncCycleSettled(client: PhotonClient): Promise<void> {
  if (client.sync.getStatus().phase !== 'syncing') return Promise.resolve()
  return new Promise((resolve) => {
    let unsubscribe: (() => void) | null = null
    let done = false
    const finish = (): void => {
      if (done) return
      done = true
      clearTimeout(timer)
      unsubscribe?.()
      resolve()
    }
    const timer = setTimeout(finish, SYNC_TIMEOUT_MS)
    unsubscribe = client.sync.subscribe(() => {
      if (client.sync.getStatus().phase !== 'syncing') finish()
    })
    if (client.sync.getStatus().phase !== 'syncing') finish()
  })
}

/**
 * Wait out a queued write and tell the subscribers what became of it.
 *
 * `handle.settled` resolves whenever the verdict lands, which for a write made
 * offline is the cycle after the network returns. Nothing is awaiting it by
 * then — `pushMutation` reported `queued` and returned — so this is where that
 * promise is finally read.
 */
async function announceSettlement(
  handle: MutationHandle<unknown>,
  collection: string,
  recordId: string
): Promise<void> {
  const decision = await handle.settled.then(
    (result) => result,
    () => null
  )
  if (!decision) return

  const client = await engine()
  await syncCycleSettled(client)
  const record = await getClientEngineRecord(collection, recordId)

  const settlement: ClientEngineSettlement = {
    status: decision.status,
    collection,
    recordId,
    record,
    ...(decision.status === 'rejected' ? { reason: decision.reason } : {}),
    ...(decision.status === 'conflict' ? { conflictId: decision.conflictId } : {}),
  }
  for (const listener of settlementListeners) listener(settlement)
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
 * What became of a write that was pushed as well as stored.
 *
 * `queued` is the interesting one and it is not a failure: the operation is in
 * the durable log and will go out on a later cycle. It is what an offline
 * write looks like, and what a write looks like while the server is down.
 * Callers show the record and move on.
 *
 * `rejected` means the server refused it and Photon has already replayed the
 * record back to what it was, so `record` is the rolled-back value — `null`
 * when the write was the record's creation. `conflict` means the server had a
 * competing version; the local value is kept on the conflict row in
 * `listClientEngineConflicts`, and the projection returns to the server's.
 */
export interface ClientEngineWriteResult<T> {
  status: 'accepted' | 'queued' | 'rejected' | 'conflict'
  /** The record as it now stands locally, after any rollback. */
  record: PhotonEngineRecord<T> | null
  reason?: string
  conflictId?: string
}

/**
 * Push one just-issued mutation and report what the server said.
 *
 * The verdict is captured before the cycle starts rather than awaited after
 * it, because `handle.settled` only ever resolves when a decision exists —
 * awaiting it directly would hang forever on the offline write this function
 * exists to support. Photon resolves it from inside `handleDecision`, which
 * runs while `syncNow` is still going, so by the time the cycle's promise
 * resumes us the listener below has already run: microtasks queued earlier
 * run first. No decision by then means there was none to have.
 *
 * The whole cycle is awaited, push and pull both. Returning as soon as the
 * decision landed would leave the pull running against a store the caller is
 * free to close, and would report a record the same cycle is about to
 * refresh — the server-assigned identifier arrives on that pull.
 *
 * A transport failure is swallowed: it is not the write's verdict. The
 * operation stays queued and the caller keeps its record.
 */
async function pushMutation<T>(
  handle: MutationHandle<T>
): Promise<ClientEngineWriteResult<T>> {
  const stored = (await handle.local) ?? handle.optimistic

  // A holder rather than a bare `let`: the assignment happens inside a
  // callback, which TypeScript's flow analysis does not follow, so a plain
  // variable would still read as `null` below.
  const settled: { decision: AckResult | null } = { decision: null }
  void handle.settled.then(
    (result) => {
      settled.decision = result
    },
    () => undefined
  )
  await syncClientEngineOperations().catch(() => undefined)
  const decision = settled.decision
  followQueue(await engine())

  const collection = stored?.key.collection
  const recordId = stored?.key.record_id
  const current =
    collection && recordId
      ? await getClientEngineRecord<T>(collection, recordId)
      : stored
        ? toEngineRecord<T>(stored)
        : null

  if (!decision) {
    // The one write whose verdict nobody is holding on to. Everything else
    // here returns a decision the caller acts on; this one is decided on a
    // later cycle, so the wait is handed to `announceSettlement` instead of
    // being abandoned.
    if (collection && recordId) {
      void announceSettlement(handle as MutationHandle<unknown>, collection, recordId)
    }
    return { status: 'queued', record: current }
  }
  if (decision.status === 'rejected') {
    return { status: 'rejected', record: current, reason: decision.reason }
  }
  if (decision.status === 'conflict') {
    return { status: 'conflict', record: current, conflictId: decision.conflictId }
  }
  return { status: 'accepted', record: current }
}

/**
 * Run the sync loop while, and only while, something is waiting to go out.
 *
 * `sync.start()` is what installs the `online` listener, the visibility
 * handler and the backoff retry. Without it a write that could not be pushed
 * sits in the durable log until the user happens to make another one — which
 * is not what "it goes out when the network returns" means, and was the gap
 * between what a queued write promised and what it did.
 *
 * Tied to the queue rather than left running, because the loop also polls, and
 * a poll pulls: an idle client would re-list a repository every interval for
 * no reason. Started here when a push leaves something behind, stopped in
 * `build` as soon as the queue drains.
 */
function followQueue(client: PhotonClient): void {
  if (client.pendingCount() > 0) client.sync.start()
}

/**
 * Write a record and push it in one call.
 *
 * The `upsert` kind specifically, not `patch`: it is the only one a
 * `rest-backed` collection routes to `RestResource.upsert`, and therefore the
 * only one whose first write reaches a create-or-update endpoint instead of an
 * update-only one. See `createLibraryRecordsResource` in `recordsApi`.
 */
export async function upsertAndPushClientEngineRecord<T>(
  collection: string,
  recordId: string,
  value: T
): Promise<ClientEngineWriteResult<T>> {
  const client = await engineFor(collection)
  return pushMutation<T>(client.upsert<T>(collection, recordId, value))
}

/**
 * Merge fields into a record and push it.
 *
 * Reports `rejected` with a null record when there is nothing to patch, rather
 * than inventing a base: a patch against an absent record merges into `{}` and
 * would push a record made of nothing but the changed fields.
 */
export async function patchAndPushClientEngineRecord<T>(
  collection: string,
  recordId: string,
  fields: Partial<T>
): Promise<ClientEngineWriteResult<T>> {
  const client = await engineFor(collection)
  const existing = await getClientEngineRecord<T>(collection, recordId)
  if (!existing) {
    return { status: 'rejected', record: null, reason: 'record not found' }
  }
  return pushMutation<T>(client.patch<T>(collection, recordId, fields))
}

export async function deleteAndPushClientEngineRecord<T = unknown>(
  collection: string,
  recordId: string
): Promise<ClientEngineWriteResult<T>> {
  const client = await engineFor(collection)
  return pushMutation<T>(client.remove<T>(collection, recordId))
}

/** Unresolved conflict rows, for a collection or for the whole scope. */
export async function listClientEngineConflicts(
  collection?: string
): Promise<readonly Conflict[]> {
  const client = collection ? await engineFor(collection) : await engine()
  return client.conflicts(collection)
}

/** A record id this client can use before it has ever reached a server. */
export function newClientEngineRecordId(prefix?: string): string {
  return newId(prefix)
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
 *
 * `complete` says `items` is the *whole* collection, which is the only thing
 * that makes "deleted upstream" distinguishable from "not on this page", so
 * pass it only for a listing that paged to the end. Records carrying an
 * unpushed local write are never reconciled away, so a create made offline
 * survives a refetch that predates it.
 */
export async function ingestClientEngineRecords<T>(
  collection: string,
  items: readonly { recordId: string; value: T; deleted?: boolean }[],
  options?: { complete?: boolean }
): Promise<void> {
  const client = await engineFor(collection)
  client.ingest<T>(collection, items, options)
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
  /** The live client, for a test that needs to watch the sync loop itself. */
  client(): Promise<PhotonClient> {
    return engine()
  },
  engineScope,
  engineActorId,
  engineDataDir,
}
