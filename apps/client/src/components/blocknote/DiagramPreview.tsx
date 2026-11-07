import { useEffect, useId, useState } from 'react'
import DOMPurify from 'dompurify'
import { useI18n } from '../../i18n'

import { diagramLanguage } from './diagramLanguage'

// Mermaid has global configuration and a shared rendering queue. Keep initialization
// and rendering together so concurrent blocks cannot change each other's theme.
let renderQueue = Promise.resolve()

export function DiagramPreview({
  language,
  source,
  onSourceVisibilityChange,
}: {
  language: string
  source: string
  onSourceVisibilityChange?: (visible: boolean) => void
}) {
  const { t } = useI18n()
  const id = useId().replace(/[^a-zA-Z0-9_-]/g, '')
  const kind = diagramLanguage(language)
  const [svg, setSvg] = useState('')
  const [failed, setFailed] = useState(false)
  const [sourceVisible, setSourceVisible] = useState(true)

  useEffect(() => {
    const controller = new AbortController()
    setSvg('')
    setFailed(false)
    if (!kind || !source.trim()) return () => controller.abort()
    // Avoid a request/render on every keystroke.
    const timer = window.setTimeout(() => {
      const render = async () => {
        try {
          let result: string
          if (kind === 'mermaid') {
            const mermaid = (await import('mermaid')).default
            if (controller.signal.aborted) return
            mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', suppressErrorRendering: true, htmlLabels: false })
            result = (await mermaid.render(`library-diagram-${id}`, source)).svg
          } else {
            const base = (import.meta.env.VITE_KROKI_BASE_URL || 'https://kroki.io').replace(/\/+$/, '')
            const response = await fetch(`${base}/plantuml/svg`, {
              method: 'POST', headers: { 'Content-Type': 'text/plain' },
              body: source, signal: controller.signal,
            })
            if (!response.ok) throw new Error(`Kroki: ${response.status}`)
            result = await response.text()
          }
          if (!controller.signal.aborted) setSvg(DOMPurify.sanitize(result, { USE_PROFILES: { svg: true, svgFilters: true } }))
        } catch {
          if (!controller.signal.aborted) setFailed(true)
        }
      }
      if (kind === 'mermaid') renderQueue = renderQueue.then(render, render)
      else void render()
    }, 350)
    return () => { window.clearTimeout(timer); controller.abort() }
  }, [id, kind, source])

  if (!kind) return null
  return (
    <div contentEditable={false} className="library-code-diagram overflow-auto rounded border border-border bg-white p-4 text-black">
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
