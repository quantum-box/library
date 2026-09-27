import {
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import type { ReactNode } from 'react'
import { HtmlViewAndEditor } from './html'
import type { CollaborationConfig } from './html/use-collaboration'
import { useTranslation } from '@/lib/i18n/useTranslation'

export type RichTextTemplateOption = {
	id: string
	name: string
	richText: string
}

type RichTextValue = Extract<
	PropertyDataForEditorFragment['value'],
	{ __typename?: 'HtmlValue' | 'MarkdownValue' | 'RichTextValue' }
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
	richTextTemplates,
}: {
	isEditing: boolean
	property: PropertyForEditorFragment
	propertyData?: PropertyDataForEditorFragment
	onChange: (input: PropertyDataForEditorFragment) => void
	name?: string
	onNameChange?: (value: string) => void
	propertiesContent?: ReactNode
	collaborationConfig?: CollaborationConfig
	richTextTemplates?: RichTextTemplateOption[]
}) {
	const { t } = useTranslation()
	const isMarkdown = property.typ === PropertyType.Markdown
	const isRichText = property.typ === PropertyType.RichText
	const contentValue = (() => {
		const value = propertyData?.value as RichTextValue | undefined
		if (!value) return isRichText ? '[]' : ''
		if (isRichText) {
			const richTextValue = value as { richText?: string; markdown?: string }
			return isEditing ? richTextValue.richText ?? '[]' : richTextValue.markdown ?? ''
		}
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
			value: isRichText
				? ({ __typename: 'RichTextValue', richText: value, markdown: '' } as RichTextValue)
				: isMarkdown
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
			{isEditing && isRichText && richTextTemplates && richTextTemplates.length > 0 ? (
				<div className='px-3 pb-3 sm:px-5'>
					<label
						htmlFor='rich-text-template-select'
						className='mb-1.5 block text-sm font-medium text-foreground'
					>
						{t.v1beta.richTextTemplates.insertIntoDraft}
					</label>
					<select
						id='rich-text-template-select'
						defaultValue=''
						className='h-9 w-full max-w-md rounded-md border border-input bg-background px-3 text-sm text-foreground'
						onChange={event => {
							const selected = richTextTemplates.find(
								template => template.id === event.target.value,
							)
							handleContentChange(selected?.richText ?? '[]')
						}}
					>
						<option value=''>
							{t.v1beta.richTextTemplates.blank}
						</option>
						{richTextTemplates.map(template => (
							<option key={template.id} value={template.id}>
								{template.name}
							</option>
						))}
					</select>
				</div>
			) : null}
			<div className='py-2 sm:py-4'>
				<HtmlViewAndEditor
					key={`${property.id}-${propertyData?.propertyId ?? 'new'}`}
					isEditing={isEditing}
					content={contentValue}
					format={isRichText ? (isEditing ? 'richText' : 'markdown') : isMarkdown ? 'markdown' : 'html'}
					onChange={handleContentChange}
					className='min-h-[320px] w-full rounded-xl bg-transparent py-4 text-base sm:py-6'
					collaborationConfig={collaborationConfig}
				/>
			</div>
		</section>
	)
}
