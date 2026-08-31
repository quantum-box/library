import { useCallback, useEffect, useRef, useState } from 'react'
import { filterSuggestionItems, insertOrUpdateBlockForSlashMenu } from '@blocknote/core'
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
import { uploadLibraryImage } from '../lib/recordsApi'
import {
  takeImageWidthFragments,
  withImageWidthFragments,
} from './blocknote/imageWidthFragments'
import { t } from '../i18n'

export type RecordBodyFormat = 'markdown' | 'richText' | 'html'

/** The repository an image dropped into the body is stored against. */
export interface RecordBodyImageTarget {
  org: string
  repo: string
  operatorId?: string
}

interface RecordBodyEditorProps {
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
  onCommit: (value: string) => void
  editable?: boolean
  surface?: 'panel' | 'page'
  /**
   * Where to store pasted and dropped images. Without it the editor still
   * embeds images by URL, but has nowhere to put a local file, so BlockNote
   * hides the upload tab.
   */
  imageTarget?: RecordBodyImageTarget
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
  return <BlockRecordBodyEditor {...props} />
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
function isArtifactHtml(value: string): boolean {
  return value.trim() === '' || /^\s*</.test(value)
}

/**
 * The editor is created once per mount, so `imageTarget` is read through a
 * ref: a record that re-renders with a new object identity must not throw
 * away the document under the caret. Whether uploads exist at all is fixed
 * on the first render, because BlockNote reads `uploadFile` when it builds
 * the editor and only offers the upload tab when it is set.
 */
function useBodyEditor(imageTarget: RecordBodyImageTarget | undefined) {
  const imageTargetRef = useRef(imageTarget)
  const [uploads] = useState(() => imageTarget !== undefined)

  useEffect(() => {
    imageTargetRef.current = imageTarget
  }, [imageTarget])

  return useCreateBlockNote({
    schema: recordBodySchema,
    uploadFile: uploads
      ? async (file: File) => {
        const target = imageTargetRef.current
        if (!target) throw new Error(t('editor.noImageTarget'))
        return uploadLibraryImage(target, file)
      }
      : undefined,
  })
}
type BodyEditor = ReturnType<typeof useBodyEditor>
/** A partial block in the record body schema — what replaceBlocks accepts. */
type BodyPartialBlock = Parameters<BodyEditor['replaceBlocks']>[1][number]

function BlockRecordBodyEditor({
  value,
  format = 'markdown',
  onCommit,
  editable = true,
  surface = 'panel',
  imageTarget,
}: RecordBodyEditorProps) {
  const lastCommitted = useRef(value)
  const loading = useRef(true)
  const seeded = useRef(false)
  const composing = useRef(false)
  const valueBeforeComposition = useRef<string | null>(null)
  const commitTimer = useRef<number | null>(null)
  const pendingValue = useRef<string | null>(null)
  const onCommitRef = useRef(onCommit)
  const editor = useBodyEditor(imageTarget)

  useEffect(() => {
    onCommitRef.current = onCommit
  }, [onCommit])

  const commitPendingValue = useCallback(() => {
    if (commitTimer.current !== null) {
      window.clearTimeout(commitTimer.current)
      commitTimer.current = null
    }

    const next = pendingValue.current
    pendingValue.current = null
    if (next === null || next === lastCommitted.current) return

    lastCommitted.current = next
    onCommitRef.current(next)
  }, [])

  const schedulePendingCommit = useCallback(() => {
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(commitPendingValue, 500)
  }, [commitPendingValue])

  useEffect(() => () => {
    if (composing.current) {
      // The live composition is not safe to persist, but an ordinary edit may
      // already have been waiting in the debounce when composition started.
      // Keep that confirmed snapshot instead of dropping it with the IME text.
      pendingValue.current = valueBeforeComposition.current
      commitPendingValue()
      return
    }
    commitPendingValue()
  }, [commitPendingValue])

  useEffect(() => {
    // Local first: once seeded, the editor document is the source of truth.
    // Re-seeding on every `value` change would replace the document under the
    // caret each time a save echoes back, dropping the newlines typed while the
    // round trip was in flight. Callers key this component by record id, so a
    // different record mounts a fresh editor. Read-only views still follow the
    // incoming value because nothing can be typed into them.
    if (seeded.current && editable) return
    if (seeded.current && value === lastCommitted.current) return

    loading.current = true
    seeded.current = true
    editor.replaceBlocks(editor.document, seedBlocks(editor, value, format))
    lastCommitted.current = value
    queueMicrotask(() => {
      loading.current = false
    })
  }, [editable, editor, format, value])

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
    }
    schedulePendingCommit()
  }, editor)

  return (
    <div
      className={surface === 'page'
        ? 'record-body-blocknote record-body-page min-h-[420px] bg-background py-2'
        : 'record-body-blocknote rounded border border-border bg-surface px-2 py-3'}
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
      }}
    >
      <BlockNoteView
        editor={editor}
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
  if (format === 'richText') return JSON.stringify(editor.document)
  if (format === 'html') return editor.blocksToHTMLLossy(editor.document)
  return editor.blocksToMarkdownLossy(
    withImageWidthFragments(editor.document) as typeof editor.document,
  )
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
