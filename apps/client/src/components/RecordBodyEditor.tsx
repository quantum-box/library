import { useCallback, useEffect, useRef, useState } from 'react'
import {
  BlockNoteEditor,
  YCursorExtension,
  YSyncExtension,
  YUndoExtension,
  filterSuggestionItems,
  insertOrUpdateBlockForSlashMenu,
} from '@blocknote/core'
import { blocksToYDoc, blocksToYXmlFragment, yXmlFragmentToBlocks } from '@blocknote/core/yjs'
import { merge3 } from '../lib/photonLive/merge3'
import {
  SuggestionMenuController,
  getDefaultReactSlashMenuItems,
  useCreateBlockNote,
  useEditorChange,
} from '@blocknote/react'
import { BlockNoteView } from '@blocknote/shadcn'
import { CodeXml } from 'lucide-react'
import '@blocknote/core/fonts/inter.css'
import '@blocknote/shadcn/style.css'
import { recordBodySchema } from './blocknote/schema'
import { HtmlArtifactEditor } from './HtmlArtifactEditor'
import { isArtifactHtml } from '../lib/libraryTable/bodyProperty'
import { uploadLibraryImage } from '../lib/recordsApi'
import {
  takeImageWidthFragments,
  withImageWidthFragments,
} from './blocknote/imageWidthFragments'
import { t } from '../i18n'
import { appKitConfig } from '../app/kitConfig'
import { PhotonLiveStatus } from './PhotonLiveStatus'
import {
  LiveBodySession,
  createPhotonLiveProvider,
  defaultUser,
  type LiveBodyEditorPort,
  type LiveBodyView,
  type PhotonLiveFormat,
  type PhotonLiveProvider,
  type PhotonLiveRecordTarget,
  type PhotonLiveState,
} from '../lib/photonLive'
import * as Y from 'yjs'

export type RecordBodyFormat = 'markdown' | 'richText' | 'html'

/** The repository an image dropped into the body is stored against. */
export interface RecordBodyImageTarget {
  org: string
  repo: string
  operatorId?: string
}

export interface RecordBodyEditorProps {
  value: string
  /**
   * How `value` is encoded and what onCommit receives.
   *
   * - `markdown`: the historical mode. Lossy — Markdown cannot represent an
   *   empty paragraph, so blank lines do not survive a round trip.
   * - `richText`: `value` is the block document as JSON and the editor's own
   *   document is committed back. Lossless; this is the reason the RichText
   *   property type exists.
   * - `html`: `value` is markup, treated as an artifact: previewed in a
   *   sandboxed frame and edited as source, never parsed into blocks. The
   *   exception is a value that is really Markdown — this editor used to
   *   commit Markdown into Html Properties, so those open in the block
   *   editor exactly as before.
   */
  format?: RecordBodyFormat
  /**
   * Save the body. A Live body also needs to know when that save is durable,
   * so a caller that can tell resolves `true` once it is, `false` if it
   * failed.
   */
  onCommit: (value: string) => void | Promise<boolean>
  editable?: boolean
  surface?: 'panel' | 'page' | 'fill'
  /** Pin the read-only public reader independently of the OS theme. */
  theme?: 'light' | 'dark'
  /**
   * Where to store pasted and dropped images. Without it the editor still
   * embeds images by URL, but has nowhere to put a local file, so BlockNote
   * hides the upload tab.
   */
  imageTarget?: RecordBodyImageTarget
  /** The record/property scope used by the opt-in Photon Live adapter. */
  liveTarget?: PhotonLiveRecordTarget
}

export function RecordBodyEditor(props: RecordBodyEditorProps) {
  // Decided once per mount, so typing cannot flip a record between editors
  // mid-edit. Callers key this component by record id.
  const [artifact] = useState(
    () => (props.format ?? 'markdown') === 'html' && isArtifactHtml(props.value),
  )
  if (artifact) {
    return (
      <HtmlArtifactEditor
        value={props.value}
        onCommit={props.onCommit}
        editable={props.editable}
        surface={props.surface}
      />
    )
  }

  const liveFormat = props.format === 'markdown' || props.format === 'richText'
    ? props.format
    : undefined
  if (props.liveTarget && liveFormat && appKitConfig.dataLive.baseUrl) {
    return <PhotonLiveRecordBodyEditor {...props} format={liveFormat} />
  }
  return <BlockRecordBodyEditor {...props} />
}

/**
 * The body editor with Photon Live.
 *
 * Live is auxiliary and must never stand between the person and their body
 * text: the document is on screen, editable and saving from the first frame,
 * whether a room answers in 50ms, in 20 seconds, or never.
 *
 * The editor is a collaborative BlockNote from the start, bound to a local
 * draft seeded from the body the page loaded. It is never remounted: joining
 * a room re-points its Yjs plugins at the room's document in place, once the
 * content on screen and in the room agree -- see LiveBodySession for when
 * that loses nothing. So typing before the room answers is no longer a reason
 * to give up on sharing this body for the rest of the mount.
 */
function PhotonLiveRecordBodyEditor(props: RecordBodyEditorProps & {
  format: PhotonLiveFormat
}) {
  const { liveTarget, format, value, onCommit } = props

  // One session per mount. Callers key this component by record and body
  // property, which is the scope a room is authorized for.
  const [{ session, initialBinding }] = useState(() => {
    const draftDoc = new Y.Doc()
    Y.applyUpdate(draftDoc, seedUpdate(value, format))
    const draft = draftDoc.getXmlFragment(appKitConfig.dataLive.fragmentName)
    return {
      session: new LiveBodySession({
        draft,
        createProvider: () => createPhotonLiveProvider({
          target: liveTarget!,
          format,
          seedUpdate,
        }),
      }),
      initialBinding: { fragment: draft, user: defaultUser() },
    }
  })
  const [view, setView] = useState<LiveBodyView>(() => session.getView())

  useEffect(() => {
    session.setCommitRest(onCommit)
  }, [onCommit, session])

  useEffect(() => {
    const unsubscribe = session.subscribe(setView)
    session.start()
    return () => {
      unsubscribe()
      // Deferred, so the editor's own unmount can still hand its last edit
      // to the room it is in.
      session.release()
    }
  }, [session])

  return (
    <>
      <PhotonLiveStatus state={liveStatusState(view)} />
      <BlockRecordBodyEditor
        {...props}
        collaboration={initialBinding}
        live={session}
        liveCollab={view.mode === 'live' ? 'on' : view.joined ? 'off' : undefined}
        editable={props.editable ?? true}
        liveTarget={undefined}
      />
    </>
  )
}

/** The body, as a Yjs update in a document of its own. */
function seedUpdate(body: string, seedFormat: PhotonLiveFormat): Uint8Array {
  const seedEditor = BlockNoteEditor.create({ schema: recordBodySchema })
  const seedDoc = blocksToYDoc(
    seedEditor,
    seedBlocks(seedEditor, body, seedFormat),
    appKitConfig.dataLive.fragmentName,
  )
  try {
    return Y.encodeStateAsUpdate(seedDoc)
  } finally {
    // This temporary document is intentionally separate from any room.
    // Applying its update to a room would make every client look seeded
    // before the server has selected the initialization winner.
    seedDoc.destroy()
  }
}

/**
 * What the status line should say for the session as a whole.
 *
 * Only the room carrying the body speaks for it. While a room is being
 * (re)opened the line reports the session itself: unsaved edits waiting for
 * a room, a room being reopened, a conflict left to the person.
 */
function liveStatusState(view: LiveBodyView): PhotonLiveState | null {
  const idle: PhotonLiveState = {
    status: 'connected',
    saveStatus: 'idle',
    error: null,
    initialized: true,
    version: 0,
    recordVersion: '',
    hasUnackedChanges: false,
    canEdit: true,
  }
  switch (view.mode) {
    case 'live': {
      const state = view.provider
      if (!state) return null
      // A retryable failure is the room reconnecting, not the end of it.
      if (state.status === 'failed' && (state.error === null || state.error.retryable)) {
        return { ...state, status: 'disconnected' }
      }
      return state
    }
    case 'joining':
      return view.holding ? { ...idle, saveStatus: 'saving' } : null
    case 'rejoining':
      return { ...idle, status: 'disconnected' }
    case 'conflict':
      return { ...idle, saveStatus: 'conflict' }
    case 'unavailable':
      return { ...idle, status: 'failed' }
  }
}

/**
 * Whether an Html Property's value should open as an artifact.
 *
 * Until this editor learned the type it committed Markdown into Html
 * Properties, so a repository can hold either dialect under the same type.
 * Running "## Heading" through the artifact preview would render the source
 * text instead of a heading, so sniff the value rather than trusting the
 * type. Every value that is actually HTML — including what apps/web writes
 * with `blocksToFullHTML` — opens with a tag. An empty value is HTML-to-be:
 * the Property's type is the only intent an empty body has.
 */
/**
 * The editor is created once per mount, so `imageTarget` is read through a
 * ref: a record that re-renders with a new object identity must not throw
 * away the document under the caret. Whether uploads exist at all is fixed
 * on the first render, because BlockNote reads `uploadFile` when it builds
 * the editor and only offers the upload tab when it is set.
 */
function useBodyEditor(
  imageTarget: RecordBodyImageTarget | undefined,
  collaboration?: InitialCollaboration,
) {
  const imageTargetRef = useRef(imageTarget)
  const [uploads] = useState(() => imageTarget !== undefined)

  useEffect(() => {
    imageTargetRef.current = imageTarget
  }, [imageTarget])

  return useCreateBlockNote({
    schema: recordBodySchema,
    // The local draft has no one else in it, so no awareness until a room.
    collaboration: collaboration
      ? {
        fragment: collaboration.fragment,
        user: collaboration.user,
        showCursorLabels: 'activity',
      }
      : undefined,
    uploadFile: uploads
      ? async (file: File) => {
        const target = imageTargetRef.current
        if (!target) throw new Error(t('editor.noImageTarget'))
        return uploadLibraryImage(target, file)
      }
      : undefined,
  }, [collaboration])
}
type BodyEditor = ReturnType<typeof useBodyEditor>

/** The document a Live body editor is created on: its local draft. */
interface InitialCollaboration {
  fragment: Y.XmlFragment
  user: { name: string; color: string }
}
/** A partial block in the record body schema — what replaceBlocks accepts. */
type BodyPartialBlock = Parameters<BodyEditor['replaceBlocks']>[1][number]

function BlockRecordBodyEditor({
  value,
  format = 'markdown',
  onCommit,
  editable = true,
  surface = 'panel',
  theme,
  imageTarget,
  collaboration,
  live,
  liveCollab,
}: RecordBodyEditorProps & {
  /** Created on this document instead of seeding blocks from `value`. */
  collaboration?: InitialCollaboration
  /** Routes every commit, and rebinds the editor to its rooms. */
  live?: LiveBodySession
  /** Published as `data-live-collab`: whether a room carries the body. */
  liveCollab?: 'on' | 'off'
}) {
  const lastCommitted = useRef(value)
  const loading = useRef(true)
  const seeded = useRef(false)
  const composing = useRef(false)
  const valueBeforeComposition = useRef<string | null>(null)
  const commitTimer = useRef<number | null>(null)
  const pendingValue = useRef<string | null>(null)
  const onCommitRef = useRef(onCommit)
  const editor = useBodyEditor(imageTarget, collaboration)

  useEffect(() => {
    onCommitRef.current = onCommit
  }, [onCommit])

  useEffect(() => {
    if (!live) return
    const port: LiveBodyEditorPort = {
      serializeFragment: (fragment) =>
        serializeBlocks(editor, yXmlFragmentToBlocks(editor, fragment), format),
      comparable: (body) => comparableBody(body, format),
      writeInto: (target, from) => {
        blocksToYXmlFragment(editor, yXmlFragmentToBlocks(editor, from), target)
      },
      mergeInto: (target, from, base) => {
        const blockKey = (block: BodyEditor['document'][number]) =>
          comparableBody(serializeBlocks(editor, [block], format), format)
        const merged = merge3(
          fullBlocks(editor, seedBlocks(editor, base, format)),
          yXmlFragmentToBlocks(editor, from),
          yXmlFragmentToBlocks(editor, target),
          blockKey,
        )
        blocksToYXmlFragment(editor, withUniqueBlockIds(merged), target)
      },
      bind: (provider) => {
        // Rebinding re-renders the document from the room. That is not an
        // edit, and must not come back as a checkpoint of the room's body.
        const wasLoading = loading.current
        loading.current = true
        try {
          bindEditorToRoom(editor, provider)
        } finally {
          loading.current = wasLoading
        }
      },
      composing: () =>
        composing.current || editor.prosemirrorView?.composing === true,
    }
    live.setEditor(port)
  }, [editor, format, live])

  const commitPendingValue = useCallback(() => {
    if (commitTimer.current !== null) {
      window.clearTimeout(commitTimer.current)
      commitTimer.current = null
    }

    const next = pendingValue.current
    pendingValue.current = null
    // A Yjs transaction can advance the room without changing the serialized
    // text. Live still needs this fresh generation after a stale checkpoint.
    if (next === null || (!live && next === lastCommitted.current)) return

    lastCommitted.current = next
    if (live) live.commit(next)
    else void onCommitRef.current(next)
  }, [live])

  // Always the newest one. A body that is waiting out its debounce when the
  // room stops carrying it has to be committed by the persistence mode that
  // is in force when the timer fires, not the one that was in force when it
  // was scheduled -- otherwise the edit is handed to a provider that has just
  // stopped accepting checkpoints and is lost.
  const commitPendingValueRef = useRef(commitPendingValue)
  useEffect(() => {
    commitPendingValueRef.current = commitPendingValue
  }, [commitPendingValue])

  const schedulePendingCommit = useCallback(() => {
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(() => commitPendingValueRef.current(), 500)
  }, [])

  const liveRef = useRef(live)
  useEffect(() => {
    liveRef.current = live
  }, [live])

  const flushNow = useCallback((leaving: boolean) => {
    if (composing.current) {
      // The live composition is not safe to persist, but an ordinary edit may
      // already have been waiting in the debounce when composition started.
      // Keep that confirmed snapshot instead of dropping it with the IME text.
      pendingValue.current = valueBeforeComposition.current
    }
    commitPendingValueRef.current()
    liveRef.current?.flush({ leaving })
  }, [])

  useEffect(() => () => flushNow(true), [flushNow])

  useEffect(() => {
    if (!live) return
    // Closing a tab, reloading or quitting the app never unmounts React, and
    // a hidden page may never run again. Hand the body over while it can go.
    const onPageHide = () => flushNow(true)
    const onVisibilityChange = () => {
      if (document.visibilityState === 'hidden') flushNow(false)
    }
    // Capture phase: at the window, capturing listeners run before the
    // room's own pagehide listener, which destroys it on unload and would
    // drop the edit still waiting in the debounce.
    window.addEventListener('pagehide', onPageHide, { capture: true })
    document.addEventListener('visibilitychange', onVisibilityChange)
    return () => {
      window.removeEventListener('pagehide', onPageHide, { capture: true })
      document.removeEventListener('visibilitychange', onVisibilityChange)
    }
  }, [flushNow, live])

  useEffect(() => {
    // Local first: once seeded, the editor document is the source of truth.
    // Re-seeding on every `value` change would replace the document under the
    // caret each time a save echoes back, dropping the newlines typed while the
    // round trip was in flight. Callers key this component by record id, so a
    // different record mounts a fresh editor. Read-only views still follow the
    // incoming value because nothing can be typed into them.
    if (collaboration) {
      // Collaboration owns the initial content. Even when the HTTP response
      // contains a body, never replace the provider fragment from React props.
      seeded.current = true
      loading.current = false
      lastCommitted.current = value
      return
    }
    if (seeded.current && editable) return
    if (seeded.current && value === lastCommitted.current) return

    loading.current = true
    seeded.current = true
    editor.replaceBlocks(editor.document, seedBlocks(editor, value, format))
    lastCommitted.current = value
    queueMicrotask(() => {
      loading.current = false
    })
  }, [collaboration, editable, editor, format, value])

  useEditorChange((changedEditor) => {
    if (loading.current || !editable) return

    pendingValue.current = serializeDocument(changedEditor, format)
    // An IME can keep composition open while the user considers conversion
    // candidates for longer than the normal save debounce. Committing then
    // re-renders the parent around BlockNote's live composition DOM and can
    // duplicate its unconfirmed text. Keep the newest document locally and
    // wait until compositionend before allowing the save to reach the parent.
    if (composing.current) {
      // ProseMirror drops an inactive Android composition after five seconds,
      // even when the browser never dispatches compositionend. Follow its
      // actual state so the next ordinary edit can resume persistence.
      if (changedEditor.prosemirrorView?.composing !== false) return
      composing.current = false
      valueBeforeComposition.current = null
      live?.compositionEnded()
    }
    schedulePendingCommit()
  }, editor)

  // Live no longer narrates its own connection, so `data-live-collab` is the
  // one place a room's actual attachment is published: absent until a room
  // has carried this body, "off" while none does. The end-to-end suite waits
  // on "on" before asserting on a shared body.
  return (
    <div
      className={surface === 'page'
        ? 'record-body-blocknote record-body-page min-h-[420px] bg-background py-2'
        : 'record-body-blocknote rounded border border-border bg-surface px-2 py-3'}
      data-live-collab={liveCollab}
      onCompositionStartCapture={() => {
        composing.current = true
        valueBeforeComposition.current = pendingValue.current
        if (commitTimer.current !== null) {
          window.clearTimeout(commitTimer.current)
          commitTimer.current = null
        }
      }}
      onCompositionEndCapture={() => {
        composing.current = false
        valueBeforeComposition.current = null
        if (pendingValue.current !== null) schedulePendingCommit()
        live?.compositionEnded()
      }}
    >
      <BlockNoteView
        editor={editor}
        theme={theme}
        editable={editable}
        className="photon-blocknote"
        data-theming-css-variables-demo
        slashMenu={false}
      >
        <SuggestionMenuController
          triggerCharacter="/"
          getItems={async (query) =>
            filterSuggestionItems(
              [
                ...getDefaultReactSlashMenuItems(editor),
                insertHtmlPreviewItem(editor),
              ],
              query,
            )
          }
        />
      </BlockNoteView>
    </div>
  )
}

/**
 * The slash menu entry for the htmlPreview block. Offered in every format:
 * only richText stores it losslessly, but the lossy formats degrade it to an
 * ```html fence rather than dropping it, the same policy as everything else
 * Markdown cannot hold.
 */
function insertHtmlPreviewItem(editor: BodyEditor) {
  return {
    title: t('editor.htmlBlockTitle'),
    subtext: t('editor.htmlBlockSubtext'),
    aliases: ['html', 'iframe', 'artifact', 'preview'],
    group: t('editor.slashMenuOthers'),
    icon: <CodeXml size={18} />,
    onItemClick: () => {
      insertOrUpdateBlockForSlashMenu(editor, { type: 'htmlPreview' })
    },
  }
}

function serializeDocument(
  editor: BodyEditor,
  format: RecordBodyFormat,
): string {
  return serializeBlocks(editor, editor.document, format)
}

function serializeBlocks(
  editor: BodyEditor,
  blocks: BodyEditor['document'],
  format: RecordBodyFormat,
): string {
  if (format === 'richText') return JSON.stringify(blocks)
  if (format === 'html') return editor.blocksToHTMLLossy(blocks)
  return editor.blocksToMarkdownLossy(
    withImageWidthFragments(blocks) as typeof blocks,
  )
}

/**
 * A body as content only, for deciding whether two copies are the same.
 *
 * Seeding a rich text body gives blocks without stored ids fresh random ids,
 * separately in every copy, and BlockNote keeps a trailing empty block with
 * an id of its own. Neither is content. Markdown carries no ids.
 */
function comparableBody(body: string, format: RecordBodyFormat): string {
  if (format !== 'richText') return body.replace(/\s+$/, '')
  let blocks: unknown
  try {
    blocks = JSON.parse(body)
  } catch {
    return body
  }
  if (!Array.isArray(blocks)) return body
  const content = blocks.map(withoutIds)
  while (content.length > 0 && isEmptyParagraph(content[content.length - 1])) content.pop()
  return JSON.stringify(content)
}

function withoutIds(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(withoutIds)
  if (!value || typeof value !== 'object') return value
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([key]) => key !== 'id')
    .map(([key, entry]) => [key, withoutIds(entry)] as const)
  return Object.fromEntries(entries)
}

function isEmptyParagraph(block: unknown): boolean {
  const { type, content, children } = block as { type?: unknown; content?: unknown; children?: unknown }
  return type === 'paragraph' &&
    (!Array.isArray(content) || content.length === 0) &&
    (!Array.isArray(children) || children.length === 0)
}

/** Parsed body blocks with every default filled in, as a document holds them. */
function fullBlocks(editor: BodyEditor, blocks: BodyPartialBlock[]): BodyEditor['document'] {
  const doc = blocksToYDoc(editor, blocks, appKitConfig.dataLive.fragmentName)
  try {
    return yXmlFragmentToBlocks(editor, doc.getXmlFragment(appKitConfig.dataLive.fragmentName))
  } finally {
    doc.destroy()
  }
}

/**
 * A merge that kept both sides' version of a block keeps both of its ids
 * too. The copy that comes second gets new ones, all the way down.
 */
function withUniqueBlockIds(blocks: BodyEditor['document']): BodyEditor['document'] {
  const seen = new Set<string>()
  const visit = (block: BodyEditor['document'][number]): BodyEditor['document'][number] => {
    const children = block.children.map(visit)
    if (!seen.has(block.id)) {
      seen.add(block.id)
      return { ...block, children }
    }
    const id = globalThis.crypto?.randomUUID?.() ?? `${block.id}-${seen.size}`
    seen.add(id)
    return { ...block, id, children }
  }
  return blocks.map(visit)
}

/**
 * Re-point the editor's Yjs plugins at a room, in place.
 *
 * This is what BlockNote's own fork/merge does. The sync plugin re-renders
 * the document from the room on registration and keeps the selection at the
 * same positions, so when the room already holds what is on screen nothing
 * visible changes -- no remount, no lost caret, no broken composition.
 */
function bindEditorToRoom(editor: BodyEditor, provider: PhotonLiveProvider): void {
  editor.unregisterExtension(['ySync', 'yCursor', 'yUndo'])
  editor.registerExtension([
    YSyncExtension({ fragment: provider.fragment }),
    YCursorExtension({
      fragment: provider.fragment,
      user: provider.user,
      provider: { awareness: provider.awareness },
      showCursorLabels: 'activity',
    }),
    YUndoExtension(),
  ])
}

function seedBlocks(
  editor: BodyEditor,
  value: string,
  format: RecordBodyFormat,
): BodyPartialBlock[] {
  if (format === 'richText' && value) {
    const parsed = parseDocument(value)
    if (parsed) return parsed
    // A value that predates the property's conversion — most likely plain
    // Markdown text still sitting in it. Opening it as content beats
    // opening a blank page over someone's body text.
  }
  if (format === 'html' && looksLikeHtml(value)) {
    return editor.tryParseHTMLToBlocks(value)
  }
  return takeImageWidthFragments(editor.tryParseMarkdownToBlocks(value || ''))
}


/**
 * Whether an Html Property's value is really markup. Kept for the block
 * editor's seeding path even though artifact-shaped values no longer reach
 * it, because a read-only view can still be handed either dialect.
 */
function looksLikeHtml(value: string): boolean {
  return /^\s*</.test(value)
}

function parseDocument(raw: string): BodyPartialBlock[] | null {
  try {
    const parsed: unknown = JSON.parse(raw)
    if (Array.isArray(parsed)) return parsed as BodyPartialBlock[]
    const blocks = (parsed as { blocks?: unknown })?.blocks
    if (Array.isArray(blocks)) return blocks as BodyPartialBlock[]
  } catch {
    // fall through
  }
  return null
}
