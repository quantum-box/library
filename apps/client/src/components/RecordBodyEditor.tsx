import { useEffect, useRef } from 'react'
import { useCreateBlockNote, useEditorChange } from '@blocknote/react'
import { BlockNoteView } from '@blocknote/shadcn'
import '@blocknote/core/fonts/inter.css'
import '@blocknote/shadcn/style.css'

interface RecordBodyEditorProps {
  value: string
  onCommit: (value: string) => void
  editable?: boolean
}

export function RecordBodyEditor({ value, onCommit, editable = true }: RecordBodyEditorProps) {
  const lastCommitted = useRef(value)
  const loading = useRef(true)
  const commitTimer = useRef<number | null>(null)
  const editor = useCreateBlockNote()

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

    if (commitTimer.current) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(() => {
      const markdown = changedEditor.blocksToMarkdownLossy(changedEditor.document)
      const next = markdown.trim()
      if (next !== lastCommitted.current.trim()) {
        lastCommitted.current = next
        onCommit(next)
      }
    }, 500)
  }, editor)

  return (
    <div className="record-body-blocknote rounded border border-border bg-surface px-2 py-3">
      <BlockNoteView
        editor={editor}
        editable={editable}
        className="photon-blocknote"
        data-theming-css-variables-demo
      />
    </div>
  )
}
