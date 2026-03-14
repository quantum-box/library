'use client'

import dynamic from 'next/dynamic'
import { useTheme } from 'next-themes'
import type { RichTextEditorProps } from './editor'
import { RichTextViewer } from './viewer'
import { CollaborationPresence } from './collaboration-presence'
import type { CollaborationConfig } from './use-collaboration'
import { useCollaboration } from './use-collaboration'

type RichTextFallbackProps = Pick<
	RichTextEditorProps,
	'value' | 'className' | 'isEditable' | 'format'
>

const RichTextEditorFallback = ({
	value,
	className,
	isEditable,
	format = 'html',
}: RichTextFallbackProps) => {
	if (isEditable) {
		return (
			<div className={className}>
				<p className='text-muted-foreground'>Loading editor…</p>
			</div>
		)
	}
	if (format === 'markdown') {
		return (
			<pre className={`${className ?? ''} whitespace-pre-wrap`}>{value}</pre>
		)
	}
	return <RichTextViewer html={value ?? ''} className={className} />
}

const RichTextEditor = dynamic<RichTextEditorProps>(
	() =>
		import('./editor').then(mod => ({
			default: mod.RichTextEditor,
		})),
	{
		ssr: false,
		loading: props => (
			<RichTextEditorFallback {...(props as RichTextFallbackProps)} />
		),
	},
)

type RichTextFormat = 'markdown' | 'html'

export const HtmlViewAndEditor = ({
	className,
	isEditing,
	content,
	format = 'html',
	onChange,
	collaborationConfig,
}: {
	className?: string
	isEditing: boolean
	content: string
	format?: RichTextFormat
	onChange: (value: string) => void
	collaborationConfig?: CollaborationConfig
}) => {
	const { theme } = useTheme()
	const collab = useCollaboration(isEditing ? collaborationConfig : undefined)

	return (
		<>
			{collab && (
				<div className='flex items-center justify-between px-1 pb-1'>
					<CollaborationStatus connected={collab.connected} />
					<CollaborationPresence collaboration={collab} />
				</div>
			)}
			<RichTextEditor
				value={content}
				onChange={onChange}
				className={className}
				isEditable={isEditing}
				format={format}
				theme={theme === 'dark' ? 'dark' : 'light'}
				collaboration={collab ?? undefined}
			/>
		</>
	)
}

function CollaborationStatus({ connected }: { connected: boolean }) {
	return (
		<div className='flex items-center gap-1.5 px-1 pb-1 text-xs text-muted-foreground'>
			<span
				className={`inline-block h-2 w-2 rounded-full ${
					connected ? 'bg-green-500' : 'bg-yellow-500'
				}`}
			/>
			{connected ? 'Connected' : 'Connecting…'}
		</div>
	)
}
