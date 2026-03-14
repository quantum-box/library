/* eslint-disable @next/next/no-img-element */
/* eslint-disable jsx-a11y/alt-text */
'use client'
import { useEffect, useState } from 'react'
//just use dynamic import
// @ts-ignore
const Editor = dynamic(() => import('draft-js').then(mod => mod.Editor), {
	ssr: false,
})
import { convertFromHTML, convertToHTML } from 'draft-convert'
import {
	type ContentBlock,
	ContentState,
	EditorState,
	KeyBindingUtil,
	RichUtils,
	convertFromRaw,
	convertToRaw,
	getDefaultKeyBinding,
} from 'draft-js'
import 'draft-js/dist/Draft.css'
import dynamic from 'next/dynamic'
import { BlockStyleControls, InlineStyleControls } from './StyleButton'

function MyEditor({
	onChange: handleOnChange,
	defaultValue,
}: {
	onChange: (html: string) => void
	defaultValue?: string
}) {
	const [editorState, setEditorState] = useState(
		defaultValue
			? EditorState.createWithContent(convertFromHTML(defaultValue))
			: EditorState.createEmpty(),
	)

	const { hasCommandModifier } = KeyBindingUtil

	const myKeyBindingFn = (e: React.KeyboardEvent) => {
		if (e.keyCode === 83 && hasCommandModifier(e)) {
			return 'myeditor-save'
		}
		return getDefaultKeyBinding(e)
	}

	useEffect(() => {
		// localStorage.removeItem('test');
		const raw = localStorage.getItem('test')
		if (raw) {
			const contentState = convertFromRaw(JSON.parse(raw))
			const newEditorState = EditorState.createWithContent(contentState)
			setEditorState(newEditorState)
		}
	}, [])

	const saveContent = () => {
		const contentState = editorState.getCurrentContent()
		const raw = convertToRaw(contentState)
		console.log('raw', raw)
		localStorage.setItem('test', JSON.stringify(raw, null, 2))
	}

	const handleKeyCommand = (command: string) => {
		if (command === 'myeditor-save') {
			saveContent()
			return 'handled'
		}
		return 'not-handled'
	}

	// ContentState から HTML に変換
	const _convertToHTML = convertToHTML({
		entityToHTML: (entity, originalText) => {
			switch (entity.type) {
				case 'LINK':
					return <a href={entity.data.url}>{originalText}</a>
				case 'IMAGE':
					return <img src={entity.data.src} alt='' />
				case 'header-one':
					return <h1> {entity.data.src} </h1>
			}
			return originalText
		},
	})

	const onChange = (editorState: EditorState) => {
		const contentState = editorState.getCurrentContent()
		const html = _convertToHTML(contentState)
		setEditorState(editorState)
		handleOnChange(html)
	}

	const saveAsHTML = () => {
		const contentState = editorState.getCurrentContent()
		const html = _convertToHTML(contentState)
		//TODO! There should be a function to send a request to a server
	}

	function myBlockStyleFn(contentBlock: ContentBlock) {
		const type = contentBlock.getType()
		if (type === 'header-one') {
			return 'font-bold text-2xl'
		}
		if (type === 'header-two') {
			return 'font-bold text-xl'
		}
		if (type === 'header-three') {
			return 'font-bold text-lg'
		}
		return ''
	}

	const toggleBlockType = (blockType: string) => {
		setEditorState(RichUtils.toggleBlockType(editorState, blockType))
	}

	const toggleInlineStyle = (inlineStyle: string) => {
		setEditorState(RichUtils.toggleInlineStyle(editorState, inlineStyle))
	}

	return (
		<div className='flex flex-col gap-1'>
			<BlockStyleControls
				editorState={editorState}
				onToggle={toggleBlockType}
			/>
			<InlineStyleControls
				editorState={editorState}
				onToggle={toggleInlineStyle}
			/>
			{/*<div className="flex justify-start gap-1">
        <button
          className="bg-blue-500 text-white py-1 px-3 rounded hover:bg-blue-700 transition"
          onClick={saveContent}
        >
          Save
        </button>
        <button
          className="bg-blue-500 text-white py-1 px-3 rounded hover:bg-blue-700 transition"
          onClick={saveAsHTML}
        >
          Save as HTML
        </button>
      </div>*/}
			<div className='border rounded p-2'>
				<Editor
					editorState={editorState}
					onChange={onChange}
					placeholder='ここから入力を行ってください。'
					handleKeyCommand={handleKeyCommand}
					keyBindingFn={myKeyBindingFn}
					blockStyleFn={myBlockStyleFn}
				/>
			</div>
		</div>
	)
}
export default MyEditor
