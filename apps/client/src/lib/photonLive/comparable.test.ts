import { describe, expect, it } from 'vitest'
import { comparableBody } from './comparable'

const paragraph = (id: string, text?: string) => ({
  id,
  type: 'paragraph',
  props: {},
  content: text ? [{ type: 'text', text, styles: {} }] : [],
  children: [],
})

describe('comparableBody', () => {
  it('ignores generated block ids', () => {
    expect(comparableBody(JSON.stringify([paragraph('a', 'Hello')]), 'richText'))
      .toBe(comparableBody(JSON.stringify([paragraph('b', 'Hello')]), 'richText'))
  })

  it('ignores the one trailing placeholder a copy may or may not have', () => {
    expect(comparableBody(JSON.stringify([paragraph('a', 'Hello'), paragraph('p')]), 'richText'))
      .toBe(comparableBody(JSON.stringify([paragraph('b', 'Hello')]), 'richText'))
  })

  it('keeps blank paragraphs someone typed beyond the placeholder', () => {
    const typed = [paragraph('a', 'Hello'), paragraph('x'), paragraph('p')]
    const plain = [paragraph('b', 'Hello'), paragraph('q')]
    expect(comparableBody(JSON.stringify(typed), 'richText'))
      .not.toBe(comparableBody(JSON.stringify(plain), 'richText'))
  })

  it('compares markdown as text, without trailing whitespace', () => {
    expect(comparableBody('# Title\n\nBody\n', 'markdown')).toBe(comparableBody('# Title\n\nBody', 'markdown'))
  })
})
