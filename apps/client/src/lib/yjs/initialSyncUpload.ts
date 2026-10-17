import * as Y from 'yjs'

/**
 * The largest update the sync relay keeps. Mirrors `MAX_UPDATE` in
 * `workers/sync/src/model.rs`: anything longer is neither logged nor relayed,
 * silently.
 */
export const SYNC_MAX_UPDATE_BYTES = 128 * 1024 - 4096

export interface InitialSyncUpload {
  /** Updates to send, in order. Each is at most `maxBytes` long. */
  updates: Uint8Array[]
  /** Clients whose missing part alone is longer than `maxBytes`; not sent. */
  skippedClients: number[]
  /**
   * Nothing was sent, so deletions this document has that the room lacks (if
   * any) stay local. Every update carries the whole delete set and a delete
   * set cannot be split, so a delete set longer than `maxBytes` ends here.
   */
  deletionsSkipped: boolean
}

/**
 * What `local` holds that `room` does not, cut into updates the relay keeps.
 *
 * On join the client used to send its whole document, twice. The relay drops
 * anything over `SYNC_MAX_UPDATE_BYTES`, and a document that had lived in the
 * old shared room was ~1 MB, so every join uploaded ~2 MB that was thrown
 * away, and the views and canvases in it never reached the user's room.
 *
 * `room` is the room's state as this client received it. Nothing is sent when
 * the room already has everything, deletions included. A diff too long for one
 * update is split per client; a client whose part alone is too long is
 * reported in `skippedClients` instead of being sent and dropped.
 */
export function missingUpdates(
  local: Y.Doc,
  room: Y.Doc,
  maxBytes = SYNC_MAX_UPDATE_BYTES,
): InitialSyncUpload {
  if (Y.equalSnapshots(Y.snapshot(local), Y.snapshot(room))) {
    return { updates: [], skippedClients: [], deletionsSkipped: false }
  }

  const roomVector = Y.encodeStateVector(room)
  const whole = Y.encodeStateAsUpdate(local, roomVector)
  if (whole.byteLength <= maxBytes) {
    return { updates: [whole], skippedClients: [], deletionsSkipped: false }
  }

  const localClocks = Y.decodeStateVector(Y.encodeStateVector(local))
  const roomClocks = Y.decodeStateVector(roomVector)
  const updates: Uint8Array[] = []
  const skippedClients: number[] = []
  for (const [client, clock] of localClocks) {
    const known = roomClocks.get(client) ?? 0
    if (clock <= known) continue
    // Every other client counts as already sent, so the update carries only
    // this client's missing items (and the document's delete set).
    const target = new Map(localClocks)
    target.set(client, known)
    const part = Y.encodeStateAsUpdate(local, Y.encodeStateVector(target))
    if (part.byteLength <= maxBytes) {
      updates.push(part)
    } else {
      skippedClients.push(client)
    }
  }
  // Each update carries the whole delete set, so deletions reach the room
  // with any update sent. With none sent, they do not -- including when the
  // difference is deletions only, which advance no client's clock.
  return { updates, skippedClients, deletionsSkipped: updates.length === 0 }
}
