import * as Y from 'yjs'
import { defaultDeleteFilter, defaultProtectedNodes, ySyncPluginKey } from 'y-prosemirror'

/** The origin of edits an editor carries into a room as it joins it. */
const carriedEdits = Symbol('carried edits')

/**
 * An undo manager for a room's fragment, as y-prosemirror's own undo plugin
 * makes one, that also tracks the edits carried in on joining.
 *
 * Switching an editor to a room replaces its undo history. What it typed
 * before the room answered is written into the room first; made under this
 * manager, that typing stays undoable in the room.
 */
export function roomUndoManager(fragment: Y.XmlFragment): Y.UndoManager {
  return new Y.UndoManager(fragment, {
    trackedOrigins: new Set<unknown>([ySyncPluginKey, carriedEdits]),
    deleteFilter: (item) => defaultDeleteFilter(item, defaultProtectedNodes),
    captureTransaction: (tr) => tr.meta.get('addToHistory') !== false,
  })
}

/** Write what an editor carries into a room, as edits `roomUndoManager` tracks. */
export function carryIntoRoom(doc: Y.Doc, change: () => void): void {
  doc.transact(change, carriedEdits)
}
