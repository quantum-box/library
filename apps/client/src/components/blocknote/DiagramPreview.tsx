import { useEffect, useRef, useState } from 'react'
import { useI18n } from '../../i18n'

import { diagramLanguage } from './diagramLanguage'
import { cachedDiagram, isAbortError, renderDiagram } from './diagramRenderer'

/** Start rendering a little before a diagram scrolls into view. */
const NEAR_VIEWPORT = '600px 0px'
/** Avoid a request/render on every keystroke. */
const EDIT_DEBOUNCE_MS = 350

export function DiagramPreview({
  language,
  source,
  defaultSourceVisible = true,
  onSourceVisibilityChange,
}: {
  language: string
  source: string
  defaultSourceVisible?: boolean
  onSourceVisibilityChange?: (visible: boolean) => void
}) {
  const { t } = useI18n()
  const kind = diagramLanguage(language)
  const container = useRef<HTMLDivElement>(null)
  // The last diagram this preview rendered, kept on screen while an edit
  // renders its replacement.
  const [shown, setShown] = useState('')
  const [failedSource, setFailedSource] = useState<string | null>(null)
  // A rebuilt block shows what it showed before at once, without a
  // "loading" flash or a layout jump.
  const cached = kind && source.trim() ? cachedDiagram(kind, source) : undefined
  const svg = cached ?? shown
  const failed = cached === undefined && failedSource === source
  const [sourceVisible, setSourceVisible] = useState(defaultSourceVisible)
  const [nearViewport, setNearViewport] = useState(() => typeof IntersectionObserver === 'undefined')
  // The source this preview was mounted with renders without the edit debounce.
  const mountedSource = useRef(source)

  useEffect(() => {
    if (nearViewport) return
    const element = container.current
    if (!element) return
    // Phones cannot afford to render every diagram of a long page at once.
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        setNearViewport(true)
        observer.disconnect()
      }
    }, { rootMargin: NEAR_VIEWPORT })
    observer.observe(element)
    return () => observer.disconnect()
  }, [nearViewport])

  useEffect(() => {
    if (!kind || !source.trim() || !nearViewport) return
    if (cachedDiagram(kind, source) !== undefined) return
    const controller = new AbortController()
    const render = () => {
      renderDiagram(kind, source, controller.signal).then(
        (result) => { if (!controller.signal.aborted) setShown(result) },
        (error: unknown) => {
          if (controller.signal.aborted || isAbortError(error)) return
          setShown('')
          setFailedSource(source)
        },
      )
    }
    if (source === mountedSource.current) {
      render()
      return () => controller.abort()
    }
    // While editing, keep the previous diagram until the new one is ready.
    const timer = window.setTimeout(render, EDIT_DEBOUNCE_MS)
    return () => { window.clearTimeout(timer); controller.abort() }
  }, [kind, nearViewport, source])

  if (!kind) return null
  return (
    <div ref={container} contentEditable={false} className="library-code-diagram overflow-auto rounded border border-border bg-white p-4 text-black">
      {onSourceVisibilityChange && (
        <div className="mb-2 flex justify-end">
          <button
            type="button"
            className="rounded px-2 py-1 text-sm text-gray-600 hover:bg-gray-100 focus-visible:outline focus-visible:outline-2"
            aria-expanded={sourceVisible}
            onClick={() => {
              const visible = !sourceVisible
              setSourceVisible(visible)
              onSourceVisibilityChange(visible)
            }}
          >
            {t(sourceVisible ? 'editor.diagramOnly' : 'editor.diagramShowCode')}
          </button>
        </div>
      )}
      {source.trim() && (
        failed ? <p role="status">{t('editor.diagramFailed')}</p>
          : svg ? <div dangerouslySetInnerHTML={{ __html: svg }} />
            : <p role="status">{t('editor.diagramLoading')}</p>
      )}
    </div>
  )
}
