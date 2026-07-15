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
    if (next === null || next.trim() === lastCommitted.current.trim()) return

    lastCommitted.current = next
    onCommitRef.current(next)
  }, [])

  useEffect(() => () => {
    commitPendingValue()
  }, [commitPendingValue])

  useEffect(() => {
    let cancelled = false
    loading.current = true

    const blocks = editor.tryParseMarkdownToBlocks(value || '')
    if (!cancelled) {
      editor.replaceBlocks(editor.document, blocks)
      lastCommitted.current = value
      queueMicrotask(() => {
        loading.current = false
      })
    }

    return () => {
      cancelled = true
    }
  }, [editor, value])

  useEditorChange((changedEditor) => {
    if (loading.current || !editable) return

    const markdown = changedEditor.blocksToMarkdownLossy(changedEditor.document)
    pendingValue.current = markdown.trim()
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
