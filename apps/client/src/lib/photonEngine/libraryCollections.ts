/**
 * Which Photon collection a Library record belongs to.
 *
 * Library's domain is Organization → Repository → Data, and a Data row is
 * owned by a Repository. Photon's `RecordKey` has two axes above the record —
 * `scope` and `collection` — and this app used to spend neither on that
 * ownership: every repository's records went into one `library_data_records`
 * collection and the owning repository was carried *inside the value*, as
 * `orgUsername` / `repoUsername` / `operatorId`. Every write then rebuilt its
 * destination from those fields with a fallback chain, and a listing had to
 * fan out across repositories to find out what it held.
 *
 * Here the ownership moves up one axis: one collection per repository, named
 * for the repository. A record's key now says where it goes, so a write has
 * nothing to reconstruct and a listing has nothing to disambiguate.
 *
 * The name is built from the **canonical Library id**, not `{org}/{repo}`.
 * ADR-0006 §8: "Library の Repo は Database への application-level mapping /
 * navigation shell であり、`repo_username` を authorization key や Durable
 * Object identity にしない". A username is renameable and org-relative; the
 * database id is neither, and it is what the API keys on.
 */

import type { CollectionConfig, RestResource } from '@quantum-box/photon'

/** A repository, in the terms a records collection needs to reach its API. */
export interface LibraryRecordsRepository {
  /** The Library `database_id`. The collection's identity. */
  databaseId: string
  org: string
  repo: string
  operatorId?: string
  repoName?: string
}

/**
 * The single collection every repository's records used to share.
 *
 * Read-only from here on: `carryLegacyLibraryRecords` copies out of it, and
 * the offline fallback in `recordsApi` reads it on the one start where the
 * per-repository collections do not exist yet. Nothing writes to it.
 */
export const LEGACY_LIBRARY_RECORDS_COLLECTION = 'library_data_records'

/**
 * Where the known repositories are kept so the next start knows them.
 *
 * The collection set is discovered from the Library API, which an offline
 * start cannot reach — and without the set there is no way to name the
 * collections holding the cached records, so a cold offline load would show
 * an empty workspace despite having every row on disk. Ingested, never
 * written as operations: this is a local index of someone else's data.
 */
export const LIBRARY_REPOSITORIES_COLLECTION = 'library_repositories'

const COLLECTION_PREFIX = 'data:'

/** The collection holding one repository's records. */
export function libraryRecordsCollection(databaseId: string): string {
  return `${COLLECTION_PREFIX}${databaseId}`
}

/** The database id a records collection is named for, or null for any other. */
export function libraryRecordsDatabaseId(collection: string): string | null {
  return collection.startsWith(COLLECTION_PREFIX)
    ? collection.slice(COLLECTION_PREFIX.length) || null
    : null
}

const repositories = new Map<string, LibraryRecordsRepository>()

function repoKey(org: string, repo: string): string {
  return `${org}/${repo}`
}

/**
 * Record what a repository is, so a collection named for it can be resolved.
 *
 * Called wherever the app learns of a repository, and always before anything
 * touches that repository's collection: the resolver below is consulted once
 * per collection name and its answer is kept for the client's lifetime, so a
 * repository that is unknown at that moment would stay unknown.
 *
 * Returns only what this call actually changed, so a caller that also stores
 * repositories durably can tell a fresh fact from a repeated one.
 */
export function rememberLibraryRepositories(
  known: readonly LibraryRecordsRepository[]
): LibraryRecordsRepository[] {
  const changed: LibraryRecordsRepository[] = []
  for (const repository of known) {
    if (!repository.databaseId || !repository.org || !repository.repo) continue
    const held = repositories.get(repository.databaseId)
    if (held && sameRepository(held, repository)) continue
    repositories.set(repository.databaseId, repository)
    changed.push(repository)
  }
  return changed
}

function sameRepository(
  held: LibraryRecordsRepository,
  next: LibraryRecordsRepository
): boolean {
  return (
    held.org === next.org &&
    held.repo === next.repo &&
    held.operatorId === next.operatorId &&
    held.repoName === next.repoName
  )
}

export function libraryRepositoryByName(
  org: string,
  repo: string
): LibraryRecordsRepository | undefined {
  const wanted = repoKey(org, repo)
  for (const repository of repositories.values()) {
    if (repoKey(repository.org, repository.repo) === wanted) return repository
  }
  return undefined
}

export function knownLibraryRepositories(): LibraryRecordsRepository[] {
  return [...repositories.values()]
}

type LibraryRecordsResourceFactory = (
  repository: LibraryRecordsRepository
) => RestResource<never>

let resourceFactory: LibraryRecordsResourceFactory | null = null

/**
 * Hand over the resource builder.
 *
 * `recordsApi` owns everything a records resource needs — the base URL, the
 * auth headers, the property mapping — and `photonEngine/client` owns the
 * client the resolver is passed to. Registering the builder rather than
 * importing it keeps that a one-way dependency: `client` never imports
 * `recordsApi`.
 */
export function setLibraryRecordsResourceFactory(
  factory: LibraryRecordsResourceFactory
): void {
  resourceFactory = factory
}

/**
 * `createPhotonClient`'s `resolveCollection`: the config for a collection the
 * client was not built with.
 *
 * One `RestResource` per collection, closing over the one repository the
 * collection is named for — which is what makes `list()` unambiguous and a
 * write's destination structural rather than reconstructed.
 */
export function resolveLibraryCollection(
  collection: string
): CollectionConfig | undefined {
  const databaseId = libraryRecordsDatabaseId(collection)
  if (!databaseId) return undefined
  const repository = repositories.get(databaseId)
  if (!repository || !resourceFactory) return undefined
  return { mode: 'rest-backed', resource: resourceFactory(repository) }
}

export const __testOnly = {
  reset(): void {
    repositories.clear()
    resourceFactory = null
  },
}
