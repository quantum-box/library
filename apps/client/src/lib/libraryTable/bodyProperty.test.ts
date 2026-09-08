import { describe, expect, it } from 'vitest'
import type { LibraryProperty } from '../recordsApi'
import {
  bodyPropertyFormat,
  bodyPropertyValue,
  getBodyProperty,
  isArtifactHtml,
} from './bodyProperty'

function property(id: string, name: string, typ: string): LibraryProperty {
  return { id, name, typ }
}

describe('getBodyProperty', () => {
  it('prefers rich text over the legacy body types', () => {
    const properties = [
      property('p1', 'Content', 'Html'),
      property('p2', 'Notes', 'RichText'),
      property('p3', 'Body', 'Markdown'),
    ]

    expect(getBodyProperty(properties)?.id).toBe('p2')
  })

  it('does not pick a text property just because it is named "content"', () => {
    const properties = [
      property('p1', 'Content', 'String'),
      property('p2', 'Rendered', 'Html'),
    ]

    // A String property named "content" used to win outright, which is how
    // Markdown from the body editor got written into Html properties.
    expect(getBodyProperty(properties)?.id).toBe('p2')
  })

  it('has no body when the repository holds no body type', () => {
    const properties = [
      property('p1', 'Body', 'String'),
      property('p2', 'Description', 'Select'),
    ]

    expect(getBodyProperty(properties)).toBeNull()
  })
})

describe('bodyPropertyFormat', () => {
  it('maps each body type to the dialect it stores', () => {
    expect(bodyPropertyFormat(property('p1', 'Body', 'RichText'))).toBe('richText')
    expect(bodyPropertyFormat(property('p2', 'Body', 'Html'))).toBe('html')
    expect(bodyPropertyFormat(property('p3', 'Body', 'Markdown'))).toBe('markdown')
  })
})

describe('bodyPropertyValue', () => {
  it('tags the committed value with the key its type is read from', () => {
    expect(bodyPropertyValue(property('p1', 'Body', 'RichText'), '[]')).toEqual({ richText: '[]' })
    expect(bodyPropertyValue(property('p2', 'Body', 'Html'), '<p>a</p>')).toEqual({
      html: '<p>a</p>',
    })
    expect(bodyPropertyValue(property('p3', 'Body', 'Markdown'), '# a')).toEqual({
      markdown: '# a',
    })
  })
})

describe('isArtifactHtml', () => {
  /**
   * The page reads this to decide its own layout, so the answer has to be
   * about the value, not the property: an Html Property can still hold the
   * Markdown this editor used to write into one.
   */
  it('is true for markup and for a value not written yet', () => {
    expect(isArtifactHtml('<!doctype html><h1>x</h1>')).toBe(true)
    expect(isArtifactHtml('\n  <section>x</section>')).toBe(true)
    expect(isArtifactHtml('')).toBe(true)
    expect(isArtifactHtml('   ')).toBe(true)
  })

  it('is false for the Markdown dialect stored in older Html Properties', () => {
    expect(isArtifactHtml('# Heading\n\ntext')).toBe(false)
    expect(isArtifactHtml('text with <em>markup</em> inside')).toBe(false)
  })
})
