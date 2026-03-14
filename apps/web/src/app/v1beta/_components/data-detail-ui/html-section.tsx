import {
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import type { ReactNode } from 'react'
import { HtmlViewAndEditor } from './html'
import type { CollaborationConfig } from './html/use-collaboration'

type RichTextValue = Extract<
	PropertyDataForEditorFragment['value'],
	{ __typename?: 'HtmlValue' | 'MarkdownValue' }
>

export function HtmlSection({
	isEditing,
	property,
	propertyData,
	onChange,
	name,
	onNameChange,
	propertiesContent,
	collaborationConfig,
}: {
	isEditing: boolean
	property: PropertyForEditorFragment
	propertyData?: PropertyDataForEditorFragment
	onChange: (input: PropertyDataForEditorFragment) => void
	name?: string
	onNameChange?: (value: string) => void
	propertiesContent?: ReactNode
	collaborationConfig?: CollaborationConfig
}) {
	const isMarkdown = property.typ === PropertyType.Markdown
	const contentValue = (() => {
		const value = propertyData?.value as RichTextValue | undefined
		if (!value) return ''
		if (isMarkdown) {
			const markdownValue = value as { markdown?: string; html?: string }
			return markdownValue.markdown ?? markdownValue.html ?? ''
		}
		const htmlValue = value as { html?: string; markdown?: string }
		return htmlValue.html ?? htmlValue.markdown ?? ''
	})()

	const handleContentChange = (value: string) => {
		onChange({
			propertyId: property.id,
			value: isMarkdown
				? ({ __typename: 'MarkdownValue', markdown: value } as RichTextValue)
				: ({ __typename: 'HtmlValue', html: value } as RichTextValue),
		} as PropertyDataForEditorFragment)
	}
	return (
		<section className='relative overflow-hidden'>
			<div className='px-3 py-5 sm:px-5 sm:py-6'>
				{isEditing ? (
					<input
						placeholder='Untitled'
						className='w-full rounded-xl border border-transparent bg-transparent px-1 text-3xl font-semibold leading-tight tracking-tight text-foreground transition-colors focus:border-primary focus:bg-background focus:outline-none focus:ring-0'
						defaultValue={name}
						onChange={e => {
							onNameChange?.(e.target.value)
						}}
					/>
				) : (
					<h1 className='text-3xl font-semibold leading-tight tracking-tight text-foreground'>
						{name || 'Untitled'}
					</h1>
				)}
			</div>
			{propertiesContent ? (
				<div className='pb-6 pt-4'>{propertiesContent}</div>
			) : null}
			<div className='py-2 sm:py-4'>
				<HtmlViewAndEditor
					key={`${property.id}-${propertyData?.propertyId ?? 'new'}`}
					isEditing={isEditing}
					content={contentValue}
					format={isMarkdown ? 'markdown' : 'html'}
					onChange={handleContentChange}
					className='min-h-[320px] w-full rounded-xl bg-transparent py-4 text-base sm:py-6'
					collaborationConfig={collaborationConfig}
				/>
			</div>
		</section>
	)
}
