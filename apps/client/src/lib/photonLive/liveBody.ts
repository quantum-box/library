import * as Y from 'yjs'
import type { PhotonLiveError, PhotonLiveProvider, PhotonLiveState } from './types'

/**
 * What the body session needs from the editor on screen.
 *
 * The editor is bound to exactly one Yjs fragment at a time: first a local
 * draft seeded from the body the page loaded, later a room. Rebinding swaps
 * the editor's Yjs plugins in place -- the editor itself, its DOM, its caret
 * and any IME are never torn down, which is the whole point.
 */
export interface LiveBodyEditorPort {
  /** A fragment's body, serialized exactly the way commits are. */
  serializeFragment(fragment: Y.XmlFragment): string
  /**
   * A serialized body without identity that differs between copies of the
   * same content (generated block ids, the trailing empty block). Two bodies
   * are the same content exactly when these are equal.
   */
  comparable(body: string): string
  /** Make the room fragment hold what `from` holds, as a minimal Yjs diff. */
  writeInto(target: Y.XmlFragment, from: Y.XmlFragment): void
  /**
   * Merge what `from` holds and what `target` holds, both descended from the
   * serialized `base`, and write the result into `target`. Nothing either
   * side added is dropped.
   */
  mergeInto(target: Y.XmlFragment, from: Y.XmlFragment, base: string): void
  /** Point the editor's sync, cursor and undo plugins at this room. */
  bind(provider: PhotonLiveProvider): void
  /** Rebinding re-renders the document, which would break a composition. */
  composing(): boolean
}

/**
 * - `joining`: the body is on screen from the local draft and a room is
 *   being opened. Nothing typed here has reached anyone yet.
 * - `live`: a room carries the body; saves are the room's checkpoints.
 * - `rejoining`: the room this editor was in stopped carrying the body and a
 *   fresh one is being opened.
 * - `conflict`: the body on screen and the canonical body both changed since
 *   they last agreed, after the canonical body was changed elsewhere.
 *   Neither is merged into the other automatically; the person's next edit
 *   is saved through the ordinary body, after which the editor rejoins on
 *   that body.
 * - `unavailable`: Live is off, refuses this person, or keeps refusing this
 *   body. Ordinary saves only.
 */
export type LiveBodyMode = 'joining' | 'live' | 'rejoining' | 'conflict' | 'unavailable'

export interface LiveBodyView {
  mode: LiveBodyMode
  /** The state of the room carrying the body, or of the one being opened. */
  provider: PhotonLiveState | null
  /** Edits typed while no room carried the body that are not saved yet. */
  holding: boolean
  /** Whether any room has carried this body during this mount. */
  joined: boolean
}

export interface LiveBodyTiming {
  /**
   * How long edits typed while no room carries the body wait for one before
   * they are saved through the ordinary body instead. Saving them through
   * REST changes the canonical body under every open room, which rotates it
   * and puts everyone in it through a rejoin, so a room that is about to
   * answer is worth a short wait. A room that is not is not worth more.
   */
  joinGraceMs: number
  /** A room that stays disconnected this long while online is abandoned. */
  stallMs: number
  retryBaseMs: number
  retryMaxMs: number
  /** How long a leaving page waits for the room to acknowledge its last body. */
  drainMs: number
}

const DEFAULT_TIMING: LiveBodyTiming = {
  joinGraceMs: 10_000,
  stallMs: 15_000,
  retryBaseMs: 1_000,
  retryMaxMs: 30_000,
  drainMs: 5_000,
}

/** Permanent refusals of a room in a row before Live is given up for this mount. */
const MAX_PERMANENT_FAILURES = 3
/** Checkpoints rejected in a row before the body is saved normally instead. */
const MAX_CHECKPOINT_ERRORS = 2

export type CommitRest = (
  body: string,
  options?: { keepalive?: boolean },
) => Promise<boolean> | boolean | void

/** Why the editor is flushing: see `LiveBodySession.flush`. */
export type FlushReason = 'hidden' | 'leaving' | 'unloading'

export interface LiveBodySessionOptions {
  /** Owned by the session: the local document the editor starts on. */
  draft: Y.XmlFragment
  createProvider(): PhotonLiveProvider
  /**
   * Save through the ordinary body. Resolving `false` (or rejecting) means
   * the body did not become durable. Replaceable with `setCommitRest`.
   */
  commitRest?: CommitRest
  /**
   * Save a body only if the record is still at `expectedRecordVersion`, in a
   * request that outlives the page. Used when the page is hidden while its
   * room is out of reach: it can never replace anything saved since.
   */
  checkpointOutlivingPage?: (body: string, expectedRecordVersion: string) => unknown
  timing?: Partial<LiveBodyTiming>
  isOnline?: () => boolean
}

interface Candidate {
  provider: PhotonLiveProvider
  unsubscribe: () => void
  /** The ordinary save count when this room's session was requested. */
  restSeq: number
}

type Degrade = 'conflict' | 'error' | 'stall'

/**
 * Carries one record body between the editor, its rooms and the ordinary
 * body save, for the lifetime of one editor mount.
 *
 * Live is auxiliary. The body is editable from the first frame; typing never
 * waits for a room, and joining one never replaces the editor. When a room
 * becomes ready this decides how to join it without losing anything:
 *
 * - the same room again (a reconnect): merge the Yjs documents, exactly as a
 *   provider's own reconnect would;
 * - nothing unsaved on screen, or the room already holds it: adopt the room;
 * - the room still holds the saved body: write what is on screen into the
 *   room, so edits typed before it answered reach everyone in it as ordinary
 *   Yjs changes;
 * - both changed, with edits typed before any room carried them: merge the
 *   two block by block into the room;
 * - both changed after the canonical body was changed elsewhere: a conflict
 *   left for the person to resolve, never saved over the other body.
 *
 * A room that stops carrying the body (a canonical change, a rejected save,
 * a connection that never comes back) is replaced by a fresh one the same
 * way, instead of leaving the editor on ordinary saves for the rest of the
 * mount.
 */
export class LiveBodySession {
  private readonly draft: Y.XmlFragment
  private readonly options: LiveBodySessionOptions
  private readonly timing: LiveBodyTiming
  private commitRestImpl: CommitRest
  private readonly listeners = new Set<(view: LiveBodyView) => void>()
  private port: LiveBodyEditorPort | null = null
  private mode: LiveBodyMode = 'joining'
  private started = false
  private disposed = false
  /** The last body known to be durable, as serialized by the editor. */
  private durable: string | null = null
  /**
   * The newest ordinary save `durable` reflects. A save the page sent later
   * may overtake an earlier one (see `flush`), so an older one settling last
   * must not move `durable` back.
   */
  private durableRestSeq = 0
  /** The room the editor is bound to, healthy or not. Null while on the draft. */
  private bound: { provider: PhotonLiveProvider; unsubscribe: () => void } | null = null
  private candidate: Candidate | null = null
  /** A body typed while no room carried it, not yet sent anywhere. */
  private held: string | null = null
  /**
   * The held body is what a room stopped carrying, not only what was typed
   * while none carried it. It is saved if no room answers, but never merged
   * into a room whose body changed elsewhere.
   */
  private heldFromRoom = false
  /**
   * The canonical body changed elsewhere while unsaved edits were on screen.
   * Until the next room decides, nothing on screen may be saved over it.
   */
  private canonicalConflict = false
  /**
   * The grace for held edits ran out without a room. Until one is joined,
   * later edits are saved through the ordinary body straight away.
   */
  private graceExpired = false
  private graceTimer: ReturnType<typeof setTimeout> | null = null
  private stallTimer: ReturnType<typeof setTimeout> | null = null
  private retryTimer: ReturnType<typeof setTimeout> | null = null
  private releaseTimer: ReturnType<typeof setTimeout> | null = null
  private retryDelay: number
  private permanentFailures = 0
  private checkpointErrors = 0
  private restSeq = 0
  private restInFlight = 0
  /** The body of the newest ordinary save still in flight. */
  private restPending: string | null = null
  /** Whether that save outlives the page (see `flush`). */
  private restPendingKeepalive = false
  /** What `checkpointOutlivingPage` was last asked for, so it goes once. */
  private lastPageCheckpoint: string | null = null
  /** The body most recently handed to the bound room as a checkpoint. */
  private lastQueued: string | null = null
  private joined = false
  private draftDestroyed = false
  /** Inside bindTo: the room's own reports are about the rebinding itself. */
  private binding = false

  constructor(options: LiveBodySessionOptions) {
    this.options = options
    this.draft = options.draft
    this.timing = { ...DEFAULT_TIMING, ...options.timing }
    this.retryDelay = this.timing.retryBaseMs
    this.commitRestImpl = options.commitRest ?? (() => false)
  }

  /** The ordinary body save, which the caller may replace on every render. */
  setCommitRest(commitRest: CommitRest): void {
    this.commitRestImpl = commitRest
  }

  /** Begin opening a room. Safe to call again after `release()`. */
  start(): void {
    this.clearTimer('releaseTimer')
    if (this.disposed || this.started) return
    this.started = true
    if (this.mode === 'joining' || this.mode === 'rejoining') this.createCandidate()
  }

  /**
   * Destroy on the next task unless started again first. React's development
   * double-mount stops and restarts an effect synchronously; a real unmount
   * does not come back. Deferring also lets the editor's own unmount flush,
   * which runs after this component's cleanup, still reach the room.
   */
  release(): void {
    if (this.disposed || this.releaseTimer !== null) return
    this.releaseTimer = globalThis.setTimeout(() => {
      this.releaseTimer = null
      this.destroy()
    }, 0)
  }

  destroy(): void {
    if (this.disposed) return
    this.disposed = true
    this.started = false
    for (const timer of ['releaseTimer', 'retryTimer', 'stallTimer', 'graceTimer'] as const) {
      this.clearTimer(timer)
    }
    this.dropCandidate()
    const bound = this.bound
    this.bound = null
    if (bound) {
      bound.unsubscribe()
      if (this.mode === 'live') this.drain(bound.provider)
      else bound.provider.destroy()
    }
    if (!this.canonicalConflict) this.releaseHold()
    this.destroyDraft()
    this.listeners.clear()
  }

  /** The editor exists and is bound to the draft. */
  setEditor(port: LiveBodyEditorPort): void {
    this.port = port
    this.durable ??= port.serializeFragment(this.draft)
    this.tryAttach()
  }

  getView(): LiveBodyView {
    const provider = this.mode === 'live'
      ? this.bound?.provider ?? null
      : this.candidate?.provider ?? null
    return {
      mode: this.mode,
      provider: provider?.getState() ?? null,
      holding: this.held !== null,
      joined: this.joined,
    }
  }

  subscribe(listener: (view: LiveBodyView) => void): () => void {
    this.listeners.add(listener)
    listener(this.getView())
    return () => this.listeners.delete(listener)
  }

  /**
   * A debounced body from the editor. The session decides where it goes.
   * `keepalive`: the page is unloading, so a body saved normally must go in
   * a request that outlives it.
   */
  commit(body: string, options?: { keepalive?: boolean }): void {
    if (this.disposed) return
    switch (this.mode) {
      case 'live':
        this.queueCheckpoint(body, options)
        return
      case 'joining':
      case 'rejoining':
        if (this.canonicalConflict) {
          // Wait for the next room to say whether this conflicts.
          this.held = body
          this.heldFromRoom = true
          this.startGrace()
          this.emit()
          return
        }
        if (this.graceExpired) {
          this.commitRest(body, options)
          return
        }
        // Once a room's unsaved body is held, what is typed on top of it is
        // still that body.
        this.heldFromRoom = this.heldFromRoom && this.held !== null
        this.held = body
        this.startGrace()
        this.emit()
        return
      case 'conflict':
        if (this.same(body, this.durable)) {
          // Undone back to the saved body: nothing is in conflict any more.
          this.mode = 'rejoining'
          this.scheduleCandidate()
          this.emit()
          return
        }
        this.commitRest(body, options)
        return
      case 'unavailable':
        this.commitRest(body, options)
    }
  }

  /**
   * Save what can be saved now.
   *
   * - `hidden`: the page may come back -- or be discarded without another
   *   event, as mobile browsers do. A room sends its checkpoint at once.
   *   Edits held for a room are saved normally, in a request that outlives
   *   the page.
   * - `leaving`: this editor unmounts in a page that stays. As `hidden`, in
   *   ordinary requests, and a disconnected room's body is saved normally; a
   *   connected room is kept open until it acknowledges (see `drain`).
   * - `unloading`: the page is going away and its socket with it, whether or
   *   not the room received the last body. Anything the room has not
   *   acknowledged is saved normally too, in a request that outlives the page.
   *
   * A room's body is not saved normally while the page is only hidden, even
   * when it is reconnecting. That save is unconditional: it would change the
   * canonical body under the room -- possibly to a document still missing
   * what peers saved meanwhile -- and restart the room for everyone in it,
   * on a mere switch away from the tab. Updates made while connected are in
   * the room already, and its next checkpoint saves them. Updates made while
   * reconnecting are only in this page's document until a socket opens again,
   * so a room out of reach has its body saved on the version it last saw
   * (`checkpointOutlivingPage`): written only if nothing was saved since,
   * never over it.
   *
   * Never while the body may conflict with one changed elsewhere.
   */
  flush({ reason = 'hidden' }: { reason?: FlushReason } = {}): void {
    if (this.disposed) return
    const keepalive = reason !== 'leaving'
    if (this.mode === 'live' && this.bound) {
      const provider = this.bound.provider
      if (provider.destroyed) {
        // Gone before its last body was acknowledged; it cannot send it now.
        const onScreen = this.bodyOnScreen()
        if (onScreen !== null && !this.same(onScreen, this.durable)) {
          this.commitRest(onScreen, { keepalive })
        }
        return
      }
      provider.flushCheckpoint()
      const unsent = provider.unsentBody()
      if (unsent === null) return
      if (reason === 'hidden') {
        if (provider.getState().status !== 'connected') {
          this.checkpointOutlivingPage(this.lastQueued ?? unsent, provider.getState().recordVersion)
        }
        return
      }
      if (reason === 'unloading' || provider.getState().status !== 'connected') {
        this.commitRest(this.lastQueued ?? unsent, { keepalive })
      }
      return
    }
    if (this.canonicalConflict) return
    const body = this.held
    if (body === null) {
      // The newest body may be on its way in an ordinary save, which a page
      // that is unloaded or discarded cancels. The same body again cannot
      // undo anything whichever of the two lands last.
      if (keepalive && this.restPending !== null) this.commitRest(this.restPending, { keepalive })
      return
    }
    this.dropHold()
    this.commitRest(body, { keepalive })
  }

  /** A binding deferred for an open composition can happen now. */
  compositionEnded(): void {
    this.tryAttach()
  }

  private emit(): void {
    const view = this.getView()
    this.listeners.forEach((listener) => listener(view))
  }

  private clearTimer(name: 'graceTimer' | 'stallTimer' | 'retryTimer' | 'releaseTimer'): void {
    const timer = this[name]
    if (timer !== null) globalThis.clearTimeout(timer)
    this[name] = null
  }

  private isOnline(): boolean {
    if (this.options.isOnline) return this.options.isOnline()
    return typeof navigator === 'undefined' || navigator.onLine !== false
  }

  private same(left: string | null, right: string | null): boolean {
    if (left === null || right === null) return left === right
    if (left === right) return true
    const port = this.port
    return port !== null && port.comparable(left) === port.comparable(right)
  }

  private destroyDraft(): void {
    if (this.draftDestroyed) return
    this.draftDestroyed = true
    this.draft.doc?.destroy()
  }

  /**
   * Keep a room the page is leaving open until it has taken the last body:
   * destroying it now would drop a checkpoint still queued or in flight.
   * If it does not take it in time, the ordinary body gets it.
   */
  private drain(provider: PhotonLiveProvider): void {
    const lastBody = this.lastQueued
    if (provider.unsentBody() === null) {
      provider.destroy()
      return
    }
    if (provider.getState().status !== 'connected') {
      if (lastBody !== null) this.commitRest(lastBody)
      provider.destroy()
      return
    }
    let done = false
    let unsubscribe = () => {}
    const finish = (saveNormally: boolean) => {
      if (done) return
      done = true
      globalThis.clearTimeout(timer)
      unsubscribe()
      if (saveNormally && lastBody !== null) this.commitRest(lastBody)
      provider.destroy()
    }
    const timer = globalThis.setTimeout(
      () => finish(provider.unsentBody() !== null),
      this.timing.drainMs,
    )
    unsubscribe = provider.subscribe((state) => {
      if (provider.unsentBody() === null) finish(false)
      // A conflict means the body changed elsewhere: saving over it is not
      // this page's call. A rejected checkpoint is not that, and the body
      // still has to be saved somewhere.
      else if (state.saveStatus === 'conflict') finish(false)
      else if (state.saveStatus === 'error') finish(true)
    })
  }

  private checkpointOutlivingPage(body: string, expectedRecordVersion: string): void {
    const checkpoint = this.options.checkpointOutlivingPage
    if (!checkpoint) return
    const key = `${expectedRecordVersion}\n${body}`
    if (key === this.lastPageCheckpoint) return
    this.lastPageCheckpoint = key
    try {
      void Promise.resolve(checkpoint(body, expectedRecordVersion)).catch(() => undefined)
    } catch {
      // Best effort: the room carries the body once the page comes back.
    }
  }

  private queueCheckpoint(body: string, options?: { keepalive?: boolean }): void {
    const provider = this.bound?.provider
    if (!provider) return
    if (provider.destroyed) {
      // The page is unloading and the room went first; it would drop this.
      this.commitRest(body, options)
      return
    }
    this.lastQueued = body
    provider.queueCheckpoint(body)
  }

  private startGrace(): void {
    if (this.graceTimer !== null) return
    this.graceTimer = globalThis.setTimeout(() => {
      this.graceTimer = null
      if (this.mode !== 'joining' && this.mode !== 'rejoining') return
      if (this.canonicalConflict) {
        // No room came to decide. Say so rather than saving over the body
        // that changed elsewhere; the next edit is the person's decision.
        this.enterConflict()
      } else {
        this.graceExpired = true
        this.releaseHold()
      }
      this.emit()
    }, this.timing.joinGraceMs)
  }

  private releaseHold(): void {
    this.clearTimer('graceTimer')
    const body = this.held
    this.held = null
    this.heldFromRoom = false
    if (body !== null) this.commitRest(body)
  }

  private dropHold(): void {
    this.clearTimer('graceTimer')
    this.held = null
    this.heldFromRoom = false
  }

  private enterConflict(): void {
    this.dropCandidate()
    this.clearTimer('retryTimer')
    this.dropHold()
    this.mode = 'conflict'
    this.canonicalConflict = false
    this.graceExpired = false
  }

  private commitRest(body: string, options?: { keepalive?: boolean }): void {
    if (body === this.durable && this.restInFlight === 0) return
    // Leaving can reach this twice for one body (the page's flush, then the
    // room's drain). One save of it in flight is enough -- unless the page is
    // unloading and that one would not outlive it.
    if (body === this.restPending && (this.restPendingKeepalive || !options?.keepalive)) return
    const seq = ++this.restSeq
    this.restInFlight += 1
    this.restPending = body
    this.restPendingKeepalive = Boolean(options?.keepalive)
    let result: ReturnType<CommitRest>
    try {
      result = this.commitRestImpl(body, options)
    } catch {
      result = false
    }
    void Promise.resolve(result)
      .then((saved) => {
        if (saved !== false && seq > this.durableRestSeq) {
          this.durable = body
          this.durableRestSeq = seq
        }
      }, () => undefined)
      .finally(() => {
        this.restInFlight -= 1
        if (this.restSeq === seq) {
          this.restPending = null
          this.restPendingKeepalive = false
        }
        this.afterRestSettled()
      })
  }

  private afterRestSettled(): void {
    if (this.disposed || this.restInFlight > 0) return
    if (this.mode === 'conflict') {
      // The person resolved the conflict by saving what is on screen. Once
      // that is the canonical body, a fresh room will carry it again.
      if (!this.same(this.bodyOnScreen(), this.durable)) return
      this.mode = 'rejoining'
      this.graceExpired = false
      this.emit()
    }
    if (this.mode !== 'joining' && this.mode !== 'rejoining') return
    if (this.candidate && this.candidate.restSeq !== this.restSeq) this.dropCandidate()
    if (this.candidate) this.tryAttach()
    else this.scheduleCandidate()
  }

  private bodyOnScreen(): string | null {
    const port = this.port
    if (!port) return null
    return port.serializeFragment(this.boundFragment())
  }

  private boundFragment(): Y.XmlFragment {
    return this.bound?.provider.fragment ?? this.draft
  }

  private createCandidate(): void {
    if (this.disposed || !this.started || this.candidate) return
    if (this.mode !== 'joining' && this.mode !== 'rejoining') return
    // A room opened while an ordinary save is in flight would be authorized
    // against the body before that save and hold it. Wait for the save.
    if (this.restInFlight > 0) return
    const provider = this.options.createProvider()
    const candidate: Candidate = {
      provider,
      restSeq: this.restSeq,
      unsubscribe: () => undefined,
    }
    this.candidate = candidate
    candidate.unsubscribe = provider.subscribe((state) => this.onProviderState(provider, state))
  }

  private dropCandidate(): void {
    const candidate = this.candidate
    if (!candidate) return
    this.candidate = null
    candidate.unsubscribe()
    candidate.provider.destroy()
  }

  private scheduleCandidate(): void {
    if (this.disposed || this.retryTimer !== null || this.candidate) return
    const delay = this.retryDelay
    this.retryDelay = Math.min(this.retryDelay * 2, this.timing.retryMaxMs)
    this.retryTimer = globalThis.setTimeout(() => {
      this.retryTimer = null
      this.createCandidate()
    }, delay)
  }

  private onProviderState(provider: PhotonLiveProvider, state: PhotonLiveState): void {
    // bindTo reads the state itself once the binding is complete.
    if (this.disposed || this.binding) return
    if (this.candidate?.provider === provider) {
      this.onCandidateState(state)
    } else if (this.bound?.provider === provider && this.mode === 'live') {
      this.onBoundState(state)
    }
    this.emit()
  }

  private onCandidateState(state: PhotonLiveState): void {
    if (state.saveStatus === 'conflict' || state.error?.kind === 'conflict') {
      // The room rotated between authorization and the handshake.
      this.dropCandidate()
      this.scheduleCandidate()
      return
    }
    if (state.status === 'failed' && state.error && !state.error.retryable) {
      this.candidateRefused(state.error)
      return
    }
    if (state.initialized && state.status === 'connected') this.tryAttach()
  }

  private candidateRefused(error: PhotonLiveError): void {
    this.dropCandidate()
    this.permanentFailures += 1
    if (error.kind === 'disabled' || this.permanentFailures >= MAX_PERMANENT_FAILURES) {
      this.giveUp()
      return
    }
    this.scheduleCandidate()
  }

  /** Live will not carry this body in this mount: ordinary saves only. */
  private giveUp(): void {
    this.dropCandidate()
    this.clearTimer('retryTimer')
    this.mode = 'unavailable'
    this.canonicalConflict = false
    this.releaseHold()
  }

  private onBoundState(state: PhotonLiveState): void {
    if (state.saveStatus === 'saved' && !state.hasUnackedChanges && this.lastQueued !== null) {
      // A save this room acknowledged: the room works, and this is durable.
      this.durable = this.lastQueued
      this.durableRestSeq = this.restSeq
      this.checkpointErrors = 0
      this.permanentFailures = 0
      this.retryDelay = this.timing.retryBaseMs
    }
    // A canonical change reports the failed connection before the conflict
    // it causes. Both are the same event: the body changed elsewhere.
    if (state.saveStatus === 'conflict' || state.error?.kind === 'conflict') {
      this.degrade('conflict')
      return
    }
    if (state.saveStatus === 'error') {
      this.degrade('error')
      return
    }
    if (state.status === 'failed' && state.error !== null && !state.error.retryable) {
      this.degrade('stall')
      return
    }
    if (state.status === 'connected' || !this.isOnline()) {
      this.clearTimer('stallTimer')
    } else if (this.stallTimer === null) {
      // Every reconnect re-authorizes, and the worker refuses some rooms at
      // the upgrade, which a browser only reports as a failed socket. A room
      // that never comes back is not "offline": open a fresh one. If it is
      // the same room after all, the documents are merged on joining it.
      this.stallTimer = globalThis.setTimeout(() => {
        this.stallTimer = null
        if (this.mode === 'live') {
          this.degrade('stall')
          this.emit()
        }
      }, this.timing.stallMs)
    }
  }

  /**
   * The bound room stopped carrying the body. The editor stays bound to its
   * document -- that is what is on screen -- while a fresh room is opened.
   *
   * After a canonical change elsewhere, unsaved edits wait for the next room
   * to decide whether they conflict. After a stall or a rejected save they
   * are held: written into the next room, or saved normally if none answers.
   */
  private degrade(kind: Degrade): void {
    const bound = this.bound
    if (!bound) return
    // Leave `live` first: detaching reports a disconnect synchronously.
    this.mode = 'rejoining'
    this.clearTimer('stallTimer')
    bound.provider.detach()
    this.graceExpired = false
    const onScreen = this.bodyOnScreen()
    const unsaved = onScreen !== null && !this.same(onScreen, this.durable)
    if (kind === 'conflict') {
      this.canonicalConflict = unsaved
      this.dropHold()
    } else if (unsaved) {
      this.held = onScreen
      this.heldFromRoom = true
      this.startGrace()
    }
    if (kind === 'error') {
      this.checkpointErrors += 1
      if (this.checkpointErrors >= MAX_CHECKPOINT_ERRORS) {
        // The room keeps rejecting this body (too large, invalid): no other
        // room will take it either. The ordinary save reports why.
        this.giveUp()
        return
      }
    }
    this.scheduleCandidate()
  }

  private tryAttach(): void {
    const port = this.port
    const candidate = this.candidate
    if (this.disposed || !port || !candidate) return
    const state = candidate.provider.getState()
    if (!state.initialized || state.status !== 'connected') return
    if (this.restInFlight > 0) return
    if (candidate.restSeq !== this.restSeq) {
      // The room was authorized before an ordinary save it cannot know about.
      this.dropCandidate()
      this.createCandidate()
      return
    }
    if (port.composing()) return

    const previous = this.bound?.provider ?? null
    if (
      previous !== null &&
      previous.roomGeneration !== null &&
      previous.roomGeneration === candidate.provider.roomGeneration
    ) {
      // The same room: whatever this editor has that it lacks was typed here.
      // Merge the documents the way the old provider's reconnect would have.
      this.bindTo(candidate, 'changed', () => Y.applyUpdate(
        candidate.provider.doc,
        Y.encodeStateAsUpdate(previous.doc, Y.encodeStateVector(candidate.provider.doc)),
      ))
      return
    }

    const theirs = port.serializeFragment(candidate.provider.fragment)
    const ours = this.bodyOnScreen()
    const base = this.durable
    if (ours === null || base === null) return
    const from = this.boundFragment()
    if (this.same(ours, theirs) || this.same(ours, base)) {
      this.bindTo(candidate, 'adopted')
    } else if (this.same(theirs, base)) {
      this.bindTo(candidate, 'changed', () => port.writeInto(candidate.provider.fragment, from))
    } else if (this.canonicalConflict || this.heldFromRoom) {
      // The body changed elsewhere while this one had unsaved edits: which
      // should win is the person's call, not this session's.
      this.enterConflict()
    } else {
      // Typed before any room carried it, while the room moved on. Keep both.
      this.bindTo(candidate, 'changed', () => port.mergeInto(candidate.provider.fragment, from, base))
    }
  }

  /**
   * `change` writes into the room before the editor is bound to it. Like the
   * binding itself, it runs Yjs transactions the room reports synchronously.
   */
  private bindTo(
    candidate: Candidate,
    how: 'adopted' | 'changed',
    change?: () => void,
  ): void {
    const port = this.port
    if (!port) return
    const previous = this.bound
    // Settle this session first. Writing into the room and binding to it run
    // Yjs transactions synchronously -- the sync plugin writes the editor back
    // into the room as it binds -- and the room reports them before either
    // returns. Seen half-way, this candidate would be decided on again
    // against a document mid-rebind.
    this.bound = { provider: candidate.provider, unsubscribe: candidate.unsubscribe }
    this.candidate = null
    this.mode = 'live'
    this.joined = true
    this.graceExpired = false
    this.canonicalConflict = false
    this.clearTimer('graceTimer')
    this.clearTimer('retryTimer')
    // A room that let this editor in ends the run of refusals: isolated ones
    // across later rejoins must not add up to giving up on Live. Rejected
    // checkpoints keep counting -- they are about this body, not the join.
    this.permanentFailures = 0
    this.retryDelay = this.timing.retryBaseMs
    this.held = null
    this.heldFromRoom = false
    this.binding = true
    try {
      change?.()
      port.bind(candidate.provider)
    } finally {
      this.binding = false
    }
    if (previous) {
      previous.unsubscribe()
      previous.provider.destroy()
    } else {
      this.destroyDraft()
    }

    const onScreen = port.serializeFragment(candidate.provider.fragment)
    if (how === 'adopted') {
      // Nothing was changed by joining. Checkpointing the room back would
      // rewrite the stored body through a lossy serialization, and bump its
      // version, just because a page was opened.
      this.durable = onScreen
      this.durableRestSeq = this.restSeq
      this.lastQueued = null
    } else if (!this.same(onScreen, this.durable)) {
      this.queueCheckpoint(onScreen)
    }
    this.onBoundState(candidate.provider.getState())
    this.emit()
  }
}
