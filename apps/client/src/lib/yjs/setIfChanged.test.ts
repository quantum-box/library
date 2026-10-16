import { describe, expect, it } from 'vitest'
import * as Y from 'yjs'
import { setIfChanged } from './setIfChanged'

function countUpdates(doc: Y.Doc, write: () => void): number {
  let updates = 0
  const onUpdate = () => {
    updates += 1
  }
  doc.on('update', onUpdate)
  doc.transact(write)
  doc.off('update', onUpdate)
  return updates
}

describe('setIfChanged', () => {
  it('records nothing when the value is already stored', () => {
    const doc = new Y.Doc()
    const ymap = doc.getMap<string>('m')
    ymap.set('title', 'same')

    expect(countUpdates(doc, () => setIfChanged(ymap, 'title', 'same'))).toBe(0)
    expect(ymap.get('title')).toBe('same')
  })

  it('writes a changed value', () => {
    const doc = new Y.Doc()
    const ymap = doc.getMap<string>('m')
    ymap.set('title', 'before')

    expect(countUpdates(doc, () => setIfChanged(ymap, 'title', 'after'))).toBe(1)
    expect(ymap.get('title')).toBe('after')
  })

  it('writes an empty string to a key that is missing', () => {
    const doc = new Y.Doc()
    const ymap = doc.getMap<string>('m')

    expect(countUpdates(doc, () => setIfChanged(ymap, 'assignee', ''))).toBe(1)
    expect(ymap.has('assignee')).toBe(true)
  })
})
