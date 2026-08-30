import { useI18n } from '../i18n'

/**
 * Sandboxed rendering of an untrusted HTML document, artifact-style.
 *
 * `sandbox="allow-scripts"` without `allow-same-origin` is the load-bearing
 * pair: the document may run its own scripts, but it gets an opaque origin —
 * no cookies, no localStorage, no reach into the app that embeds it. The
 * server-side renderer (packages/blocknote/src/to_html.rs) emits the same
 * sandbox for the read-only HTML view, so the two paths must stay in step.
 */
export function HtmlPreviewFrame({
  source,
  title,
  className,
}: {
  source: string
  title?: string
  className?: string
}) {
  const { t } = useI18n()

  return (
    <iframe
      data-testid="html-preview-frame"
      sandbox="allow-scripts"
      srcDoc={source}
      title={title ?? t('editor.htmlPreviewFrameTitle')}
      className={className ?? 'h-full w-full border-0 bg-white'}
    />
  )
}
