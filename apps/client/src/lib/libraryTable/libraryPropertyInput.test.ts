import { describe, expect, it } from 'vitest'
import {
  clearedPropertyValue,
  isMultilineEditableProperty,
  libraryPropertyValueToGraphqlInput,
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

  it('keeps Markdown and HTML values in their schema-specific fields', () => {
    const markdownProperty: LibraryProperty = {
      id: 'prop-body',
      name: 'Body',
      typ: 'Markdown',
    }
    const htmlProperty: LibraryProperty = {
      id: 'prop-html',
      name: 'Rendered body',
      typ: 'Html',
    }

    expect(parseEditablePropertyValue(markdownProperty, '# Hello')).toEqual({
      markdown: '# Hello',
    })
    expect(parseEditablePropertyValue(htmlProperty, '<p>Hello</p>')).toEqual({
      html: '<p>Hello</p>',
    })
    expect(libraryDataItemToGraphqlPropertyData(
      [markdownProperty, htmlProperty],
      [
        { propertyId: markdownProperty.id, value: { markdown: '# Hello' } },
        { propertyId: htmlProperty.id, value: { html: '<p>Hello</p>' } },
      ],
    )).toEqual([
      { propertyId: markdownProperty.id, value: { markdown: '# Hello' } },
      { propertyId: htmlProperty.id, value: { html: '<p>Hello</p>' } },
    ])
  })

  it('encodes an emptied value as a clear command instead of dropping the Property', () => {
    const dateProperty: LibraryProperty = { id: 'prop-date', name: 'Date', typ: 'Date' }
    const selectProperty: LibraryProperty = {
      id: 'prop-status',
      name: 'Status',
      typ: 'Select',
      meta: { options: [{ id: 'opt-1', name: 'Todo', key: 'todo' }] },
    }

    // parseEditablePropertyValue has no empty form for these types, and an
    // omitted Property keeps its server value, so the empty value has to be
    // spelled out and survive encoding.
    expect(parseEditablePropertyValue(dateProperty, '')).toBeNull()
    expect(clearedPropertyValue(dateProperty)).toEqual({ date: '' })
    expect(libraryPropertyValueToGraphqlInput(dateProperty, { date: '' })).toEqual({ date: '' })
    expect(libraryPropertyValueToGraphqlInput(selectProperty, { optionId: '' })).toEqual({
      select: '',
    })
    expect(libraryDataItemToGraphqlPropertyData(
      [dateProperty],
      [{ propertyId: dateProperty.id, value: { date: '' } }],
    )).toEqual([{ propertyId: dateProperty.id, value: { date: '' } }])
  })

  it('edits multi-line values multi-line so a single-line input cannot strip the newlines', () => {
    const markdownProperty: LibraryProperty = { id: 'prop-body', name: 'Body', typ: 'Markdown' }

    expect(isMultilineEditableProperty(stringProperty, 'one line')).toBe(false)
    expect(isMultilineEditableProperty(stringProperty, 'first\nsecond')).toBe(true)
    expect(isMultilineEditableProperty(markdownProperty, '')).toBe(true)
  })
})
