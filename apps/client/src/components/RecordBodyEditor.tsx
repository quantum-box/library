import { useCallback, useEffect, useRef } from 'react'
import type { PartialBlock } from '@blocknote/core'
import { useCreateBlockNote, useEditorChange } from '@blocknote/react'
import { BlockNoteView } from '@blocknote/shadcn'
import '@blocknote/core/fonts/inter.css'
import '@blocknote/shadcn/style.css'

export type RecordBodyFormat = 'markdown' | 'richText'

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
   */
  format?: RecordBodyFormat
  onCommit: (value: string) => void
  editable?: boolean
  surface?: 'panel' | 'page'
}

export function RecordBodyEditor({
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
  const editor = useCreateBlockNote()

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

    pendingValue.current = format === 'richText'
      ? JSON.stringify(changedEditor.document)
      : changedEditor.blocksToMarkdownLossy(changedEditor.document)
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
      />
    </div>
  )
}

function seedBlocks(
  editor: ReturnType<typeof useCreateBlockNote>,
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
  return editor.tryParseMarkdownToBlocks(value || '')
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
