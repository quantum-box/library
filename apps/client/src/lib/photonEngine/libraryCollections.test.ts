/**
 * The collection name is now the routing decision, so these are the tests for
 * routing: what a name means, and what the client is told a name is.
 */
import { afterEach, describe, expect, it } from 'vitest'

import type { RestResource } from '@quantum-box/photon'

import {
  __testOnly,
  knownLibraryRepositories,
  libraryRecordsCollection,
  libraryRecordsDatabaseId,
  libraryRepositoryByName,
  rememberLibraryRepositories,
  resolveLibraryCollection,
  setLibraryRecordsResourceFactory,
} from './libraryCollections'

const photonCore = {
  databaseId: 'repo-1',
  org: 'quantum-box',
  repo: 'photon-core',
  operatorId: 'operator-1',
  repoName: 'quantum-box / Photon Core',
}

function stubResource(): RestResource<never> {
  return {
    list: async () => [],
    create: async () => undefined,
    update: async () => undefined,
    remove: async () => undefined,
    toRecord: (item: never) => ({ recordId: '', value: item }),
  } as unknown as RestResource<never>
}

afterEach(() => {
  __testOnly.reset()
})

describe('collection names', () => {
  it('names a collection for the repository that owns its records', () => {
    expect(libraryRecordsCollection('repo-1')).toBe('data:repo-1')
    expect(libraryRecordsDatabaseId('data:repo-1')).toBe('repo-1')
  })

  it('is keyed on the canonical id, not the renameable username', () => {
    // ADR-0006 §8: `repo_username` is a navigation shell, not an identity. Two
    // repositories that swap usernames must not swap records.
    expect(libraryRecordsCollection('repo-1')).not.toContain('photon-core')
  })

  it('claims no collection it does not own', () => {
    expect(libraryRecordsDatabaseId('documents')).toBeNull()
    expect(libraryRecordsDatabaseId('library_data_records')).toBeNull()
    expect(libraryRecordsDatabaseId('data:')).toBeNull()
  })
})

describe('the repository registry', () => {
  it('finds a repository by id and by name', () => {
    rememberLibraryRepositories([photonCore])

    expect(libraryRepositoryByName('quantum-box', 'photon-core')).toEqual(photonCore)
    expect(knownLibraryRepositories()).toEqual([photonCore])
  })

  it('reports only what actually changed', () => {
    expect(rememberLibraryRepositories([photonCore])).toEqual([photonCore])
    // Repeating a fact is not news — a caller that stores repositories durably
    // would otherwise write on every hydration.
    expect(rememberLibraryRepositories([photonCore])).toEqual([])
    expect(rememberLibraryRepositories([{ ...photonCore, repoName: 'renamed' }]))
      .toEqual([{ ...photonCore, repoName: 'renamed' }])
  })

  it('ignores a repository with no identity to key on', () => {
    expect(rememberLibraryRepositories([{ databaseId: '', org: 'a', repo: 'b' }])).toEqual([])
    expect(knownLibraryRepositories()).toEqual([])
  })
})

describe('resolveLibraryCollection', () => {
  it('gives a known repository its own rest-backed resource', () => {
    const built: string[] = []
    setLibraryRecordsResourceFactory((repository) => {
      built.push(repository.databaseId)
      return stubResource()
    })
    rememberLibraryRepositories([photonCore])

    const config = resolveLibraryCollection('data:repo-1')

    expect(config?.mode).toBe('rest-backed')
    // One resource per collection, closing over the one repository it is named
    // for. That is what makes its `list()` a repository's data list rather than
    // a search across every repository.
    expect(built).toEqual(['repo-1'])
  })

  it('leaves every other collection to the engine', () => {
    setLibraryRecordsResourceFactory(() => stubResource())

    expect(resolveLibraryCollection('documents')).toBeUndefined()
    expect(resolveLibraryCollection('attachments')).toBeUndefined()
    expect(resolveLibraryCollection('records')).toBeUndefined()
  })

  it('declines a repository it has never heard of', () => {
    // Photon keeps the answer for the client's lifetime, so guessing here
    // would pin a repository to the wrong mode for the whole session. The
    // registry is filled before anything touches a collection instead.
    setLibraryRecordsResourceFactory(() => stubResource())

    expect(resolveLibraryCollection('data:unknown-repo')).toBeUndefined()
  })
})
