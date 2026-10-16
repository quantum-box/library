import type * as Y from 'yjs'

/**
 * Set `key` on `ymap` only when the stored value differs.
 *
 * `Y.Map.set` records a new item even when the value is unchanged, and the
 * item it replaces stays in the document's history. The shared workspace room
 * keeps that history, so rewriting every field of every record and view on
 * each hydration grew the room by one full copy per rewrite (about 10 MB of
 * repeated identical values), and every client downloaded all of it on join.
 */
export function setIfChanged(ymap: Y.Map<string>, key: string, value: string): void {
  // A missing key reads as `undefined`, which never equals a string.
  if (ymap.get(key) === value) return
  ymap.set(key, value)
}
