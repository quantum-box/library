import { fitArtifactToFrame } from '../lib/html/artifactDocument'
import { useI18n } from '../i18n'

/**
 * Sandboxed rendering of an untrusted HTML document, artifact-style.
 *
 * `sandbox="allow-scripts"` without `allow-same-origin` is the load-bearing
 * pair: the document may run its own scripts, but it gets an opaque origin —
 * no cookies, no localStorage, no reach into the app that embeds it. The
 * server-side renderer (packages/blocknote/src/to_html.rs) emits the same
 * sandbox for the read-only HTML view, so the two paths must stay in step.
 *
 * `fitArtifactToFrame` is what keeps a document written for a desktop window
 * from panning a phone sideways; see that module. `lockOverscroll` is for the
 * callers whose frame owns its whole region: it stops the document
 * rubber-banding at its ends, at the price of no longer chaining the scroll
 * out to whatever is behind -- which is why an embedded frame leaves it off.
 */
export function HtmlPreviewFrame({
  source,
  title,
  className,
  lockOverscroll = false,
}: {
  source: string
  title?: string
  className?: string
  lockOverscroll?: boolean
}) {
  const { t } = useI18n()

  return (
    <iframe
      data-testid="html-preview-frame"
      sandbox="allow-scripts"
      srcDoc={fitArtifactToFrame(source, { lockOverscroll })}
      title={title ?? t('editor.htmlPreviewFrameTitle')}
      className={className ?? 'h-full w-full border-0 bg-white'}
    />
  )
}
