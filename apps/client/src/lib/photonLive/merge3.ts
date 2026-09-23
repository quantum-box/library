/**
 * A three-way merge of sequences, one element per block.
 *
 * Used when edits were typed before a room answered and the room moved on
 * meanwhile: `ours` and `theirs` both descend from `base`, and neither may
 * be dropped. Regions only one side changed take that side. Regions both
 * sides changed differently keep both -- theirs, then ours -- because a
 * duplicated paragraph is visible and fixable, and a lost one is neither.
 *
 * Elements are compared by `key`, which should ignore identity that differs
 * between copies of the same content (generated block ids).
 */
export function merge3<T>(
  base: readonly T[],
  ours: readonly T[],
  theirs: readonly T[],
  key: (item: T) => string,
): T[] {
  const baseKeys = base.map(key)
  const oursKeys = ours.map(key)
  const theirsKeys = theirs.map(key)
  // For each base index, where it survives in ours / theirs (or -1).
  const inOurs = matchIndexes(baseKeys, oursKeys)
  const inTheirs = matchIndexes(baseKeys, theirsKeys)

  const result: T[] = []
  let b = 0
  let o = 0
  let t = 0
  while (b <= base.length) {
    // The next base element both sides kept, in order: a stable anchor.
    let anchor = b
    while (
      anchor < base.length &&
      !(inOurs[anchor] >= o && inTheirs[anchor] >= t)
    ) anchor += 1
    const oEnd = anchor < base.length ? inOurs[anchor] : ours.length
    const tEnd = anchor < base.length ? inTheirs[anchor] : theirs.length

    const baseChunk = baseKeys.slice(b, anchor)
    const oursChunk = oursKeys.slice(o, oEnd)
    const theirsChunk = theirsKeys.slice(t, tEnd)
    if (sameKeys(oursChunk, baseChunk)) {
      result.push(...theirs.slice(t, tEnd))
    } else if (sameKeys(theirsChunk, baseChunk) || sameKeys(oursChunk, theirsChunk)) {
      result.push(...ours.slice(o, oEnd))
    } else {
      result.push(...theirs.slice(t, tEnd), ...ours.slice(o, oEnd))
    }

    if (anchor >= base.length) break
    // The anchor itself: both kept it unchanged, so either copy will do.
    result.push(theirs[tEnd])
    b = anchor + 1
    o = oEnd + 1
    t = tEnd + 1
  }
  return result
}

/** A longest-common-subsequence alignment of `from` onto `to`. */
function matchIndexes(from: readonly string[], to: readonly string[]): number[] {
  const n = from.length
  const m = to.length
  // lengths[i][j]: LCS length of from[i..] and to[j..]
  const lengths: Uint32Array[] = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1))
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      lengths[i][j] = from[i] === to[j]
        ? lengths[i + 1][j + 1] + 1
        : Math.max(lengths[i + 1][j], lengths[i][j + 1])
    }
  }
  const matches = new Array<number>(n).fill(-1)
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (from[i] === to[j]) {
      matches[i] = j
      i += 1
      j += 1
    } else if (lengths[i + 1][j] >= lengths[i][j + 1]) {
      i += 1
    } else {
      j += 1
    }
  }
  return matches
}

function sameKeys(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index])
}
