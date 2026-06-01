import { describe, expect, it } from 'vitest'
import {
  libraryDataItemToGraphqlPropertyData,
  mergeLibraryDataProperty,
  parseEditablePropertyValue,
} from './libraryPropertyInput'
import type { LibraryDataItem, LibraryProperty } from '../recordsApi'

const stringProperty: LibraryProperty = {
  id: 'prop-title',
  name: 'Title',
  typ: 'String',
}

describe('libraryPropertyInput', () => {
  it('merges property values onto a data item', () => {
    const item: LibraryDataItem = {
      id: 'data-1',
      name: 'Row',
      propertyData: [],
    }
    const next = mergeLibraryDataProperty(item, 'prop-title', { string: 'Hello' })
    expect(next.propertyData).toEqual([{ propertyId: 'prop-title', value: { string: 'Hello' } }])
  })

  it('converts property data to GraphQL input payloads', () => {
    const payload = libraryDataItemToGraphqlPropertyData(
      [stringProperty],
      [{ propertyId: 'prop-title', value: { string: 'Alpha' } }]
    )
    expect(payload).toEqual([
      { propertyId: 'prop-title', value: { string: 'Alpha' } },
    ])
  })

  it('parses select values by option label', () => {
    const selectProperty: LibraryProperty = {
      id: 'prop-status',
      name: 'Status',
      typ: 'Select',
      meta: { options: [{ id: 'opt-1', name: 'Todo', key: 'todo' }] },
    }
    expect(parseEditablePropertyValue(selectProperty, 'Todo')).toEqual({ optionId: 'opt-1' })
  })
})
