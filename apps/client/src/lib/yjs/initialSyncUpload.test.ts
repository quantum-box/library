import { describe, expect, it } from 'vitest'
import * as Y from 'yjs'
import { missingUpdates } from './initialSyncUpload'

function syncedPair(): { local: Y.Doc; room: Y.Doc } {
  const local = new Y.Doc()
  local.getArray<string>('records').push(['shared'])
  const room = new Y.Doc()
  Y.applyUpdate(room, Y.encodeStateAsUpdate(local))
  return { local, room }
}

function applyAll(doc: Y.Doc, updates: Uint8Array[]) {
  for (const update of updates) Y.applyUpdate(doc, update)
}

describe('missingUpdates', () => {
  it('sends nothing when the room already has the whole document', () => {
    const { local, room } = syncedPair()

    expect(missingUpdates(local, room)).toEqual({
      updates: [],
      skippedClients: [],
      deletionsSkipped: false,
    })
  })

  it('sends only what the room lacks, in one update when it fits', () => {
    const { local, room } = syncedPair()
    local.getArray<string>('records').push(['offline edit'])

    const { updates } = missingUpdates(local, room)
    applyAll(room, updates)

    expect(updates).toHaveLength(1)
    expect(updates[0].byteLength).toBeLessThan(Y.encodeStateAsUpdate(local).byteLength)
    expect(room.getArray('records').toArray()).toEqual(['shared', 'offline edit'])
  })

  it('sends a deletion made offline even though no client moved forward', () => {
    const { local, room } = syncedPair()
    local.getArray<string>('records').delete(0, 1)

    applyAll(room, missingUpdates(local, room).updates)

    expect(room.getArray('records').toArray()).toEqual([])
  })

  it('splits a diff too long for one update per client, each within the limit', () => {
    const room = new Y.Doc()
    const local = new Y.Doc()
    for (let i = 0; i < 4; i++) {
      const author = new Y.Doc()
      author.getArray<string>('records').push([`client ${i} `.padEnd(400, 'x')])
      Y.applyUpdate(local, Y.encodeStateAsUpdate(author))
    }

    const limit = 700
    const { updates, skippedClients } = missingUpdates(local, room, limit)
    applyAll(room, updates)

    expect(updates.length).toBeGreaterThan(1)
    expect(updates.every((update) => update.byteLength <= limit)).toBe(true)
    expect(skippedClients).toEqual([])
    expect(Y.equalSnapshots(Y.snapshot(room), Y.snapshot(local))).toBe(true)
  })

  it('reports a client whose part alone is too long instead of sending it', () => {
    const room = new Y.Doc()
    const local = new Y.Doc()
    local.getArray<string>('records').push(['x'.repeat(2_000)])

    const { updates, skippedClients } = missingUpdates(local, room, 500)

    expect(updates).toEqual([])
    expect(skippedClients).toEqual([local.clientID])
  })

  it('reports deletions too large for one update instead of dropping them silently', () => {
    const local = new Y.Doc()
    const records = local.getArray<string>('records')
    // Many separate items, so their deletion is a long, fragmented delete set.
    for (let i = 0; i < 400; i++) records.push([`row ${i}`])
    const room = new Y.Doc()
    Y.applyUpdate(room, Y.encodeStateAsUpdate(local))
    for (let i = records.length - 1; i >= 0; i -= 2) records.delete(i, 1)

    const result = missingUpdates(local, room, 64)

    expect(result.updates).toEqual([])
    expect(result.skippedClients).toEqual([])
    expect(result.deletionsSkipped).toBe(true)
  })
})
