import { describe, expect, it } from 'vitest'
import { RecordSaveQueue } from './recordSaveQueue'

function deferred() {
  let resolve!: (value: boolean) => void
  const promise = new Promise<boolean>((settle) => { resolve = settle })
  return { promise, resolve }
}

const flush = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('RecordSaveQueue', () => {
  it('sends saves one at a time, in order', async () => {
    const queue = new RecordSaveQueue()
    const sent: string[] = []
    const first = deferred()
    const a = queue.push(() => { sent.push('a'); return first.promise })
    const b = queue.push(() => { sent.push('b'); return Promise.resolve(true) })
    await flush()
    expect(sent).toEqual(['a'])
    first.resolve(false)
    await expect(a).resolves.toBe(false)
    await expect(b).resolves.toBe(true)
    expect(sent).toEqual(['a', 'b'])
  })

  it('starts an urgent save at once, in the same task', async () => {
    const queue = new RecordSaveQueue()
    const sent: string[] = []
    void queue.push(() => { sent.push('a'); return new Promise<boolean>(() => {}) })
    await flush()
    void queue.push(() => { sent.push('b'); return Promise.resolve(true) })
    void queue.push(() => { sent.push('urgent'); return Promise.resolve(true) }, { urgent: true })
    // No await: a page being torn down may not run another task.
    expect(sent).toEqual(['a', 'urgent'])
  })

  it('answers the saves waiting behind it with the urgent one', async () => {
    const queue = new RecordSaveQueue()
    const sent: string[] = []
    const first = deferred()
    const urgentSave = deferred()
    const a = queue.push(() => { sent.push('a'); return first.promise })
    await flush()
    const b = queue.push(() => { sent.push('b'); return Promise.resolve(true) })
    const urgent = queue.push(() => { sent.push('urgent'); return urgentSave.promise }, { urgent: true })
    const after = queue.push(() => { sent.push('after'); return Promise.resolve(true) })

    first.resolve(true)
    await expect(a).resolves.toBe(true)
    await flush()
    // Not sent, and nothing after the urgent save goes before it settles.
    expect(sent).toEqual(['a', 'urgent'])

    urgentSave.resolve(true)
    await expect(urgent).resolves.toBe(true)
    await expect(b).resolves.toBe(true)
    await expect(after).resolves.toBe(true)
    // It overtook 'a', so it is written once more after it, then 'after'.
    expect(sent).toEqual(['a', 'urgent', 'urgent', 'after'])
  })

  it('sends an urgent save that overtook one under way again once both settle', async () => {
    const queue = new RecordSaveQueue()
    const sent: string[] = []
    const older = deferred()
    void queue.push(() => { sent.push('older'); return older.promise })
    await flush()
    let settled = false
    const urgent = queue.push(() => { sent.push('newest'); return Promise.resolve(true) }, { urgent: true })
    void urgent.then(() => { settled = true })
    // Sent at once, but the older one may still be applied after it on the
    // server: not saved yet.
    expect(sent).toEqual(['older', 'newest'])
    await flush()
    expect(settled).toBe(false)

    older.resolve(true)
    await expect(urgent).resolves.toBe(true)
    expect(sent).toEqual(['older', 'newest', 'newest'])
  })

  it('does not send an urgent save twice when nothing was under way', async () => {
    const queue = new RecordSaveQueue()
    const sent: string[] = []
    await queue.push(() => { sent.push('earlier'); return Promise.resolve(true) })
    await queue.push(() => { sent.push('urgent'); return Promise.resolve(true) }, { urgent: true })
    await queue.push(() => { sent.push('later'); return Promise.resolve(true) })
    expect(sent).toEqual(['earlier', 'urgent', 'later'])
  })

  it('reports a superseded save as failed when the urgent one fails', async () => {
    const queue = new RecordSaveQueue()
    const first = deferred()
    void queue.push(() => first.promise)
    await flush()
    const b = queue.push(() => Promise.resolve(true))
    void queue.push(() => Promise.resolve(false), { urgent: true })
    first.resolve(true)
    await expect(b).resolves.toBe(false)
  })
})
