import { describe, expect, it } from 'vitest'
import { propertyValueEditText, propertyValueText } from './libraryPropertyFormat'
import type { LibraryProperty } from '../recordsApi'

const htmlProperty: LibraryProperty = {
  id: 'prop-body',
  name: 'Body',
  typ: 'Html',
}

describe('propertyValueEditText', () => {
  it('keeps the newlines that the display formatter collapses', () => {
    const value = { html: '<p>first</p>\n\n<p>second</p>' }

    expect(propertyValueText(htmlProperty, value)).toBe('first second')
    expect(propertyValueEditText(htmlProperty, value)).toBe(value.html)
  })

  it('falls back to the display text for every other type', () => {
    const stringProperty: LibraryProperty = { id: 'prop-title', name: 'Title', typ: 'String' }

    expect(propertyValueEditText(stringProperty, { string: 'first\nsecond' })).toBe('first\nsecond')
  })
})
