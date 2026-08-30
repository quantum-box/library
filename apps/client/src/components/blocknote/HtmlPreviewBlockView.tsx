import { useEffect, useRef, useState } from 'react'
import { HtmlPreviewFrame } from '../HtmlPreviewFrame'
import { useI18n } from '../../i18n'

/**
 * The block UI for the htmlPreview custom block: a sandboxed live preview
 * with an inline source editor behind an Edit toggle.
 *
 * In its own file because the block spec module exports a non-component
 * (the spec factory), which Fast Refresh cannot reload components from.
 */
export function HtmlPreviewBlockView({
  source,
  editable,
  onChange,
}: {
  source: string
  editable: boolean
  onChange: (source: string) => void
}) {
  const { t } = useI18n()
  // A brand-new block has nothing to show, so open straight into the code
  // editor; a filled one opens as the preview it is. While editing, the
  // draft is local-first; the preview always shows the committed props, so
  // Undo is reflected the moment editing ends.
  const [editing, setEditing] = useState(editable && source.trim() === '')
  const [draft, setDraft] = useState(source)
  const commitTimer = useRef<number | null>(null)
  const onChangeRef = useRef(onChange)

  useEffect(() => {
    onChangeRef.current = onChange
  }, [onChange])

  const commit = (value: string) => {
    setDraft(value)
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(() => {
      onChangeRef.current(value)
    }, 500)
  }

  useEffect(() => () => {
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
  }, [])

  return (
    <div
      data-testid="html-preview-block"
      className="my-1 w-full overflow-hidden rounded border border-border bg-surface"
      // The editor treats clicks as selection; keep them inside the block UI.
      contentEditable={false}
    >
      <div className="flex items-center gap-2 border-b border-border px-2 py-1">
        <span className="text-xs font-medium text-muted-foreground">HTML</span>
        {editable ? (
          <button
            type="button"
            data-testid="html-preview-block-toggle"
            className="ml-auto rounded px-2 py-0.5 text-xs text-muted-foreground hover:bg-muted"
            onClick={() => {
              if (editing) {
                onChangeRef.current(draft)
                setEditing(false)
              } else {
                setDraft(source)
                setEditing(true)
              }
            }}
          >
            {editing ? t('common.done') : t('common.edit')}
          </button>
        ) : null}
      </div>
      {editing ? (
        <textarea
          data-testid="html-preview-block-code"
          value={draft}
          spellCheck={false}
          placeholder="<!doctype html>"
          onChange={(event) => commit(event.target.value)}
          onBlur={() => onChangeRef.current(draft)}
          className="h-64 w-full resize-y bg-background p-3 font-mono text-sm leading-relaxed text-foreground outline-none"
        />
      ) : (
        <div className="h-80 resize-y overflow-auto">
          {source.trim() === '' ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              {t('editor.emptyHtmlBlock')}
            </div>
          ) : (
            <HtmlPreviewFrame source={source} />
          )}
        </div>
      )}
    </div>
  )
}
