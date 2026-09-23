/**
 * The saves of one record, sent one at a time and in order.
 *
 * A save that must go out while the page is hidden or unloading cannot wait
 * its turn: a browser suspending, discarding or tearing the page down stops
 * whatever is still pending, and a request started from a callback that
 * never runs is never sent. An urgent save therefore starts at once. The
 * saves queued before it that have not started yet are answered by it
 * instead of sent -- which is only right when it carries everything they
 * did, as a save of the whole record as the page last knew it does.
 *
 * A save already under way is left to finish, and nothing orders the two on
 * the server: the older one may be applied last. So when an urgent save
 * overtook one, it is sent once more after both have settled, and it counts
 * as saved only with that write. A page that lives on -- as a merely hidden
 * one usually does -- ends with the newest record written last, and nothing
 * waiting on the save (a Live room about to be opened) goes ahead before.
 */
export class RecordSaveQueue {
  private tail: Promise<unknown> = Promise.resolve()
  private sequence = 0
  private supersededBefore = 0
  private superseding: Promise<boolean> = Promise.resolve(true)
  private running = 0

  /**
   * Queue `send`, which settles `true` once its save is durable and `false`
   * if it failed; it should not reject. Resolves with its result -- for an
   * urgent save that overtook another, the result of sending it again -- or
   * with the result of the first attempt of the urgent save that took its
   * place.
   */
  push(send: () => Promise<boolean>, { urgent = false }: { urgent?: boolean } = {}): Promise<boolean> {
    const sequence = ++this.sequence
    if (urgent) {
      const overtook = this.running > 0
      const first = this.run(send)
      // Whatever comes after still waits for the saves already under way.
      const settled = Promise.allSettled([this.tail, first])
      const saving = overtook ? settled.then(() => this.run(send)) : first
      this.supersededBefore = sequence
      // Its first attempt: the tail the re-send waits for includes them.
      this.superseding = first
      this.tail = overtook ? saving : settled
      return saving
    }
    const saving = this.tail.catch(() => undefined).then(() =>
      sequence < this.supersededBefore ? this.superseding : this.run(send),
    )
    this.tail = saving
    return saving
  }

  private run(send: () => Promise<boolean>): Promise<boolean> {
    this.running += 1
    return send().finally(() => {
      this.running -= 1
    })
  }
}
