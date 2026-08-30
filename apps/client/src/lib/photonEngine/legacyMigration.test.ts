/**
 * The carry-over decides whether to open the old store from the IndexedDB
 * listing alone, because opening PGlite to ask would create the very database
 * it is asking about. That makes the name test load-bearing: say "present"
 * when it is not and every tab pays a Postgres boot to learn nothing; say
 * "absent" when it is there and the user's local documents are dropped.
 */
import { describe, expect, it } from 'vitest'

import { LEGACY_ENGINE_DATA_DIR, isCarriedCollection, namesLegacyDatabase } from './legacyMigration'

const legacyDir = LEGACY_ENGINE_DATA_DIR.replace(/^idb:\/\//, '')

describe('namesLegacyDatabase', () => {
  it('recognizes the database PGlite mounts for the old data directory', () => {
    expect(namesLegacyDatabase(`/pglite/${legacyDir}`)).toBe(true)
  })

  it('does not mistake the engine\'s own directory for the old one', () => {
    // The engine opens `<legacy>-v2`, so the old directory is a *prefix* of the
    // live one. A substring test would match here and send us back to opening
    // PGlite on every load.
    expect(namesLegacyDatabase(`/pglite/${legacyDir}-v2`)).toBe(false)
    expect(namesLegacyDatabase(`photon-commit-journal:${LEGACY_ENGINE_DATA_DIR}-v2`)).toBe(false)
  })

  it('ignores the other stores this app keeps alongside it', () => {
    expect(namesLegacyDatabase('/pglite/library-docs-tenant-library')).toBe(false)
    expect(namesLegacyDatabase(undefined)).toBe(false)
  })
})

describe('isCarriedCollection', () => {
  it('covers the collections that exist nowhere but the local store', () => {
    expect(isCarriedCollection('documents')).toBe(true)
    expect(isCarriedCollection('attachments')).toBe(true)
  })

  it('leaves records out — the Library API owns them, so they are refetched', () => {
    expect(isCarriedCollection('records')).toBe(false)
    expect(isCarriedCollection('library_data_records')).toBe(false)
  })
})
