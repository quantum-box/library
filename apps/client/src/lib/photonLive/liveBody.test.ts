import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as Y from 'yjs'
import { Awareness } from 'y-protocols/awareness'
import { LiveBodySession, type LiveBodyEditorPort } from './liveBody'
import { merge3 } from './merge3'
import { PhotonLiveError, type PhotonLiveProvider, type PhotonLiveState } from './types'

/**
 * A fragment's "body" is a Y.Text in the same document, so Yjs merges,
 * copies and snapshots of a room behave the way they do for real bodies.
 */
function bodyOf(fragment: Y.XmlFragment): string {
  return fragment.doc!.getText('body').toString()
}

function setBody(fragment: Y.XmlFragment, body: string): void {
  const text = fragment.doc!.getText('body')
  fragment.doc!.transact(() => {
    text.delete(0, text.length)
    text.insert(0, body)
  })
}

/** A room stand-in with the provider's reporting behaviour. */
class FakeRoom implements PhotonLiveProvider {
  readonly doc = new Y.Doc()
  readonly fragment = this.doc.getXmlFragment('prosemirror')
  readonly awareness = new Awareness(this.doc)
  readonly user = { name: 'Aoi', color: '#000' }
  readonly session = null
  readonly queued: string[] = []
  roomGeneration: string | null = null
  detached = false
  destroyed = false
  flushed = 0
  private unsent: string | null = null
  private state: PhotonLiveState = {
    status: 'authorizing',
    saveStatus: 'idle',
    error: null,
    initialized: false,
    version: 0,
    recordVersion: '1',
    hasUnackedChanges: false,
    canEdit: false,
  }
  private readonly listeners = new Set<(state: PhotonLiveState) => void>()

  getState(): PhotonLiveState {
    return this.state
  }
  unsentBody(): string | null {
    return this.unsent
  }
  subscribe(listener: (state: PhotonLiveState) => void): () => void {
    this.listeners.add(listener)
    listener(this.state)
    return () => this.listeners.delete(listener)
  }
  subscribeSave(listener: (state: PhotonLiveState) => void): () => void {
    return this.subscribe(listener)
  }
  queueCheckpoint(body: string): void {
    this.queued.push(body)
    this.unsent = body
    // As the real provider: a queued body is unsaved until its ACK.
    this.set({ saveStatus: 'saving', hasUnackedChanges: true })
  }
  flushCheckpoint(): void {
    this.flushed += 1
  }
  detach(): void {
    this.detached = true
    // As the real provider: detaching reports a disconnect synchronously.
    this.set({ status: 'disconnected' })
  }
  destroy(): void {
    this.destroyed = true
  }
  set(next: Partial<PhotonLiveState>): void {
    this.state = { ...this.state, ...next }
    this.listeners.forEach((listener) => listener(this.state))
  }
  ready(generation: string | null = null): void {
    this.roomGeneration = generation
    this.set({ status: 'connected', initialized: true, canEdit: true })
  }
  ack(): void {
    this.unsent = null
    this.set({ saveStatus: 'saved', hasUnackedChanges: false })
  }
}

function harness(initialBody = 'Base') {
  const draftDoc = new Y.Doc()
  const draft = draftDoc.getXmlFragment('prosemirror')
  setBody(draft, initialBody)
  let bound: Y.XmlFragment = draft
  let composing = false
  let onBind: ((provider: PhotonLiveProvider) => void) | null = null
  const rooms: FakeRoom[] = []
  const rest: string[] = []
  let restResult: Promise<boolean> | boolean = true
  let roomSeed: (room: FakeRoom) => void = (room) => setBody(room.fragment, initialBody)

  const words = (body: string) => body.split(' ')
  const port: LiveBodyEditorPort = {
    serializeFragment: bodyOf,
    comparable: (body) => body.trim(),
    writeInto: (target, from) => setBody(target, bodyOf(from)),
    mergeInto: (target, from, base) => {
      setBody(target, merge3(words(base), words(bodyOf(from)), words(bodyOf(target)), (w) => w).join(' '))
    },
    bind: (provider) => {
      bound = provider.fragment
      onBind?.(provider)
    },
    composing: () => composing,
  }
  const session = new LiveBodySession({
    draft,
    createProvider: () => {
      const room = new FakeRoom()
      roomSeed(room)
      rooms.push(room)
      return room
    },
    commitRest: (body) => {
      rest.push(body)
      return restResult
    },
    timing: { joinGraceMs: 10_000, stallMs: 15_000, retryBaseMs: 1_000, retryMaxMs: 8_000, drainMs: 5_000 },
    isOnline: () => true,
  })
  session.setEditor(port)

  return {
    session,
    rooms,
    rest,
    draftDoc,
    get onScreen() {
      return bodyOf(bound)
    },
    get boundTo() {
      return bound
    },
    /** A keystroke: what is on screen changes, then the debounce commits it. */
    type(body: string) {
      setBody(bound, body)
      session.commit(body)
    },
    roomBody(room: FakeRoom, body: string) {
      setBody(room.fragment, body)
    },
    /** How the next room's document starts. */
    seedRooms(seed: (room: FakeRoom) => void) {
      roomSeed = seed
    },
    setComposing(value: boolean) {
      composing = value
    },
    setRestResult(value: Promise<boolean> | boolean) {
      restResult = value
    },
    /** What the room reports while the editor is being bound to it. */
    whileBinding(callback: (provider: PhotonLiveProvider) => void) {
      onBind = callback
    },
  }
}

describe('LiveBodySession', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('joins a room that answers before anything is typed, without writing to it', () => {
    const h = harness()
    h.session.start()
    expect(h.rooms).toHaveLength(1)
    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.boundTo).toBe(h.rooms[0].fragment)
    expect(h.draftDoc.isDestroyed).toBe(true)
    // Opening a page is not an edit: nothing is checkpointed back.
    expect(h.rooms[0].queued).toEqual([])
    h.type('Base shared')
    expect(h.rooms[0].queued).toEqual(['Base shared'])
    expect(h.rest).toEqual([])
  })

  it('writes edits typed before the room answered into an unchanged room', () => {
    const h = harness()
    h.session.start()
    h.type('Base typed early')
    expect(h.session.getView()).toMatchObject({ mode: 'joining', holding: true })
    expect(h.rest).toEqual([])

    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.boundTo).toBe(h.rooms[0].fragment)
    expect(h.onScreen).toBe('Base typed early')
    expect(h.rooms[0].queued).toEqual(['Base typed early'])
    // Nothing went through the ordinary body: the canonical body is not
    // changed under everyone else's room.
    expect(h.rest).toEqual([])
  })

  it('adopts a room a peer changed when nothing was typed here', () => {
    const h = harness()
    h.session.start()
    h.roomBody(h.rooms[0], 'Base peer')
    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('Base peer')
    expect(h.rooms[0].queued).toEqual([])
  })

  it('merges edits typed before joining with a room that moved on meanwhile', () => {
    const h = harness('one two')
    h.session.start()
    h.type('zero one two')
    h.roomBody(h.rooms[0], 'one two three')
    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('live')
    // Neither side is lost, and nothing is saved around the room.
    expect(h.onScreen).toBe('zero one two three')
    expect(h.rooms[0].queued).toEqual(['zero one two three'])
    expect(h.rest).toEqual([])
  })

  it('is not re-entered by what the room reports while the editor binds to it', () => {
    const h = harness()
    h.session.start()
    h.type('Base typed early')
    let rebinds = 0
    h.whileBinding((provider) => {
      rebinds += 1
      // The sync plugin writes the editor back into the room as it binds,
      // and the room reports that edit before bind returns.
      ;(provider as unknown as FakeRoom).set({ hasUnackedChanges: true })
    })
    h.rooms[0].ready()
    expect(rebinds).toBe(1)
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('Base typed early')
    expect(h.rooms[0].queued).toEqual(['Base typed early'])
  })

  it('is not re-entered by what the room reports while edits are written into it', () => {
    const h = harness()
    h.session.start()
    h.type('Base typed early')
    // Writing into the room is a local Yjs change the room reports at once.
    h.rooms[0].doc.on('update', () => h.rooms[0].set({ hasUnackedChanges: true }))
    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.boundTo).toBe(h.rooms[0].fragment)
    expect(h.onScreen).toBe('Base typed early')
    expect(h.rooms[0].queued).toEqual(['Base typed early'])
  })

  it('does not bind while an IME composition is open', () => {
    const h = harness()
    h.session.start()
    h.setComposing(true)
    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('joining')
    h.setComposing(false)
    h.session.compositionEnded()
    expect(h.session.getView().mode).toBe('live')
  })

  it('saves held edits normally when no room answers in time, then joins on that body', async () => {
    const h = harness()
    h.session.start()
    h.type('Base slow room')
    vi.advanceTimersByTime(10_000)
    expect(h.rest).toEqual(['Base slow room'])
    await vi.advanceTimersByTimeAsync(0)
    // The room authorized before that save cannot hold it; a fresh one is
    // opened once the save has settled.
    expect(h.rooms[0].destroyed).toBe(true)
    h.seedRooms((room) => setBody(room.fragment, 'Base slow room'))
    await vi.advanceTimersByTimeAsync(1_000)
    expect(h.rooms).toHaveLength(2)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.rooms[1].queued).toEqual([])
  })

  it('keeps saving normally after the grace until a room joins', async () => {
    const h = harness()
    h.session.start()
    h.type('Base one')
    vi.advanceTimersByTime(10_000)
    await Promise.resolve()
    h.type('Base two')
    expect(h.rest).toEqual(['Base one', 'Base two'])
  })

  it('treats a retryable connection failure as reconnecting, not as the end of Live', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('timeout', 'timeout', undefined, true),
    })
    expect(h.session.getView().mode).toBe('live')
    expect(h.rooms[0].detached).toBe(false)
    h.rooms[0].set({ status: 'connected', error: null })
    vi.advanceTimersByTime(60_000)
    expect(h.session.getView().mode).toBe('live')
  })

  it('replaces a room that never reconnects and carries unsaved edits into the next one', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base stuck')
    h.rooms[0].set({ status: 'disconnected' })
    vi.advanceTimersByTime(15_000)
    expect(h.rooms[0].detached).toBe(true)
    expect(h.session.getView()).toMatchObject({ mode: 'rejoining', holding: true })

    vi.advanceTimersByTime(1_000)
    expect(h.rooms).toHaveLength(2)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('Base stuck')
    expect(h.rooms[1].queued).toEqual(['Base stuck'])
    expect(h.rooms[0].destroyed).toBe(true)
    expect(h.rest).toEqual([])
  })

  it('merges the documents when a stalled room turns out to be the same room', () => {
    const h = harness('A')
    h.session.start()
    h.rooms[0].ready('generation-1')
    // Relayed to the room but not acknowledged, then the socket drops.
    h.type('AB')
    const inRoom = Y.encodeStateAsUpdate(h.rooms[0].doc)
    h.rooms[0].set({ status: 'disconnected' })
    h.type('ABC')
    vi.advanceTimersByTime(15_000)
    h.seedRooms((room) => Y.applyUpdate(room.doc, inRoom))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready('generation-1')
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('ABC')
    expect(h.rooms[1].queued).toEqual(['ABC'])
    expect(h.rest).toEqual([])
  })

  it('rejoins a replaced room without losing anything when nothing was unsaved', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base saved')
    h.rooms[0].ack()
    // Someone else's ordinary save replaced the canonical body: 4410.
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })
    expect(h.session.getView().mode).toBe('rejoining')

    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('External body')
    expect(h.rest).toEqual([])
    expect(h.rooms[1].queued).toEqual([])
  })

  it('leaves a real conflict on screen until the person saves it', async () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base saved')
    h.rooms[0].ack()
    h.type('Base saved and a draft')
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })

    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('conflict')
    expect(h.onScreen).toBe('Base saved and a draft')
    // The draft is not saved over the external body behind anyone's back.
    expect(h.rest).toEqual([])

    h.type('Base saved and a draft, resolved')
    expect(h.rest).toEqual(['Base saved and a draft, resolved'])
    await vi.advanceTimersByTimeAsync(0)
    expect(h.session.getView().mode).toBe('rejoining')
    h.seedRooms((room) => setBody(room.fragment, 'Base saved and a draft, resolved'))
    await vi.advanceTimersByTimeAsync(2_000)
    h.rooms.at(-1)!.ready()
    expect(h.session.getView().mode).toBe('live')
  })

  it('keeps the newest saved body when an older save settles after it', async () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base saved')
    h.rooms[0].ack()
    h.type('Base saved and a draft')
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })
    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('conflict')

    const saves: Array<(saved: boolean) => void> = []
    h.session.setCommitRest(() => new Promise<boolean>((resolve) => saves.push(resolve)))
    h.type('Base resolved')
    // Typed on and switched away: the newer save may overtake the first.
    setBody(h.boundTo, 'Base resolved, then more')
    h.session.commit('Base resolved, then more', { keepalive: true })
    saves[1](true)
    await vi.advanceTimersByTimeAsync(0)
    saves[0](true)
    await vi.advanceTimersByTimeAsync(0)
    // What is on screen is saved, so the conflict is over.
    expect(h.session.getView().mode).toBe('rejoining')
  })

  it('does not save a debounce that lands after a canonical change over it', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base saved')
    h.rooms[0].ack()
    h.type('Base saved and a draft')
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })
    // The editor's debounce fires after the room has already gone.
    h.type('Base saved and a draft, still typing')
    expect(h.rest).toEqual([])

    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('conflict')
    expect(h.rest).toEqual([])
  })

  it('reports a conflict instead of saving when no room decides one in time', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base draft')
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })
    h.type('Base draft, more')
    vi.advanceTimersByTime(10_000)
    expect(h.session.getView().mode).toBe('conflict')
    expect(h.rest).toEqual([])
  })

  it('treats the failed connection a canonical change reports first as that conflict', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base saved')
    h.rooms[0].ack()
    h.type('Base saved and an offline draft')
    // The provider's order for a rotated room: the connection fails with a
    // conflict error, and only then does the save status follow.
    h.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('canonical body changed', 'conflict', 409),
    })
    h.rooms[0].set({ saveStatus: 'conflict' })
    expect(h.session.getView()).toMatchObject({ mode: 'rejoining', holding: false })

    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('conflict')
    expect(h.rest).toEqual([])
  })

  it('leaves unsaved edits from a stalled room on screen when the next room conflicts', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base saved')
    h.rooms[0].ack()
    h.type('Base saved and unsent')
    h.rooms[0].set({ status: 'disconnected' })
    vi.advanceTimersByTime(15_000)
    expect(h.session.getView()).toMatchObject({ mode: 'rejoining', holding: true })

    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView()).toMatchObject({ mode: 'conflict', holding: false })
    vi.advanceTimersByTime(60_000)
    expect(h.rest).toEqual([])
    expect(h.onScreen).toBe('Base saved and unsent')
  })

  it('leaves the conflict when the draft is undone back to the saved body', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base draft')
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })
    h.seedRooms((room) => setBody(room.fragment, 'External body'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('conflict')

    h.type('Base')
    expect(h.rest).toEqual([])
    expect(h.session.getView().mode).toBe('rejoining')
    vi.advanceTimersByTime(8_000)
    h.rooms.at(-1)!.ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('External body')
  })

  it('saves a body the rooms keep rejecting normally instead of rejoining forever', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready('generation-1')
    h.type('Base far too large')
    h.rooms[0].set({ saveStatus: 'error' })
    expect(h.session.getView().mode).toBe('rejoining')
    h.seedRooms((room) => Y.applyUpdate(room.doc, Y.encodeStateAsUpdate(h.rooms[0].doc)))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready('generation-1')
    expect(h.rooms[1].queued).toEqual(['Base far too large'])
    h.rooms[1].set({ saveStatus: 'error' })

    expect(h.session.getView().mode).toBe('unavailable')
    expect(h.rest).toEqual(['Base far too large'])
    vi.advanceTimersByTime(60_000)
    expect(h.rooms).toHaveLength(2)
  })

  it('gives up on a room that is disabled and saves normally', () => {
    const h = harness()
    h.session.start()
    h.type('Base while disabled')
    h.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('disabled', 'disabled', 404),
    })
    expect(h.session.getView().mode).toBe('unavailable')
    expect(h.rest).toEqual(['Base while disabled'])
    h.type('Base later')
    expect(h.rest).toEqual(['Base while disabled', 'Base later'])
  })

  it('retries a refused room a few times before giving up', () => {
    const h = harness()
    h.session.start()
    for (let attempt = 0; attempt < 2; attempt += 1) {
      h.rooms.at(-1)!.set({
        status: 'failed',
        error: new PhotonLiveError('forbidden', 'unauthorized', 403),
      })
      expect(h.session.getView().mode).toBe('joining')
      vi.advanceTimersByTime(8_000)
    }
    h.rooms.at(-1)!.set({
      status: 'failed',
      error: new PhotonLiveError('forbidden', 'unauthorized', 403),
    })
    expect(h.session.getView().mode).toBe('unavailable')
  })

  it('starts counting refusals again once a room lets the editor in', () => {
    const h = harness()
    h.session.start()
    const refuse = () => h.rooms.at(-1)!.set({
      status: 'failed',
      error: new PhotonLiveError('forbidden', 'unauthorized', 403),
    })
    refuse()
    vi.advanceTimersByTime(1_000)
    refuse()
    vi.advanceTimersByTime(2_000)
    h.rooms[2].ready()
    expect(h.session.getView().mode).toBe('live')

    // The room goes away; the next one is opened without the old backoff.
    refuse()
    expect(h.session.getView().mode).toBe('rejoining')
    vi.advanceTimersByTime(999)
    expect(h.rooms).toHaveLength(3)
    vi.advanceTimersByTime(1)
    expect(h.rooms).toHaveLength(4)
    // One refusal after a successful join is not the third in a row.
    refuse()
    expect(h.session.getView().mode).toBe('rejoining')
  })

  it('flushes held edits through the ordinary body when the page goes away', () => {
    const h = harness()
    h.session.start()
    h.type('Base unmounted early')
    h.session.flush()
    expect(h.rest).toEqual(['Base unmounted early'])
  })

  it('saves held edits in a request that outlives the page when it is hidden', () => {
    const h = harness()
    const calls: Array<{ body: string; keepalive?: boolean }> = []
    h.session.setCommitRest((body, options) => {
      calls.push({ body, keepalive: options?.keepalive })
      return true
    })
    h.session.start()
    h.type('Base typed before switching away')
    // A mobile browser may discard a hidden page without firing pagehide.
    h.session.flush({ reason: 'hidden' })
    expect(calls).toEqual([{ body: 'Base typed before switching away', keepalive: true }])
  })

  it('only flushes a connected room when the page is hidden', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base last edit')
    h.session.flush({ reason: 'hidden' })
    expect(h.rooms[0].flushed).toBe(1)
    // Saving it normally would restart the room on every switch away.
    expect(h.rest).toEqual([])
  })

  it('does not save a reconnecting room normally when the page is only hidden', () => {
    const h = harness()
    const calls: Array<{ body: string; keepalive?: boolean }> = []
    h.session.setCommitRest((body, options) => {
      calls.push({ body, keepalive: options?.keepalive })
      return true
    })
    h.session.start()
    h.rooms[0].ready()
    h.type('Base typed while reconnecting')
    h.rooms[0].set({ status: 'connecting' })
    h.session.flush({ reason: 'hidden' })
    expect(h.rooms[0].flushed).toBe(1)
    // The room has the Yjs updates and merges them on reconnect. A normal
    // save would overwrite what peers saved meanwhile and restart the room.
    expect(calls).toEqual([])
  })

  it('flushes a connected room instead of saving normally when the editor unmounts', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base last edit')
    h.session.flush({ reason: 'leaving' })
    expect(h.rooms[0].flushed).toBe(1)
    expect(h.rest).toEqual([])
  })

  it('saves the last edit normally when the page unload already destroyed the room', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    // The provider's own pagehide handler ran first.
    h.rooms[0].destroy()
    h.type('Base typed just before closing')
    h.session.flush({ reason: 'leaving' })
    expect(h.rooms[0].queued).toEqual([])
    expect(h.rest).toEqual(['Base typed just before closing'])
  })

  it('also saves an unacknowledged body normally when the page unloads', () => {
    const h = harness()
    const calls: Array<{ body: string; keepalive?: boolean }> = []
    h.session.setCommitRest((body, options) => {
      calls.push({ body, keepalive: options?.keepalive })
      return true
    })
    h.session.start()
    h.rooms[0].ready()
    h.type('Base sent but not acknowledged')
    h.session.flush({ reason: 'unloading' })
    expect(h.rooms[0].flushed).toBe(1)
    expect(calls).toEqual([{ body: 'Base sent but not acknowledged', keepalive: true }])
  })

  it('saves the last edit in a request that outlives the page while saving normally', () => {
    const h = harness()
    const calls: Array<{ body: string; keepalive?: boolean }> = []
    h.session.setCommitRest((body, options) => {
      calls.push({ body, keepalive: options?.keepalive })
      return new Promise<boolean>(() => {})
    })
    h.session.start()
    h.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('disabled', 'disabled', 404),
    })
    expect(h.session.getView().mode).toBe('unavailable')
    h.type('Base saved normally')
    // Typed while that save is on its way; the closing page flushes it.
    setBody(h.boundTo, 'Base typed while closing')
    h.session.commit('Base typed while closing', { keepalive: true })
    h.session.flush({ reason: 'unloading' })
    expect(calls).toEqual([
      { body: 'Base saved normally', keepalive: undefined },
      { body: 'Base typed while closing', keepalive: true },
    ])
  })

  it('sends the newest body again when its ordinary save may not outlive the page', () => {
    const h = harness()
    const calls: Array<{ body: string; keepalive?: boolean }> = []
    h.session.setCommitRest((body, options) => {
      calls.push({ body, keepalive: options?.keepalive })
      return new Promise<boolean>(() => {})
    })
    h.session.start()
    h.rooms[0].set({
      status: 'failed',
      error: new PhotonLiveError('disabled', 'disabled', 404),
    })
    h.type('Base saved normally')
    h.session.flush({ reason: 'hidden' })
    h.session.flush({ reason: 'unloading' })
    h.session.flush({ reason: 'unloading' })
    expect(calls).toEqual([
      { body: 'Base saved normally', keepalive: undefined },
      { body: 'Base saved normally', keepalive: true },
    ])
  })

  it('does not save normally on unload once the room acknowledged the body', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base acknowledged')
    h.rooms[0].ack()
    h.session.flush({ reason: 'unloading' })
    expect(h.rest).toEqual([])
  })

  it('saves normally when leaving while the room is disconnected', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base typed offline')
    h.rooms[0].set({ status: 'disconnected' })
    h.session.flush({ reason: 'leaving' })
    expect(h.rest).toEqual(['Base typed offline'])
  })

  it('keeps a leaving room open until it acknowledges the last body', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base in flight')
    h.session.release()
    vi.advanceTimersByTime(0)
    expect(h.rooms[0].destroyed).toBe(false)
    h.rooms[0].ack()
    expect(h.rooms[0].destroyed).toBe(true)
    expect(h.rest).toEqual([])
  })

  it('saves normally when a leaving room never acknowledges the last body', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base never acked')
    h.session.release()
    vi.advanceTimersByTime(5_000)
    expect(h.rooms[0].destroyed).toBe(true)
    expect(h.rest).toEqual(['Base never acked'])
  })

  it('saves normally when a leaving room rejects the last body', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base rejected on the way out')
    h.session.release()
    vi.advanceTimersByTime(0)
    h.rooms[0].set({ saveStatus: 'error' })
    expect(h.rooms[0].destroyed).toBe(true)
    expect(h.rest).toEqual(['Base rejected on the way out'])
  })

  it('does not save normally when a leaving room reports a conflict', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base draft on the way out')
    h.session.release()
    vi.advanceTimersByTime(0)
    h.rooms[0].set({ saveStatus: 'conflict' })
    expect(h.rooms[0].destroyed).toBe(true)
    expect(h.rest).toEqual([])
  })

  it('saves a disconnected leaving body once, not from both flush and drain', () => {
    const h = harness()
    h.setRestResult(new Promise<boolean>(() => {}))
    h.session.start()
    h.rooms[0].ready()
    h.type('Base typed offline')
    h.rooms[0].set({ status: 'disconnected' })
    h.session.flush({ reason: 'leaving' })
    h.session.release()
    vi.advanceTimersByTime(0)
    expect(h.rooms[0].destroyed).toBe(true)
    expect(h.rest).toEqual(['Base typed offline'])
  })

  it('survives a development double mount and is destroyed only on a real release', () => {
    const h = harness()
    h.session.start()
    h.session.release()
    h.session.start()
    vi.advanceTimersByTime(0)
    expect(h.rooms[0].destroyed).toBe(false)
    h.rooms[0].ready()
    expect(h.session.getView().mode).toBe('live')

    h.session.release()
    vi.advanceTimersByTime(0)
    expect(h.rooms[0].destroyed).toBe(true)
  })

  it('treats a room that saved everything as the new durable body', () => {
    const h = harness()
    h.session.start()
    h.rooms[0].ready()
    h.type('Base acknowledged')
    h.rooms[0].ack()
    // Kicked out after the ACK: nothing on screen is unsaved, so the next
    // room is adopted as it is.
    h.rooms[0].set({ status: 'failed', saveStatus: 'conflict' })
    h.seedRooms((room) => setBody(room.fragment, 'Base acknowledged, and a peer'))
    vi.advanceTimersByTime(1_000)
    h.rooms[1].ready()
    expect(h.session.getView().mode).toBe('live')
    expect(h.onScreen).toBe('Base acknowledged, and a peer')
  })
})
