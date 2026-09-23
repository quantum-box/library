/**
 * The saves of one record, sent one at a time and in order.
 *
 * A save that must go out while the page unloads cannot wait its turn: a
 * browser tearing the page down cancels whatever is still pending, and a
 * request started from a callback that never runs is never sent. An urgent
 * save therefore starts at once. The saves queued before it that have not
 * started yet are answered by it instead of sent -- which is only right
 * when it carries everything they did, as a save of the whole record as the
 * page last knew it does. A save already under way is left to finish.
 */
export class RecordSaveQueue {
  private tail: Promise<unknown> = Promise.resolve()
  private sequence = 0
  private supersededBefore = 0
  private superseding: Promise<boolean> = Promise.resolve(true)

  /**
   * Queue `send`, which settles `true` once its save is durable and `false`
   * if it failed; it should not reject. Resolves with its result, or with
   * the result of the urgent save that took its place.
   */
  push(send: () => Promise<boolean>, { urgent = false }: { urgent?: boolean } = {}): Promise<boolean> {
    const sequence = ++this.sequence
    if (urgent) {
      const saving = send()
      this.supersededBefore = sequence
      this.superseding = saving
      // Whatever comes after still waits for the saves already under way.
      this.tail = Promise.allSettled([this.tail, saving])
      return saving
    }
    const saving = this.tail.catch(() => undefined).then(() =>
      sequence < this.supersededBefore ? this.superseding : send(),
    )
    this.tail = saving
    return saving
  }
}
