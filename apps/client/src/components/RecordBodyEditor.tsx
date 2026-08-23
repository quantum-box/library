import { useCallback, useEffect, useRef } from 'react'
import { useCreateBlockNote, useEditorChange } from '@blocknote/react'
import { BlockNoteView } from '@blocknote/shadcn'
import '@blocknote/core/fonts/inter.css'
import '@blocknote/shadcn/style.css'

interface RecordBodyEditorProps {
  value: string
  onCommit: (value: string) => void
  editable?: boolean
  surface?: 'panel' | 'page'
}

export function RecordBodyEditor({
  value,
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
    editor.replaceBlocks(editor.document, editor.tryParseMarkdownToBlocks(value || ''))
    lastCommitted.current = value
    queueMicrotask(() => {
      loading.current = false
    })
  }, [editable, editor, value])

  useEditorChange((changedEditor) => {
    if (loading.current || !editable) return

    pendingValue.current = changedEditor.blocksToMarkdownLossy(changedEditor.document)
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
