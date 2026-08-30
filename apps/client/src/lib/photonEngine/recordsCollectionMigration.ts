/**
 * One-time carry-over from the single records collection to one per repository.
 *
 * Until Stage 3b every repository's records lived in `library_data_records`.
 * Renaming the collections orphans whatever a user's browser already holds
 * there, and a first load with no network would show an empty workspace while
 * the rows sat on disk under a name nothing reads any more.
 *
 * The Library API owns records, so this is a comfort rather than a rescue: a
 * refetch restores everything. That is why it is allowed to give up quietly —
 * a repository this device has no rows for, a value that cannot be routed, a
 * store that will not open. Losing the carry-over costs one refetch; failing
 * to start costs the app.
 *
 * The old rows are left where they are. Deleting them would mean writing
 * delete operations into the push queue for records the Library API still has,
 * and keeping them is the way back if this turns out to have been wrong. They
 * are dead weight in PGlite and nothing reads them once the flag below is set.
 *
 * This deliberately does not touch `documents` or `attachments`. Those exist
 * nowhere but the local store — see `legacyMigration.ts` — and are not
 * partitioned by repository at all.
 */

import { ingestClientEngineRecords, listClientEngineRecords } from './client'
import {
  LEGACY_LIBRARY_RECORDS_COLLECTION,
  type LibraryRecordsRepository,
  libraryRecordsCollection,
} from './libraryCollections'

/** Enough of a cached record to say which repository wrote it. */
interface CarriedRecord {
  id: string
  orgUsername?: string
  repoUsername?: string
}

const DONE_FLAG_KEY = `library.photon.records-collections::${LEGACY_LIBRARY_RECORDS_COLLECTION}`

function readDoneFlag(): boolean {
  try {
    return globalThis.localStorage?.getItem(DONE_FLAG_KEY) === '1'
  } catch {
    // Private-mode browsers throw on access. Running the copy again is
    // harmless — `ingest` is an upsert — so treat this as "not yet".
    return false
  }
}

function writeDoneFlag(): void {
  try {
    globalThis.localStorage?.setItem(DONE_FLAG_KEY, '1')
  } catch {
    // Not being able to remember costs a repeat of an idempotent copy.
  }
}

/** Whether the old collection is still the only place some records live. */
export function legacyRecordsCollectionPending(): boolean {
  return !readDoneFlag()
}

/**
 * Move what the old collection holds into the collection of the repository
 * that owns it.
 *
 * Routing is by `orgUsername` / `repoUsername` on the value, which is the only
 * thing the old rows carry — the very denormalization this migration exists to
 * stop relying on. It is read here once and never again.
 *
 * `repositories` is whatever the caller currently knows. A record for a
 * repository not in that set stays put and is refetched; the flag is still set
 * afterwards, because the alternative is re-opening the old collection on
 * every load forever for rows the API will hand back anyway.
 */
export async function carryLegacyLibraryRecords(
  repositories: readonly LibraryRecordsRepository[]
): Promise<void> {
  if (readDoneFlag() || repositories.length === 0) return

  try {
    const cached = await listClientEngineRecords<CarriedRecord>(
      LEGACY_LIBRARY_RECORDS_COLLECTION
    )
    if (cached.length > 0) {
      const byRepository = new Map<string, { recordId: string; value: CarriedRecord }[]>()
      for (const record of cached) {
        const owner = repositories.find(
          (repository) =>
            repository.org === record.value.orgUsername &&
            repository.repo === record.value.repoUsername
        )
        if (!owner) continue
        const items = byRepository.get(owner.databaseId) ?? []
        items.push({ recordId: record.recordId, value: record.value })
        byRepository.set(owner.databaseId, items)
      }

      for (const [databaseId, items] of byRepository) {
        await ingestClientEngineRecords(libraryRecordsCollection(databaseId), items)
      }
    }
    writeDoneFlag()
  } catch (error: unknown) {
    console.warn('[photon] could not carry cached records to their repository', error)
  }
}

export const __testOnly = { DONE_FLAG_KEY }
