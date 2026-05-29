import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { DiagramBlock, normalizeDiagramLanguage } from './diagram-block'
import './style.css'

type MarkdownViewerProps = {
	markdown: string
	className?: string
	theme?: 'light' | 'dark'
}

export function MarkdownViewer({
	markdown,
	className,
	theme,
}: MarkdownViewerProps) {
	return (
		<div className={`library-markdown-viewer ${className ?? ''}`}>
			<ReactMarkdown
				remarkPlugins={[remarkGfm]}
				components={{
					pre({ children }) {
						const child = Array.isArray(children) ? children[0] : children
						if (!isCodeElement(child)) {
							return <pre>{children}</pre>
						}

						const language = getCodeLanguage(child.props.className)
						const diagramLanguage = normalizeDiagramLanguage(language)
						if (!diagramLanguage) {
							return <pre>{children}</pre>
						}

						return (
							<DiagramBlock
								language={diagramLanguage}
								source={String(child.props.children ?? '').replace(/\n$/, '')}
								theme={theme}
							/>
						)
					},
				}}
			>
				{markdown}
			</ReactMarkdown>
		</div>
	)
}

function isCodeElement(
	child: unknown,
): child is { type: 'code'; props: { className?: string; children?: unknown } } {
	return (
		typeof child === 'object' &&
		child !== null &&
		'type' in child &&
		child.type === 'code' &&
		'props' in child
	)
}

function getCodeLanguage(className: string | undefined) {
	return className?.match(/language-([^\s]+)/)?.[1]
}
