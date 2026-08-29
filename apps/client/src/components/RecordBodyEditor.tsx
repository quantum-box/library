import { useCallback, useEffect, useRef, useState } from 'react'
import type { PartialBlock } from '@blocknote/core'
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

export type RecordBodyFormat = 'markdown' | 'richText' | 'html'

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

function useBodyEditor() {
  return useCreateBlockNote({ schema: recordBodySchema })
}
type BodyEditor = ReturnType<typeof useBodyEditor>

function BlockRecordBodyEditor({
  value,
  format = 'markdown',
  onCommit,
  editable = true,
  surface = 'panel',
}: RecordBodyEditorProps) {
  const lastCommitted = useRef(value)
  const loading = useRef(true)
  const seeded = useRef(false)
  const commitTimer = useRef<number | null>(null)
  const pendingValue = useRef<string | null>(null)
  const onCommitRef = useRef(onCommit)
  const editor = useBodyEditor()

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

  useEffect(() => () => {
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
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(commitPendingValue, 500)
  }, editor)

  return (
    <div
      className={surface === 'page'
        ? 'record-body-blocknote record-body-page min-h-[420px] bg-background py-2'
        : 'record-body-blocknote rounded border border-border bg-surface px-2 py-3'}
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
    title: 'HTML',
    subtext: 'HTML document rendered in a sandboxed preview',
    aliases: ['html', 'iframe', 'artifact', 'preview'],
    group: 'Others',
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
  return editor.blocksToMarkdownLossy(editor.document)
}

function seedBlocks(
  editor: BodyEditor,
  value: string,
  format: RecordBodyFormat,
): PartialBlock[] {
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
  return editor.tryParseMarkdownToBlocks(value || '')
}

/**
 * Whether an Html Property's value is really markup. Kept for the block
 * editor's seeding path even though artifact-shaped values no longer reach
 * it, because a read-only view can still be handed either dialect.
 */
function looksLikeHtml(value: string): boolean {
  return /^\s*</.test(value)
}

function parseDocument(raw: string): PartialBlock[] | null {
  try {
    const parsed: unknown = JSON.parse(raw)
    if (Array.isArray(parsed)) return parsed as PartialBlock[]
    const blocks = (parsed as { blocks?: unknown })?.blocks
    if (Array.isArray(blocks)) return blocks as PartialBlock[]
  } catch {
    // fall through
  }
  return null
}
