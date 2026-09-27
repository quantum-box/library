import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '@/components/ui/dialog'
import { HtmlViewAndEditor } from './data-detail-ui/html'
import { platformAction } from '@/app/v1beta/_lib/platform-action'
import type { RichTextTemplate } from '@/gen/graphql'
import { useTranslation } from '@/lib/i18n/useTranslation'
import { FileText, Plus, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

export function RichTextTemplateManager({
	org,
	repo,
	accessToken,
	templates,
	onTemplatesChange,
	loading,
	loadError,
}: {
	org: string
	repo: string
	accessToken: string
	templates: RichTextTemplate[]
	onTemplatesChange: (templates: RichTextTemplate[]) => void
	loading: boolean
	loadError: string | null
}) {
	const { t } = useTranslation()
	const copy = t.v1beta.richTextTemplates
	const [selectedId, setSelectedId] = useState<string | null>(null)
	const [name, setName] = useState('')
	const [richText, setRichText] = useState('[]')
	const [editing, setEditing] = useState(false)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [notice, setNotice] = useState<string | null>(null)
	const [deleteOpen, setDeleteOpen] = useState(false)

	const selectedTemplate = useMemo(
		() => templates.find(template => template.id === selectedId),
		[templates, selectedId],
	)

	useEffect(() => {
		setSelectedId(null)
		setName('')
		setRichText('[]')
		setEditing(false)
		setError(null)
		setNotice(null)
	}, [org, repo])

	const startNew = () => {
		setSelectedId(null)
		setName('')
		setRichText('[]')
		setEditing(true)
		setError(null)
		setNotice(null)
	}

	const selectTemplate = (template: RichTextTemplate) => {
		setSelectedId(template.id)
		setName(template.name)
		setRichText(template.richText)
		setEditing(true)
		setError(null)
		setNotice(null)
	}

	const save = async () => {
		setBusy(true)
		setError(null)
		setNotice(null)
		try {
			const input = {
				orgUsername: org,
				repoUsername: repo,
				name: name.trim(),
				richText,
			}
			let saved: RichTextTemplate | undefined
			if (selectedId) {
				const result = await platformAction(
					sdk => sdk.updateRichTextTemplate({ templateId: selectedId, input }),
					{ accessToken },
				)
				saved = result.updateRichTextTemplate
			} else {
				const result = await platformAction(
					sdk => sdk.saveRichTextTemplate({ input }),
					{ accessToken },
				)
				saved = result.saveRichTextTemplate
			}
			if (!saved) throw new Error(copy.saveFailed)
			onTemplatesChange(
				[...templates.filter(template => template.id !== saved.id), saved]
					.sort((left, right) => left.name.localeCompare(right.name)),
			)
			setSelectedId(saved.id)
			setName(saved.name)
			setRichText(saved.richText)
			setNotice(copy.saveSuccess)
		} catch (cause) {
			setError(cause instanceof Error ? cause.message : copy.saveFailed)
		} finally {
			setBusy(false)
		}
	}

	const remove = async () => {
		if (!selectedId) return
		setBusy(true)
		setError(null)
		try {
			const result = await platformAction(
				sdk =>
					sdk.deleteRichTextTemplate({
						orgUsername: org,
						repoUsername: repo,
						templateId: selectedId,
					}),
				{ accessToken },
			)
			if (!result.deleteRichTextTemplate) throw new Error(copy.deleteFailed)
			onTemplatesChange(templates.filter(template => template.id !== selectedId))
			setSelectedId(null)
			setName('')
			setRichText('[]')
			setEditing(false)
			setDeleteOpen(false)
			setNotice(null)
		} catch (cause) {
			setError(cause instanceof Error ? cause.message : copy.deleteFailed)
		} finally {
			setBusy(false)
		}
	}

	return (
		<section className='mx-auto my-6 w-full max-w-6xl rounded-lg border border-border bg-background p-4 shadow-sm sm:p-5 lg:px-6'>
			<div className='flex flex-wrap items-start justify-between gap-3'>
				<div className='flex items-center gap-2'>
					<FileText className='h-4 w-4 text-muted-foreground' aria-hidden='true' />
					<div>
						<h2 className='text-sm font-semibold'>{copy.title}</h2>
						<p className='mt-1 text-xs text-muted-foreground'>{copy.description}</p>
					</div>
				</div>
				<Button type='button' variant='outline' size='sm' onClick={startNew} disabled={busy}>
					<Plus className='mr-1 h-4 w-4' aria-hidden='true' />
					{copy.newTemplate}
				</Button>
			</div>

			{loadError ? (
				<p role='alert' className='mt-4 rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive'>
					{copy.loadFailed}: {loadError}
				</p>
			) : null}
			{loading ? <p className='mt-4 text-sm text-muted-foreground'>{copy.loading}</p> : null}

			{!loading && templates.length > 0 ? (
				<div className='mt-4 flex flex-wrap gap-2'>
					{templates.map(template => (
						<Button
							key={template.id}
							type='button'
							variant={template.id === selectedId ? 'secondary' : 'ghost'}
							size='sm'
							onClick={() => selectTemplate(template)}
							disabled={busy}
						>
							{template.name}
						</Button>
					))}
				</div>
			) : null}
			{!loading && templates.length === 0 && !editing ? (
				<p className='mt-4 text-sm text-muted-foreground'>{copy.empty}</p>
			) : null}

			{editing ? (
				<div className='mt-4 space-y-3 border-t border-border pt-4'>
					<div className='max-w-lg space-y-1.5'>
						<label htmlFor='rich-text-template-name' className='text-sm font-medium'>
							{copy.name}
						</label>
						<Input
							id='rich-text-template-name'
							value={name}
							onChange={event => setName(event.target.value)}
							maxLength={255}
							disabled={busy}
						/>
					</div>
					<fieldset className='space-y-1.5 border-0 p-0'>
						<legend className='text-sm font-medium'>{copy.body}</legend>
						<HtmlViewAndEditor
							key={selectedId ?? `new-${org}-${repo}`}
							isEditing
							content={richText}
							format='richText'
							onChange={setRichText}
							className='min-h-[260px] w-full rounded-md border border-border px-3 py-2'
						/>
					</fieldset>
					{error ? <p role='alert' className='text-sm text-destructive'>{error}</p> : null}
					{notice ? <output className='block text-sm text-green-700'>{notice}</output> : null}
					<div className='flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3'>
						<Button
							type='button'
							variant='ghost'
							size='sm'
							disabled={!selectedTemplate || busy}
							onClick={() => setDeleteOpen(true)}
						>
							<Trash2 className='mr-1 h-4 w-4' aria-hidden='true' />
							{t.v1beta.common.delete}
						</Button>
						<Button type='button' size='sm' disabled={busy || !name.trim()} onClick={() => void save()}>
							{t.v1beta.common.save}
						</Button>
						<Button type='button' variant='outline' size='sm' disabled={busy} onClick={() => setEditing(false)}>
							{t.v1beta.common.cancel}
						</Button>
					</div>
				</div>
			) : null}

			<Dialog open={deleteOpen} onOpenChange={open => !busy && setDeleteOpen(open)}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>{copy.deleteTitle}</DialogTitle>
						<DialogDescription>{copy.deleteDescription}</DialogDescription>
					</DialogHeader>
					{error ? <p role='alert' className='text-sm text-destructive'>{error}</p> : null}
					<DialogFooter>
						<DialogClose asChild>
							<Button type='button' variant='outline' disabled={busy}>
								{t.v1beta.common.cancel}
							</Button>
						</DialogClose>
						<Button type='button' variant='destructive' disabled={busy} onClick={() => void remove()}>
							<Trash2 className='mr-1 h-4 w-4' aria-hidden='true' />
							{t.v1beta.common.delete}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</section>
	)
}
