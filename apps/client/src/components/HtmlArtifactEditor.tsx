import { useCallback, useEffect, useRef, useState } from 'react'
import { HtmlPreviewFrame } from './HtmlPreviewFrame'

/**
 * The body editor for an Html Property whose value really is markup: a
 * sandboxed live preview with the source one tab away, in the spirit of a
 * Claude artifact.
 *
 * The alternative — parsing the markup into BlockNote blocks, which is what
 * this format used to do — destroys exactly what makes an HTML value worth
 * having: `tryParseHTMLToBlocks` drops `<style>`, `<script>` and any
 * structure BlockNote has no block for. Here the source string is the value
 * and nothing rewrites it.
 */
export function HtmlArtifactEditor({
  value,
  onCommit,
  editable = true,
  surface = 'panel',
}: {
  value: string
  onCommit: (value: string) => void
  editable?: boolean
  surface?: 'panel' | 'page'
}) {
  // Local first, same contract as RecordBodyEditor: once mounted, the local
  // draft is the source of truth so a save echoing back does not stomp the
  // caret. Callers key this component by record id.
  const [source, setSource] = useState(value)
  const [tab, setTab] = useState<'preview' | 'code'>(
    value.trim() === '' && editable ? 'code' : 'preview',
  )
  const lastCommitted = useRef(value)
  const commitTimer = useRef<number | null>(null)
  const pendingValue = useRef<string | null>(null)
  const onCommitRef = useRef(onCommit)

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

  const handleChange = (next: string) => {
    setSource(next)
    pendingValue.current = next
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(commitPendingValue, 500)
  }

  const frameHeight = surface === 'page' ? 'h-[560px]' : 'h-[320px]'
  const shown = editable ? source : value

  return (
    <div className="overflow-hidden rounded border border-border bg-surface">
      <div className="flex items-center gap-1 border-b border-border px-2 py-1">
        <TabButton
          active={tab === 'preview'}
          onClick={() => setTab('preview')}
          testId="html-artifact-tab-preview"
        >
          Preview
        </TabButton>
        {editable || shown ? (
          <TabButton
            active={tab === 'code'}
            onClick={() => setTab('code')}
            testId="html-artifact-tab-code"
          >
            Code
          </TabButton>
        ) : null}
        <span className="ml-auto text-xs text-muted-foreground">HTML</span>
      </div>
      {tab === 'preview' ? (
        <div className={`${frameHeight} resize-y overflow-auto`}>
          {shown.trim() === '' ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              Nothing to preview yet — write some HTML in the Code tab.
            </div>
          ) : (
            <HtmlPreviewFrame source={shown} />
          )}
        </div>
      ) : (
        <textarea
          data-testid="html-artifact-code"
          value={shown}
          readOnly={!editable}
          spellCheck={false}
          placeholder="<!doctype html>"
          onChange={(event) => handleChange(event.target.value)}
          className={`${frameHeight} w-full resize-y bg-background p-3 font-mono text-sm leading-relaxed text-foreground outline-none`}
        />
      )}
    </div>
  )
}

function TabButton({
  active,
  onClick,
  testId,
  children,
}: {
  active: boolean
  onClick: () => void
  testId: string
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      data-testid={testId}
      onClick={onClick}
      className={
        active
          ? 'rounded px-2 py-0.5 text-xs font-medium bg-selected text-primary'
          : 'rounded px-2 py-0.5 text-xs text-muted-foreground hover:bg-muted'
      }
    >
      {children}
    </button>
  )
}
