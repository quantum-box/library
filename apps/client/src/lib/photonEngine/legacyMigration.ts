/**
 * One-time carry-over from the pre-`@quantum-box/photon` local store.
 *
 * The old implementation wrote its own PGlite schema. It shares table names
 * with the engine's but not columns: `photon_engine_records` declared
 * `value_json` and `updated_at` NOT NULL and the engine writes neither, and
 * `photon_engine_operations` was keyed on `local_sequence` where the engine
 * keys on `operation_id` and additionally requires `kind` and
 * `received_at_ms`. Reusing the directory would mean altering a live database
 * in the user's browser, primary key included, with no way back if it went
 * wrong. So the engine opens a new directory and this moves what has to
 * survive.
 *
 * What has to survive is narrow. `documents` and `attachments` exist **only**
 * here: the Engine sync endpoint the old client pushed to does not exist on
 * library-api, so every push 404'd and was swallowed, and nothing about them
 * ever reached a server. Records are different — the Library API owns them, so
 * they are refetched rather than moved.
 *
 * Pending operations are deliberately dropped. They could never have been
 * pushed (same 404), and replaying writes whose decisions were never seen into
 * a fresh op-log would push them at a server that has no idea they exist.
 *
 * The old directory is left untouched, so a build that predates this still
 * opens the data it knows.
 *
 * TODO(stage-4-followup): library-api now serves `/api/engine/*`, so the
 * premise above is weakening: once a deployment has run with
 * `LIBRARY_PHOTON_ENGINE_ENABLED=true` for long enough that every client has
 * synced, `documents` and `attachments` have a server copy and could be
 * refetched like records rather than carried by hand. Not yet: the flag is off
 * by default, so for now no browser's documents have ever reached a server, and
 * removing this would lose them.
 */

import { PGlite } from '@electric-sql/pglite'
import type { PhotonClient } from '@quantum-box/photon'

import { appKitConfig } from '../../app/kitConfig'

/** Where the pre-engine implementation kept its database. */
export const LEGACY_ENGINE_DATA_DIR = appKitConfig.engine.pgliteDataDir

/** Only what has no other home. `records` come back from the Library API. */
const CARRIED_COLLECTIONS = ['documents', 'attachments'] as const

/**
 * Whether the carry-over writes to this collection.
 *
 * Reads and writes of a carried collection have to wait for it; everything
 * else — `records` above all, which the first screen is drawn from — does not.
 */
export function isCarriedCollection(collection: string): boolean {
  return (CARRIED_COLLECTIONS as readonly string[]).includes(collection)
}

const DONE_MARKER_COLLECTION = '__library_migration'
const DONE_MARKER_RECORD = 'legacy-engine-v1'

/**
 * Cross-tab guard, checked before anything opens the old database.
 *
 * PGlite holds one connection per data directory and a second *tab* on the
 * same directory is invisible to its in-process registry. Opening the old
 * directory on every load in every tab doubled the contention for no reason:
 * after the first success there is nothing left to read. `localStorage` is
 * shared across tabs and synchronous, so the common case costs one string
 * read. The record inside the new store stays the durable answer — this flag
 * can be cleared, and then the marker still stops a second run.
 */
const DONE_FLAG_KEY = `library.photon.legacy-migrated::${LEGACY_ENGINE_DATA_DIR}`

/**
 * A migration that cannot finish must not stop the app from starting.
 * Everything it moves is best-effort, and an unmigrated load retries on the
 * next one, so a bound is strictly better than a hang.
 */
const MIGRATION_TIMEOUT_MS = 10_000

interface LegacyRecordRow {
  collection: string
  record_id: string
  value_json: string | null
  record_json: string | null
  deleted: boolean
}

/**
 * Copy `documents` and `attachments` out of the old database, once.
 *
 * Never throws: a browser that has no old database, or one whose old database
 * is unreadable, must still get a working client. Losing the carry-over is
 * recoverable — the user re-creates a document — whereas failing to start is
 * not.
 */
export async function migrateLegacyEngineData(client: PhotonClient): Promise<void> {
  if (readDoneFlag()) return
  try {
    await withTimeout(carryOver(client), MIGRATION_TIMEOUT_MS)
  } catch (error: unknown) {
    console.warn('[photon] could not carry the previous local store over', error)
  }
}

async function carryOver(client: PhotonClient): Promise<void> {
  if (await alreadyMigrated(client)) {
    writeDoneFlag()
    return
  }

  const rows = await readLegacyRows()
  for (const collection of CARRIED_COLLECTIONS) {
    const items = rows
      .filter((row) => row.collection === collection && !row.deleted)
      .map((row) => ({ recordId: row.record_id, value: decodeValue(row) }))
      .filter((item): item is { recordId: string; value: unknown } => item.value !== undefined)

    if (items.length) client.ingest(collection, items)
  }

  await markMigrated(client, rows.length)
  writeDoneFlag()
}

function readDoneFlag(): boolean {
  try {
    return globalThis.localStorage?.getItem(DONE_FLAG_KEY) === '1'
  } catch {
    // Private-mode browsers throw on access. Fall through to the slow path.
    return false
  }
}

function writeDoneFlag(): void {
  try {
    globalThis.localStorage?.setItem(DONE_FLAG_KEY, '1')
  } catch {
    // Not being able to remember is survivable: the in-store marker still
    // makes a second run a no-op.
  }
}

async function withTimeout<T>(work: Promise<T>, ms: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    return await Promise.race([
      work,
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error(`timed out after ${ms}ms`)), ms)
      }),
    ])
  } finally {
    if (timer !== undefined) clearTimeout(timer)
  }
}

async function alreadyMigrated(client: PhotonClient): Promise<boolean> {
  await client.hydrateCollection(DONE_MARKER_COLLECTION)
  const query = client.liveRecord(DONE_MARKER_COLLECTION, DONE_MARKER_RECORD)
  try {
    return query.getSnapshot().data != null
  } finally {
    query.destroy()
  }
}

async function markMigrated(client: PhotonClient, carried: number): Promise<void> {
  // Ingested, not written as an operation: this is bookkeeping for this device,
  // and it must never reach the push queue.
  client.ingest(DONE_MARKER_COLLECTION, [
    { recordId: DONE_MARKER_RECORD, value: { carried, at: new Date().toISOString() } },
  ])
}

/**
 * Whether an old database is there at all, without opening one.
 *
 * `PGlite.create` does not probe a data directory, it *establishes* one: on a
 * browser that never ran the old build it runs a full `initdb`, writes an
 * empty Postgres into IndexedDB, and only then can be asked whether it holds
 * anything. That is seconds of WASM work — enough to hit
 * `MIGRATION_TIMEOUT_MS` on a loaded machine — to answer "no", and it leaves
 * behind exactly the database it was looking for.
 *
 * `indexedDB.databases()` answers the same question for free. PGlite's `idb://`
 * backend mounts at `/pglite/<dataDir>`, so the old store cannot exist without
 * a database whose name ends in the directory it was given. Suffix, not
 * substring: the engine's own directory is this one plus `-v2`, and a
 * substring test would match it and put us straight back to opening PGlite.
 *
 * Only a listing that runs and comes back without a match is taken as an
 * answer. Where `databases()` is missing (Firefox before 126) this says
 * nothing and the caller opens the store as it always did.
 */
async function legacyDatabaseMissing(): Promise<boolean> {
  try {
    const list = await globalThis.indexedDB?.databases?.()
    if (!list) return false
    return !list.some((database) => namesLegacyDatabase(database.name))
  } catch {
    // A browser that refuses the listing has told us nothing either.
    return false
  }
}

/** Whether an IndexedDB database name is PGlite's for the old data directory. */
export function namesLegacyDatabase(name: string | undefined): boolean {
  return name?.endsWith(LEGACY_ENGINE_DATA_DIR.replace(/^idb:\/\//, '')) ?? false
}

async function readLegacyRows(): Promise<LegacyRecordRow[]> {
  if (await legacyDatabaseMissing()) return []

  let db: PGlite | null = null
  try {
    db = await PGlite.create(LEGACY_ENGINE_DATA_DIR)

    // The old database may predate a column, or not exist at all. `to_regclass`
    // answers without raising, so a fresh install takes the cheap path out.
    const present = await db.query<{ exists: boolean }>(
      "SELECT to_regclass('public.photon_engine_records') IS NOT NULL AS exists"
    )
    if (!present.rows[0]?.exists) return []

    const result = await db.query<LegacyRecordRow>(
      `SELECT collection, record_id, value_json, record_json, deleted
       FROM photon_engine_records
       WHERE scope = $1 AND collection = ANY($2)`,
      [appKitConfig.workspace.scope, [...CARRIED_COLLECTIONS]]
    )
    return result.rows
  } finally {
    await db?.close().catch(() => undefined)
  }
}

/**
 * The old store wrote the value twice: `value_json` for reads and
 * `record_json` for the full engine record. Either can be missing on rows
 * written by older builds, so both are tried before the row is skipped.
 */
function decodeValue(row: LegacyRecordRow): unknown {
  if (row.value_json) {
    try {
      return JSON.parse(row.value_json)
    } catch {
      // fall through to record_json
    }
  }
  if (row.record_json) {
    try {
      const parsed: unknown = JSON.parse(row.record_json)
      if (parsed && typeof parsed === 'object' && 'value' in parsed) {
        return (parsed as { value: unknown }).value
      }
    } catch {
      return undefined
    }
  }
  return undefined
}
