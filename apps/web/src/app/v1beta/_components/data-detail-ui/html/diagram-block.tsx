import DOMPurify from 'dompurify'
import { useEffect, useId, useState } from 'react'

type DiagramLanguage = 'mermaid' | 'plantuml' | 'puml'

type DiagramBlockProps = {
	language: DiagramLanguage
	source: string
	theme?: 'light' | 'dark'
}

const KROKI_BASE_URL =
	import.meta.env.VITE_KROKI_BASE_URL?.replace(/\/+$/, '') || 'https://kroki.io'

export function normalizeDiagramLanguage(language: string | undefined) {
	const normalized = language?.toLowerCase().trim()
	if (
		normalized === 'mermaid' ||
		normalized === 'plantuml' ||
		normalized === 'puml'
	) {
		return normalized
	}
	return undefined
}

export function DiagramBlock({ language, source, theme }: DiagramBlockProps) {
	return language === 'mermaid' ? (
		<MermaidDiagram source={source} theme={theme} />
	) : (
		<PlantUmlDiagram source={source} />
	)
}

function MermaidDiagram({
	source,
	theme,
}: {
	source: string
	theme?: 'light' | 'dark'
}) {
	const id = useId().replace(/[^a-zA-Z0-9_-]/g, '')
	const [svg, setSvg] = useState<string>()
	const [error, setError] = useState<string>()

	useEffect(() => {
		let cancelled = false
		setSvg(undefined)
		setError(undefined)

		async function renderMermaid() {
			try {
				const mermaid = (await import('mermaid')).default
				mermaid.initialize({
					startOnLoad: false,
					securityLevel: 'strict',
					theme: theme === 'dark' ? 'dark' : 'default',
				})
				const result = await mermaid.render(`library-mermaid-${id}`, source)
				if (!cancelled) {
					setSvg(DOMPurify.sanitize(result.svg))
				}
			} catch (err) {
				if (!cancelled) {
					setError(err instanceof Error ? err.message : 'Failed to render diagram')
				}
			}
		}

		renderMermaid()
		return () => {
			cancelled = true
		}
	}, [id, source, theme])

	if (error) {
		return <DiagramFallback label='Mermaid' source={source} error={error} />
	}

	return (
		<figure className='library-diagram-block' aria-label='Mermaid diagram'>
			{svg ? (
				<div
					className='library-diagram-svg'
					// biome-ignore lint/security/noDangerouslySetInnerHtml: Mermaid returns SVG generated from markdown diagram source.
					dangerouslySetInnerHTML={{ __html: svg }}
				/>
			) : (
				<div className='library-diagram-loading'>Rendering diagram...</div>
			)}
		</figure>
	)
}

function PlantUmlDiagram({ source }: { source: string }) {
	const [imageUrl, setImageUrl] = useState<string>()
	const [error, setError] = useState<string>()

	useEffect(() => {
		const controller = new AbortController()
		let objectUrl: string | undefined
		setImageUrl(undefined)
		setError(undefined)

		async function renderPlantUml() {
			try {
				const response = await fetch(`${KROKI_BASE_URL}/plantuml/svg`, {
					method: 'POST',
					headers: { 'Content-Type': 'text/plain' },
					body: source,
					signal: controller.signal,
				})
				if (!response.ok) {
					throw new Error(`Kroki returned ${response.status}`)
				}
				const blob = await response.blob()
				objectUrl = URL.createObjectURL(blob)
				setImageUrl(objectUrl)
			} catch (err) {
				if (controller.signal.aborted) return
				setError(
					err instanceof Error ? err.message : 'Failed to render PlantUML',
				)
			}
		}

		renderPlantUml()
		return () => {
			controller.abort()
			if (objectUrl) {
				URL.revokeObjectURL(objectUrl)
			}
		}
	}, [source])

	if (error) {
		return <DiagramFallback label='PlantUML' source={source} error={error} />
	}

	return (
		<figure className='library-diagram-block' aria-label='PlantUML diagram'>
			{imageUrl ? (
				<img className='library-diagram-image' src={imageUrl} alt='PlantUML diagram' />
			) : (
				<div className='library-diagram-loading'>Rendering diagram...</div>
			)}
		</figure>
	)
}

function DiagramFallback({
	label,
	source,
	error,
}: {
	label: string
	source: string
	error: string
}) {
	return (
		<div className='library-diagram-error'>
			<div className='library-diagram-error-title'>
				{label} diagram could not be rendered.
			</div>
			<div className='library-diagram-error-message'>{error}</div>
			<pre>
				<code>{source}</code>
			</pre>
		</div>
	)
}
