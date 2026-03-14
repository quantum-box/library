type StyleButtonProps = {
	onClick?: () => void
	onToggle?: (style: string) => void
	active?: boolean
	className?: string
	style?: string
	label: string
}

const StyleButton = (props: StyleButtonProps) => {
	const handleMouseDown = (e: React.MouseEvent<HTMLSpanElement>) => {
		e.preventDefault()
		props?.onToggle!(props.style || '')
	}

	const handleClick = (e: React.MouseEvent<HTMLSpanElement>) => {
		e.preventDefault()

		if (props?.onClick) {
			props.onClick()
		}
	}

	const className = props.className

	return (
		// biome-ignore lint/a11y/useKeyWithClickEvents: <explanation>
		<span
			className={className}
			onMouseDown={handleMouseDown}
			onClick={handleClick}
		>
			{props.label}
		</span>
	)
}

const BLOCK_TYPES = [
	{ label: 'P', style: 'paragraph' },
	{ label: 'H1', style: 'header-one' },
	{ label: 'H2', style: 'header-two' },
	{ label: 'H3', style: 'header-three' },
	{ label: 'Blockquote', style: 'blockquote' },
	{ label: 'UL', style: 'unordered-list-item' },
	{ label: 'OL', style: 'ordered-list-item' },
	{ label: 'Code Block', style: 'code-block' },
]

// biome-ignore lint/suspicious/noExplicitAny: <explanation>
const BlockStyleControls = (props: any) => {
	const { editorState } = props
	const selection = editorState.getSelection()
	const blockType = editorState
		.getCurrentContent()
		.getBlockForKey(selection.getStartKey())
		.getType()

	return (
		<div className='flex justify-start gap-1'>
			{BLOCK_TYPES.map(type => (
				<StyleButton
					key={type.label}
					active={type.style === blockType}
					label={type.label}
					onToggle={props.onToggle}
					style={type.style}
					className='bg-gray-300 bg-opacity-50 text-black py-1 px-3 rounded hover:bg-opacity-100 transition'
				/>
			))}
		</div>
	)
}

const INLINE_STYLES = [
	{ label: 'Bold', style: 'BOLD' },
	{ label: 'Italic', style: 'ITALIC' },
	{ label: 'Underline', style: 'UNDERLINE' },
	{ label: 'Code', style: 'CODE' },
]

// biome-ignore lint/suspicious/noExplicitAny: <explanation>
const InlineStyleControls = (props: any) => {
	const currentStyle = props.editorState.getCurrentInlineStyle()

	return (
		<div className='flex justify-start gap-1'>
			{INLINE_STYLES.map(type => (
				<StyleButton
					key={type.label}
					active={currentStyle.has(type.style)}
					label={type.label}
					onToggle={props.onToggle}
					style={type.style}
					className='bg-gray-300 bg-opacity-50 text-black py-1 px-3 rounded hover:bg-opacity-100 transition'
				/>
			))}
		</div>
	)
}

export { BlockStyleControls }
export { InlineStyleControls }
