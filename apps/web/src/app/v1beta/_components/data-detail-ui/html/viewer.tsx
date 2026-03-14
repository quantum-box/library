import '@blocknote/core/fonts/inter.css'
import '@blocknote/core/style.css'
import './style.css'

export function RichTextViewer({
	html,
	className,
}: {
	html: string
	className?: string
}) {
	return (
		<>
			<div
				className={className}
				// biome-ignore lint/security/noDangerouslySetInnerHtml: <explanation>
				dangerouslySetInnerHTML={{ __html: html }}
			/>
		</>
	)
}
