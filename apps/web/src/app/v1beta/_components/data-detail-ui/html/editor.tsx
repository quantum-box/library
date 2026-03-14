'use client'
import { BlockNoteSchema, createCodeBlockSpec } from '@blocknote/core'
import '@blocknote/core/fonts/inter.css'
import { codeBlockOptions } from '@blocknote/code-block'
import { BlockNoteView } from '@blocknote/ariakit'
import '@blocknote/ariakit/style.css'
import { useCreateBlockNote } from '@blocknote/react'
import { useEffect, useRef } from 'react'
import type { CollaborationState } from './use-collaboration'
import './style.css'

const MARKDOWN_PATTERNS = [
	/^#{1,6}\s/m, // Heading: # Heading
	/^\s*[-*+]\s/m, // Bullet list: - item
	/^\s*\d+\.\s/m, // Numbered list: 1. item
	/```/, // Code block
	/^\s*>/m, // Blockquote: > quote
	/\[.+\]\(.+\)/, // Link: [text](url)
	/^\|.+\|$/m, // Table: | col | col |
	/\*\*.+\*\*/, // Bold: **bold**
]

const RICH_HTML_PATTERN =
	/<(h[1-6]|ul|ol|li|strong|em|pre|code|blockquote|table)\b/i

function looksLikeMarkdown(text: string): boolean {
	return MARKDOWN_PATTERNS.some(p => p.test(text))
}

function hasRichFormatting(html: string): boolean {
	return RICH_HTML_PATTERN.test(html)
}

const schema = BlockNoteSchema.create().extend({
	blockSpecs: {
		codeBlock: createCodeBlockSpec(codeBlockOptions),
	},
})

export type RichTextEditorProps = {
	value?: string
	onChange: (value: string) => void
	className?: string
	isEditable?: boolean
	theme?: 'light' | 'dark'
	format?: 'markdown' | 'html'
	/** When provided, enables real-time collaboration. */
	collaboration?: CollaborationState
}

export function RichTextEditor({
	value,
	onChange,
	className,
	isEditable,
	theme,
	format = 'html',
	collaboration,
}: RichTextEditorProps) {
	const collaborationOption = collaboration
		? {
				provider: collaboration.provider,
				fragment: collaboration.fragment,
				user: {
					name:
						collaboration.provider.awareness.getLocalState()?.user?.name ??
						'Anonymous',
					color:
						collaboration.provider.awareness.getLocalState()?.user?.color ??
						'#6b7280',
				},
			}
		: undefined

	const editor = useCreateBlockNote({
		schema,
		collaboration: collaborationOption,
	})
	const editorRef = useRef(editor)
	editorRef.current = editor

	const containerRef = useRef<HTMLDivElement>(null)

	const lastSyncedValueRef = useRef<string | undefined>(undefined)

	// Intercept paste at DOM level (capture phase) before BlockNote processes it
	useEffect(() => {
		const container = containerRef.current
		if (!container) return

		const handlePaste = (event: ClipboardEvent) => {
			const ed = editorRef.current
			if (!ed) return

			const clipboardData = event.clipboardData
			if (!clipboardData) return

			const text = clipboardData.getData('text/plain')
			if (!text || !looksLikeMarkdown(text)) return

			// If HTML has real rich formatting, let default handle it
			const html = clipboardData.getData('text/html')
			if (html && hasRichFormatting(html)) return

			event.preventDefault()
			event.stopImmediatePropagation()
			;(async () => {
				try {
					if (!editorRef.current) return

					const blocks = await ed.tryParseMarkdownToBlocks(text)

					// Remove selected content before inserting
					const selection = ed.getSelection()
					if (selection) {
						ed.removeBlocks(selection.blocks.map(b => b.id))
					}

					const cursor = ed.getTextCursorPosition()
					ed.insertBlocks(blocks, cursor.block, 'after')
				} catch {
					// Parsing failed — let the text be inserted as-is
					// by doing nothing (default was already prevented,
					// so manually insert as plain text fallback)
					document.execCommand('insertText', false, text)
				}
			})()
		}

		container.addEventListener('paste', handlePaste, { capture: true })
		return () =>
			container.removeEventListener('paste', handlePaste, {
				capture: true,
			})
	}, [])

	const handleOnChange = async () => {
		if (format === 'markdown') {
			const markdown = await editor.blocksToMarkdownLossy()
			lastSyncedValueRef.current = markdown
			onChange(markdown)
			return
		}
		const html = await editor.blocksToFullHTML(editor.document)
		lastSyncedValueRef.current = html
		onChange(html)
	}

	// Load initial content only when NOT in collaboration mode.
	// In collaboration mode, Yjs handles document state.
	useEffect(() => {
		if (collaboration) return

		async function loadInitialContent() {
			const initial = value ?? ''
			if (lastSyncedValueRef.current === initial) {
				return
			}
			const blocks =
				format === 'markdown'
					? await editor.tryParseMarkdownToBlocks(initial)
					: await editor.tryParseHTMLToBlocks(initial)
			editor.replaceBlocks(editor.document, blocks)
			lastSyncedValueRef.current = initial
		}
		loadInitialContent()
	}, [editor, format, value, collaboration])

	return (
		<div ref={containerRef}>
			<BlockNoteView
				className={className}
				editable={isEditable}
				editor={editor}
				theme={theme}
				onChange={handleOnChange}
			/>
		</div>
	)
}
