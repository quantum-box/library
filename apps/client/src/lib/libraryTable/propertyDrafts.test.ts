import { describe, expect, it } from 'vitest'
import type { LibraryProperty } from '../recordsApi'
import { canRenameProperty, propertyRenameDraft, tablePropertyTypeChoices } from './propertyDrafts'

describe('propertyDrafts', () => {
  it('renames a Select display label without changing its key or options', () => {
    const property: LibraryProperty = {
      id: 'p1',
      name: 'status',
      typ: 'Select',
      meta: { options: [{ id: 'o1', key: 'draft', name: 'Draft' }] },
    }
    expect(propertyRenameDraft(property, 'state')).toEqual({
      name: 'status',
      displayName: 'state',
      type: 'SELECT',
      options: [{ id: 'o1', identifier: 'draft', label: 'Draft' }],
    })
  })

  it('renames an Id display label without changing its key or generator', () => {
    const property: LibraryProperty = {
      id: 'p2',
      name: 'id',
      typ: 'Id',
      meta: { autoGenerate: true },
    }
    expect(propertyRenameDraft(property, 'key')).toEqual({
      name: 'id',
      displayName: 'key',
      type: 'ID',
      autoGenerateId: true,
    })
  })

  it('renames a Relation display label without changing its key or target', () => {
    const property: LibraryProperty = {
      id: 'p3',
      name: 'linked',
      typ: 'Relation',
      meta: { databaseId: 'repo-9' },
    }
    expect(propertyRenameDraft(property, 'related')).toEqual({
      name: 'linked',
      displayName: 'related',
      type: 'RELATION',
      relationDatabaseId: 'repo-9',
    })
  })

  /** Without the metadata the rename would switch generation off silently. */
  it('refuses a rename the listing cannot describe', () => {
    const property: LibraryProperty = { id: 'p4', name: 'id', typ: 'Id', meta: null }
    expect(canRenameProperty(property)).toBe(false)
    expect(propertyRenameDraft(property, 'key')).toBeNull()
  })

  it('leaves Relation out of the header picker, which cannot ask for a target', () => {
    const choices = [{ value: 'STRING' as const }, { value: 'RELATION' as const }]
    expect(tablePropertyTypeChoices(choices)).toEqual([{ value: 'STRING' }])
  })
})
