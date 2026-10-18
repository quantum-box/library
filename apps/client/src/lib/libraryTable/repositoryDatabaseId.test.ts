import { describe, expect, it } from 'vitest'
import { getRepositoryDatabaseId } from './repositoryDatabaseId'

describe('repository relation targets', () => {
  it('uses the backing Database ID instead of the Repo ID', () => {
    expect(getRepositoryDatabaseId({ id: 'rp_repository', databases: ['db_records'] })).toBe('db_records')
  })
  it('does not submit a Repo ID when database metadata is missing', () => {
    expect(getRepositoryDatabaseId({ id: 'rp_repository' })).toBeUndefined()
  })
  it('supports legacy repositories identified by Database IDs', () => {
    expect(getRepositoryDatabaseId({ id: 'db_records' })).toBe('db_records')
  })
})
