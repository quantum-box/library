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
 */

import { PGlite } from '@electric-sql/pglite'
import type { PhotonClient } from '@quantum-box/photon'

import { appKitConfig } from '../../app/kitConfig'

/** Where the pre-engine implementation kept its database. */
export const LEGACY_ENGINE_DATA_DIR = appKitConfig.engine.pgliteDataDir

/** Only what has no other home. `records` come back from the Library API. */
const CARRIED_COLLECTIONS = ['documents', 'attachments'] as const

const DONE_MARKER_COLLECTION = '__library_migration'
const DONE_MARKER_RECORD = 'legacy-engine-v1'

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
  try {
    if (await alreadyMigrated(client)) return

    const rows = await readLegacyRows()
    for (const collection of CARRIED_COLLECTIONS) {
      const items = rows
        .filter((row) => row.collection === collection && !row.deleted)
        .map((row) => ({ recordId: row.record_id, value: decodeValue(row) }))
        .filter((item): item is { recordId: string; value: unknown } => item.value !== undefined)

      if (items.length) client.ingest(collection, items)
    }

    await markMigrated(client, rows.length)
  } catch (error: unknown) {
    console.warn('[photon] could not carry the previous local store over', error)
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

async function readLegacyRows(): Promise<LegacyRecordRow[]> {
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
