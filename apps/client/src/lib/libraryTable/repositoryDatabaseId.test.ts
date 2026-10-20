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
  it.each(['kpi', 'quantumbox/kpi', 'database-kpi'])(
    'does not treat a legacy repository identifier %s as a Database ID', (id) => {
      expect(getRepositoryDatabaseId({ id })).toBeUndefined()
    },
  )
  it('does not offer invalid database metadata as a relation target', () => {
    expect(getRepositoryDatabaseId({ id: 'rp_repository', databases: ['rp_other'] })).toBeUndefined()
  })

})
