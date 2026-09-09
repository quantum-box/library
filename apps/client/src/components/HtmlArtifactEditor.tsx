import { useCallback, useEffect, useRef, useState } from 'react'
import { Maximize2, Minimize2 } from 'lucide-react'
import { HtmlPreviewFrame } from './HtmlPreviewFrame'
import { useI18n } from '../i18n'

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
  /**
   * `fill` gives the artifact the whole region its page hands it, with no
   * fixed height and no drag handle: an artifact is a page of its own, and
   * boxing it inside an article column wastes most of the window.
   */
  surface?: 'panel' | 'page' | 'fill'
}) {
  const { t } = useI18n()
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
  // The artifact is a whole page squeezed into an article column. Native
  // fullscreen is what gives it the viewport, and it is the browser's own
  // affordance, so Esc and the platform's exit gesture keep working.
  const frame = useRef<HTMLDivElement | null>(null)
  const [fullscreen, setFullscreen] = useState(false)
  // iPhone has no element fullscreen at all -- `requestFullscreen` is simply
  // absent -- so on the iOS app the button used to be a no-op that still
  // called itself "Full screen". The overlay is the same promise kept in CSS.
  const [overlay, setOverlay] = useState(false)

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

  // Esc and the platform's own exit leave fullscreen without telling this
  // component, so the button's label follows the document, not the click.
  useEffect(() => {
    const sync = () =>
      setFullscreen(document.fullscreenElement === frame.current)
    document.addEventListener('fullscreenchange', sync)
    return () => document.removeEventListener('fullscreenchange', sync)
  }, [])

  // The overlay is its own exit route: it keeps the toolbar on screen, which
  // matters most exactly where it is used, since a phone has no Esc key.
  useEffect(() => {
    if (!overlay) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOverlay(false)
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [overlay])

  const toggleFullscreen = () => {
    if (document.fullscreenElement) {
      void document.exitFullscreen()
      return
    }
    if (overlay) {
      setOverlay(false)
      return
    }

    const request = frame.current?.requestFullscreen
    if (!request) {
      setOverlay(true)
      return
    }
    // A browser can still refuse a request it advertises -- a permissions
    // policy, an embedding that disallows it. Fall back rather than leave the
    // button dead.
    void request.call(frame.current).catch(() => setOverlay(true))
  }

  const handleChange = (next: string) => {
    setSource(next)
    pendingValue.current = next
    if (commitTimer.current !== null) window.clearTimeout(commitTimer.current)
    commitTimer.current = window.setTimeout(commitPendingValue, 500)
  }

  const fill = surface === 'fill'
  const expanded = fullscreen || overlay
  const frameHeight = surface === 'page' ? 'h-[560px]' : 'h-[320px]'
  const shown = editable ? source : value

  return (
    <div
      data-testid="html-artifact-surface"
      // The overlay takes the whole viewport rather than just the preview, so
      // the toolbar -- and with it the way back out -- stays reachable. Fixed
      // positioning escapes the safe-area padding on `#root`, so it re-applies
      // the insets itself, the way `.detail-panel` does.
      className={
        overlay
          ? 'fixed inset-0 z-[100] flex flex-col overflow-hidden bg-surface pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)] pr-[env(safe-area-inset-right)] pt-[env(safe-area-inset-top)]'
          : fill
            ? 'flex h-full min-h-0 flex-col overflow-hidden bg-surface'
            : 'overflow-hidden rounded border border-border bg-surface'
      }
    >
      <div className="flex shrink-0 items-center gap-1 border-b border-border px-2 py-1">
        <TabButton
          active={tab === 'preview'}
          onClick={() => setTab('preview')}
          testId="html-artifact-tab-preview"
        >
          {t('editor.previewTab')}
        </TabButton>
        {editable || shown ? (
          <TabButton
            active={tab === 'code'}
            onClick={() => setTab('code')}
            testId="html-artifact-tab-code"
          >
            {t('editor.codeTab')}
          </TabButton>
        ) : null}
        <span className="ml-auto text-xs text-muted-foreground">HTML</span>
        {/*
          Once the overlay is up it owns the screen, so the way back out has to
          outlive whatever put it there: switching to Code, or clearing the
          document, would otherwise strand a phone -- no button, no Esc key.
        */}
        {expanded || (tab === 'preview' && shown.trim() !== '') ? (
          <button
            type="button"
            data-testid="html-artifact-fullscreen"
            onClick={toggleFullscreen}
            aria-label={
              expanded ? t('editor.exitFullscreen') : t('editor.fullscreen')
            }
            title={
              expanded ? t('editor.exitFullscreen') : t('editor.fullscreen')
            }
            className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            {expanded ? (
              <Minimize2 className="size-3.5" aria-hidden="true" />
            ) : (
              <Maximize2 className="size-3.5" aria-hidden="true" />
            )}
          </button>
        ) : null}
      </div>
      {tab === 'preview' ? (
        <div
          ref={frame}
          // In fullscreen the element is the viewport; when filling, its
          // parent decides the height. Either way the fixed height and the
          // resize handle have to get out of the way. `overscroll-contain`
          // keeps a document still wider than the frame -- one that declared
          // its own viewport, say -- scrolling inside this box rather than
          // handing the drag to the app behind it; `overscroll-none`, where
          // the artifact owns its region, additionally drops the rubber-band
          // the box would show at its own ends.
          className={
            fullscreen
              ? 'h-screen w-screen overflow-auto overscroll-none bg-white'
              : overlay || fill
                ? 'min-h-0 flex-1 overflow-auto overscroll-none'
                : `${frameHeight} resize-y overflow-auto overscroll-contain`
          }
        >
          {shown.trim() === '' ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              {t('editor.nothingToPreview')}
            </div>
          ) : (
            /*
              An artifact that owns its region -- fullscreen, the phone
              overlay, a `fill` surface -- has nothing behind it to scroll, so
              a drag past its top should stop rather than bounce. A boxed one
              sits in a scrolling column, where the reader's scroll has to
              chain out to the page once the document ends.
            */
            <HtmlPreviewFrame source={shown} lockOverscroll={fill || expanded} />
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
          className={`${
            fill ? 'min-h-0 flex-1' : `${frameHeight} resize-y`
          } w-full bg-background p-3 font-mono text-sm leading-relaxed text-foreground outline-none`}
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
