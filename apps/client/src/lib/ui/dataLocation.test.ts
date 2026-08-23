import { describe, expect, it, vi } from 'vitest'
import {
  databaseIdFromLocation,
  isDataListPath,
  navigateToData,
  splitRepoDatabaseId,
} from './dataLocation'

describe('splitRepoDatabaseId', () => {
  it('splits org/repo database ids and rejects everything else', () => {
    expect(splitRepoDatabaseId('quantum-box/library')).toEqual({
      organization: 'quantum-box',
      repository: 'library',
    })
    expect(splitRepoDatabaseId(undefined)).toBeNull()
    expect(splitRepoDatabaseId('standalone')).toBeNull()
    expect(splitRepoDatabaseId('/leading')).toBeNull()
    expect(splitRepoDatabaseId('trailing/')).toBeNull()
    expect(splitRepoDatabaseId('a/b/c')).toBeNull()
  })
})

describe('databaseIdFromLocation', () => {
  it('extracts the repo id from data paths and falls back to search', () => {
    expect(databaseIdFromLocation('/quantum-box/library/data', undefined)).toBe(
      'quantum-box/library',
    )
    expect(databaseIdFromLocation('/quantum-box/library/data/rec-1', undefined)).toBe(
      'quantum-box/library',
    )
    expect(databaseIdFromLocation('/databases', 'standalone')).toBe('standalone')
    expect(databaseIdFromLocation('/home', undefined)).toBeUndefined()
  })
})

describe('isDataListPath', () => {
  it('matches all-data and repo data list paths but not detail pages', () => {
    expect(isDataListPath('/databases')).toBe(true)
    expect(isDataListPath('/databases/')).toBe(true)
    expect(isDataListPath('/quantum-box/library/data')).toBe(true)
    expect(isDataListPath('/quantum-box/library/data/rec-1')).toBe(false)
    expect(isDataListPath('/databases/rec-1')).toBe(false)
    expect(isDataListPath('/home')).toBe(false)
  })
})

describe('navigateToData', () => {
  it('routes repo databases to path form and others to search form', () => {
    const navigate = vi.fn()

    navigateToData(navigate as never, 'quantum-box/library', { view: 'board' })
    expect(navigate).toHaveBeenLastCalledWith({
      to: '/$organization/$repository/data',
      params: { organization: 'quantum-box', repository: 'library' },
      search: { view: 'board' },
      replace: undefined,
    })

    navigateToData(
      navigate as never,
      'quantum-box/library',
      {},
      { recordId: 'rec-1', replace: true },
    )
    expect(navigate).toHaveBeenLastCalledWith({
      to: '/$organization/$repository/data/$recordId',
      params: { organization: 'quantum-box', repository: 'library', recordId: 'rec-1' },
      search: {},
      replace: true,
    })

    navigateToData(navigate as never, 'standalone', {})
    expect(navigate).toHaveBeenLastCalledWith({
      to: '/databases',
      search: { database: 'standalone' },
      replace: undefined,
    })

    navigateToData(navigate as never, undefined, {}, { recordId: 'rec-2' })
    expect(navigate).toHaveBeenLastCalledWith({
      to: '/databases/$recordId',
      params: { recordId: 'rec-2' },
      search: { database: undefined },
      replace: undefined,
    })
  })
})
