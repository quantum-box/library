import { describe, expect, it } from 'vitest'
import { merge3 } from './merge3'

const merge = (base: string, ours: string, theirs: string) =>
  merge3([...base], [...ours], [...theirs], (value) => value).join('')

describe('merge3', () => {
  it('takes the only side that changed', () => {
    expect(merge('abc', 'abc', 'abXc')).toBe('abXc')
    expect(merge('abc', 'aYbc', 'abc')).toBe('aYbc')
    expect(merge('abc', 'abc', 'abc')).toBe('abc')
  })

  it('keeps changes each side made to different places', () => {
    expect(merge('abcde', 'aXbcde', 'abcdYe')).toBe('aXbcdYe')
    expect(merge('abcde', 'acde', 'abcdeZ')).toBe('acdeZ')
    expect(merge('', 'ours', '')).toBe('ours')
    expect(merge('ab', 'abO', 'abT')).toBe('abTO')
  })

  it('keeps both when the same place changed differently', () => {
    expect(merge('abc', 'aXc', 'aYc')).toBe('aYXc')
    // The same change on both sides is taken once.
    expect(merge('abc', 'aXc', 'aXc')).toBe('aXc')
  })

  it('compares by key, keeping the actual element', () => {
    const base = [{ id: 'b1', text: 'one' }, { id: 'b2', text: 'two' }]
    const ours = [{ id: 'o1', text: 'one' }, { id: 'o2', text: 'two' }, { id: 'o3', text: 'mine' }]
    const theirs = [{ id: 't0', text: 'zero' }, { id: 't1', text: 'one' }, { id: 't2', text: 'two' }]
    expect(merge3(base, ours, theirs, (block) => block.text).map((block) => block.id))
      .toEqual(['t0', 't1', 't2', 'o3'])
  })
})
