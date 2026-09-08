import { beforeEach, describe, expect, it } from 'vitest'
import type { LibraryProperty } from '../recordsApi'
import {
  emptyTableLayout,
  loadTableLayout,
  orderedProperties,
  reorderProperties,
  resetTableLayout,
  saveTableLayout,
  setColumnWidth,
  tableLayoutStorageKey,
  togglePropertyHidden,
  visibleProperties,
} from './tableLayout'

const properties: LibraryProperty[] = [
  { id: 'a', name: 'Alpha', typ: 'String' },
  { id: 'b', name: 'Beta', typ: 'Select' },
  { id: 'c', name: 'Gamma', typ: 'Boolean' },
]

describe('tableLayout', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('keeps the repository order until the reader changes it', () => {
    expect(orderedProperties(properties, emptyTableLayout).map((p) => p.id)).toEqual([
      'a',
      'b',
      'c',
    ])
  })

  it('puts a Property the arrangement has never seen at the end', () => {
    const layout = { ...emptyTableLayout, order: ['c', 'a'] }
    expect(orderedProperties(properties, layout).map((p) => p.id)).toEqual(['c', 'a', 'b'])
  })

  it('drops an id whose Property is gone', () => {
    const layout = { ...emptyTableLayout, order: ['deleted', 'b', 'a', 'c'] }
    expect(orderedProperties(properties, layout).map((p) => p.id)).toEqual(['b', 'a', 'c'])
  })

  it('moves a column to where the one it was dropped on sits', () => {
    const moved = reorderProperties(properties, emptyTableLayout, 'c', 'a')
    expect(orderedProperties(properties, moved).map((p) => p.id)).toEqual(['c', 'a', 'b'])
  })

  it('leaves the arrangement alone when the drop lands on the column itself', () => {
    expect(reorderProperties(properties, emptyTableLayout, 'a', 'a')).toBe(emptyTableLayout)
  })

  it('hides and shows one column at a time', () => {
    const hidden = togglePropertyHidden(emptyTableLayout, 'b')
    expect(visibleProperties(properties, hidden).map((p) => p.id)).toEqual(['a', 'c'])
    expect(visibleProperties(properties, togglePropertyHidden(hidden, 'b')).map((p) => p.id)).toEqual(
      ['a', 'b', 'c'],
    )
  })

  it('rounds a dragged width and refuses a nonsensical one', () => {
    expect(setColumnWidth(emptyTableLayout, 'a', 180.4).widths).toEqual({ a: 180 })
    expect(setColumnWidth(emptyTableLayout, 'a', -20)).toBe(emptyTableLayout)
  })

  it('round-trips through storage, per repository', () => {
    const layout = { order: ['c'], hidden: ['b'], widths: { a: 200 } }
    saveTableLayout('quantum-box', 'docs', layout)
    expect(loadTableLayout('quantum-box', 'docs')).toEqual(layout)
    // Another repository's table is untouched by this one's arrangement.
    expect(loadTableLayout('quantum-box', 'other')).toEqual(emptyTableLayout)
  })

  it('reads a repository under either spelling of its name', () => {
    expect(tableLayoutStorageKey('Quantum-Box', 'Docs')).toBe(
      tableLayoutStorageKey('quantum-box', 'docs'),
    )
  })

  it('ignores a stored entry that is not an arrangement', () => {
    window.localStorage.setItem(tableLayoutStorageKey('quantum-box', 'docs'), '{"order":42}')
    expect(loadTableLayout('quantum-box', 'docs')).toEqual(emptyTableLayout)
  })

  it('resets to the repository order', () => {
    const layout = { order: ['c'], hidden: ['b'], widths: { a: 200 } }
    expect(resetTableLayout(layout)).toEqual(emptyTableLayout)
    // Nothing to reset leaves the same object, so the table does not re-render.
    expect(resetTableLayout(emptyTableLayout)).toBe(emptyTableLayout)
  })
})
