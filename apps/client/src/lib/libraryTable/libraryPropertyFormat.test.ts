import { describe, expect, it } from 'vitest'
import {
  CELL_TEXT_LIMIT,
  propertyCellText,
  propertyValueEditText,
  propertyValueText,
} from './libraryPropertyFormat'
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

const richTextProperty: LibraryProperty = {
  id: 'prop-content',
  name: 'Content',
  typ: 'RichText',
}

describe('a listing preview', () => {
  it('reads as the value text', () => {
    const value = { preview: { text: 'The opening line', truncated: true } }

    expect(propertyValueText(richTextProperty, value)).toBe('The opening line')
  })

  /**
   * The guard that stops a table edit from truncating a body: a preview is
   * an abbreviation of the document, so seeding an editor with it would
   * stage the abbreviation as the new document.
   */
  it('is never handed to an editor', () => {
    const value = { preview: { text: 'The opening line', truncated: true } }

    expect(propertyValueEditText(richTextProperty, value)).toBeUndefined()
  })

  it('gives way to the document once the record itself is loaded', () => {
    const document = JSON.stringify([
      { type: 'paragraph', content: [{ type: 'text', text: 'The whole thing' }] },
    ])
    const value = { richText: document, preview: { text: 'The whole', truncated: true } }

    expect(propertyValueEditText(richTextProperty, value)).toBe(document)
  })
})

describe('propertyCellText', () => {
  it('carries a listing preview through with its truncation flag', () => {
    const cell = propertyCellText(richTextProperty, {
      preview: { text: 'The opening line', truncated: true },
    })

    expect(cell).toEqual({ text: 'The opening line', truncated: true })
  })

  /**
   * A value read from a single record arrives uncapped, and the cell is one
   * line high either way -- so the cut happens before the text reaches the
   * DOM rather than in CSS.
   */
  it('caps a document that arrives in full', () => {
    const long = 'x'.repeat(CELL_TEXT_LIMIT + 50)
    const document = JSON.stringify([
      { type: 'paragraph', content: [{ type: 'text', text: long }] },
    ])

    const cell = propertyCellText(richTextProperty, { richText: document })

    expect(cell?.text).toHaveLength(CELL_TEXT_LIMIT)
    expect(cell?.truncated).toBe(true)
  })

  it('leaves a short value alone', () => {
    const cell = propertyCellText(richTextProperty, {
      preview: { text: 'Short', truncated: false },
    })

    expect(cell).toEqual({ text: 'Short', truncated: false })
  })

  /**
   * The API caps its preview by Rust `char`s. Measuring the same body with
   * `String.length` counts UTF-16 units, so a 200-code-point value ending in
   * an emoji read as over-limit and got sliced through its surrogate pair.
   */
  it('measures the cell limit in code points, not UTF-16 units', () => {
    const property = { id: 'p', name: 'Body', typ: 'RichText' } as LibraryProperty
    const body = `${'a'.repeat(199)}\u{1F600}`
    const cell = propertyCellText(property, { string: body }, 200)
    expect(Array.from(body)).toHaveLength(200)
    expect(body.length).toBe(201)
    expect(cell).toEqual({ text: body, truncated: false })
  })

  it('does not split a surrogate pair when it does truncate', () => {
    const property = { id: 'p', name: 'Body', typ: 'RichText' } as LibraryProperty
    const cell = propertyCellText(property, { string: '\u{1F600}\u{1F600}\u{1F600}' }, 2)
    expect(cell?.truncated).toBe(true)
    expect(cell?.text).toBe('\u{1F600}\u{1F600}')
    expect(cell?.text).not.toContain('\uFFFD')
  })
})
