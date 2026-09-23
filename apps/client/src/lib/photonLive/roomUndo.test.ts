import { describe, expect, it } from 'vitest'
import * as Y from 'yjs'
import { carryIntoRoom, roomUndoManager } from './roomUndo'

function text(fragment: Y.XmlFragment): string {
  return fragment.toArray().map((node) => (node as Y.XmlText).toString()).join('')
}

describe('room undo', () => {
  it('keeps typing carried into a room undoable', () => {
    const doc = new Y.Doc()
    const fragment = doc.getXmlFragment('prosemirror')
    fragment.insert(0, [new Y.XmlText('Base')])
    const undoManager = roomUndoManager(fragment)

    carryIntoRoom(doc, () => {
      ;(fragment.get(0) as Y.XmlText).insert(4, ' typed before joining')
    })
    expect(text(fragment)).toBe('Base typed before joining')

    undoManager.undo()
    expect(text(fragment)).toBe('Base')
  })

  it('does not track what peers wrote', () => {
    const doc = new Y.Doc()
    const fragment = doc.getXmlFragment('prosemirror')
    const undoManager = roomUndoManager(fragment)
    const peer = new Y.Doc()
    peer.getXmlFragment('prosemirror').insert(0, [new Y.XmlText('from a peer')])
    Y.applyUpdate(doc, Y.encodeStateAsUpdate(peer), 'remote')
    expect(undoManager.canUndo()).toBe(false)
  })
})
