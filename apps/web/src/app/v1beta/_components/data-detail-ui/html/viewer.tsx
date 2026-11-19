import '@blocknote/core/fonts/inter.css'
import '@blocknote/core/style.css'
import './style.css'
import DOMPurify from 'dompurify'
import { useMemo } from 'react'

/**
 * Strip everything that can run script from a stored Html property value.
 *
 * The value is written by any repository writer (UI, MCP, import) and can be
 * a full scripted document, so it must never reach the app document as-is:
 * this origin holds the viewer's session tokens. DOMPurify's defaults drop
 * `<script>`, event-handler attributes, `javascript:` URLs and similar, while
 * keeping the formatting a rich-text body needs. The v2 client renders the
 * same values in an opaque-origin sandboxed iframe instead.
 */
export function sanitizeRichTextHtml(html: string): string {
	return DOMPurify.sanitize(html, {
		// Frames and forms could still navigate or submit from this origin.
		FORBID_TAGS: ['iframe', 'frame', 'object', 'embed', 'form'],
	})
}

export function RichTextViewer({
	html,
	className,
}: {
	html: string
	className?: string
}) {
	const safeHtml = useMemo(() => sanitizeRichTextHtml(html), [html])
	return (
		<div
			className={className}
			// biome-ignore lint/security/noDangerouslySetInnerHtml: the value is sanitized by sanitizeRichTextHtml above
			dangerouslySetInnerHTML={{ __html: safeHtml }}
		/>
	)
}
